#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
APP_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
ENV_FILE="${INTEL_CRAWL_ENV_FILE:-$APP_DIR/.env}"

APP_NAME="intel-crawl-app"
ALERT_ENV="${NANGMAN_ALERT_ENV:-dev}"
INCLUDE_SUCCESS="${INTEL_CRAWL_ALERT_INCLUDE_SUCCESS:-false}"
PIPELINE_ALERT_S3_BUCKET="${NANGMAN_PIPELINE_ALERT_S3_BUCKET:-${INTEL_CRAWL_PIPELINE_ALERT_S3_BUCKET:-}}"
PIPELINE_ALERT_S3_PREFIX="${NANGMAN_PIPELINE_ALERT_S3_PREFIX:-pipeline-alert-event/schema=pipeline_alert_event_v1}"

CLUSTER="${INTEL_CRAWL_ECS_CLUSTER:-ecs-nangman-dev-invest-apn2}"
SERVICE="${INTEL_CRAWL_ECS_SERVICE:-svc-nangman-dev-intel-crawl}"
LOG_GROUP="${INTEL_CRAWL_LOG_GROUP:-/ecs/nangman/dev/intel-crawl}"
AWS_REGION="${AWS_REGION:-ap-northeast-2}"
L0_BUCKET="${INTEL_CRAWL_L0_BUCKET:-}"
ERROR_LOOKBACK_MINUTES="${INTEL_CRAWL_ALERT_ERROR_LOG_LOOKBACK_MINUTES:-30}"

if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
fi

log() {
  printf '%s\n' "$*"
}

die() {
  printf 'intel crawl runtime alert failed: %s\n' "$*" >&2
  exit 1
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "missing required command: $1"
  fi
}

is_true() {
  case "$1" in
    1 | true | TRUE | yes | YES) return 0 ;;
    *) return 1 ;;
  esac
}

redact() {
  sed -E 's/[0-9]{12}/<aws-account-id>/g; s/[[:space:]]+$//'
}

send_pipeline_alert() {
  local priority="$1"
  local title="$2"
  local text="$3"
  if [[ -z "$PIPELINE_ALERT_S3_BUCKET" ]]; then
    die "NANGMAN_PIPELINE_ALERT_S3_BUCKET or INTEL_CRAWL_PIPELINE_ALERT_S3_BUCKET is required"
  fi
  local now_ms dt hour event_id key payload_file
  now_ms="$(date -u +%s000)"
  dt="$(date -u +%Y-%m-%d)"
  hour="$(date -u +%H)"
  event_id="pipeline_alert_intel_crawl_${now_ms}_$$"
  key="${PIPELINE_ALERT_S3_PREFIX%/}/dt=${dt}/hour=${hour}/app=${APP_NAME}/priority=${priority}/${event_id}.json"
  payload_file="$(mktemp)"
  local payload
  payload="$(jq -nc \
    --arg event_id "$event_id" \
    --arg dedupe_key "${APP_NAME}:${priority}:${title}" \
    --arg app "$APP_NAME" \
    --arg env "$ALERT_ENV" \
    --arg priority "$priority" \
    --arg title "$title" \
    --arg rendered_text "$text" \
    --argjson created_at_ms "$now_ms" \
    '{schema_version:"pipeline_alert_event_v1",event_id:$event_id,dedupe_key:$dedupe_key,app:$app,environment:$env,priority:$priority,title:$title,conclusion:"Runtime wrapper emitted a pipeline alert.",rendered_text:$rendered_text,current_state:["pre-rendered runtime alert"],reasons:[],next_actions:[],safety:["paper/live/order execution unchanged"],created_at_ms:$created_at_ms}')"
  printf '%s\n' "$payload" > "$payload_file"
  aws s3api put-object \
    --region "$AWS_REGION" \
    --bucket "$PIPELINE_ALERT_S3_BUCKET" \
    --key "$key" \
    --body "$payload_file" \
    --content-type application/json >/dev/null
  rm -f "$payload_file"
}

append_check() {
  local file="$1"
  local title="$2"
  shift 2
  {
    printf '\n## %s\n' "$title"
    "$@" 2>&1 | redact
  } >> "$file"
}

check_runtime() {
  local output_file="$1"
  local failures=0
  local service_json
  service_json="$(aws ecs describe-services \
    --region "$AWS_REGION" \
    --cluster "$CLUSTER" \
    --services "$SERVICE" \
    --output json)"

  jq '{services:.services[] | {serviceName,desiredCount,runningCount,pendingCount,status,taskDefinition,rolloutState:(.deployments[0].rolloutState // "unknown")}}' \
    <<< "$service_json" | redact >> "$output_file"

  local desired running
  desired="$(jq -r '.services[0].desiredCount // 0' <<< "$service_json")"
  running="$(jq -r '.services[0].runningCount // 0' <<< "$service_json")"
  if [[ "$desired" == "0" || "$running" != "$desired" ]]; then
    printf 'service_not_fully_running desired=%s running=%s\n' "$desired" "$running" >> "$output_file"
    failures=$((failures + 1))
  fi

  local start_time
  start_time="$((($(date -u +%s) - (ERROR_LOOKBACK_MINUTES * 60)) * 1000))"
  local error_events
  error_events="$(aws logs filter-log-events \
    --region "$AWS_REGION" \
    --log-group-name "$LOG_GROUP" \
    --start-time "$start_time" \
    --filter-pattern 'panic ?ERROR ?error ?AccessDenied ?OutOfMemory ?SIGKILL ?Killed' \
    --limit 10 \
    --query 'events[].{timestamp:timestamp,message:message}' \
    --output json)"
  local error_count
  error_count="$(jq 'length' <<< "$error_events")"
  printf 'recent_error_log_count=%s\n' "$error_count" >> "$output_file"
  if [[ "$error_count" != "0" ]]; then
    jq '.' <<< "$error_events" | redact >> "$output_file"
    failures=$((failures + 1))
  fi

  if [[ -n "$L0_BUCKET" ]]; then
    append_check "$output_file" "latest raw intel manifest" \
      aws s3api list-objects-v2 \
        --region "$AWS_REGION" \
        --bucket "$L0_BUCKET" \
        --prefix manifests/schema=intel_l0_manifest_v1/ \
        --max-items 1000 \
        --query 'sort_by(Contents || `[]`, &LastModified)[-1].{key:Key,lastModified:LastModified,size:Size}' \
        --output json
  else
    printf 'crawl_l0_bucket_check=skipped reason=INTEL_CRAWL_L0_BUCKET_not_set\n' >> "$output_file"
  fi

  return "$failures"
}

message() {
  local priority="$1"
  local title="$2"
  local output_file="$3"
  local next_action="$4"
  local now_kst
  now_kst="$(TZ=Asia/Seoul date '+%Y-%m-%d %H:%M:%S KST')"
  cat <<EOF
[${priority}][intel-crawl-app] ${title}

결론:
Intel crawl runtime 상태를 확인했습니다.

현재 상태:
- env: ${ALERT_ENV}
- cluster: ${CLUSTER}
- service: ${SERVICE}
- log_group: ${LOG_GROUP}
- app_dir: ${APP_DIR}

주요 원인:
$(tail -n 18 "$output_file" | sed 's/^/- /')

다음 행동:
${next_action}

안전 상태:
- 이 알림은 raw intel 수집 상태 알림입니다.
- paper/live/order execution을 변경하지 않습니다.

발송 시각: ${now_kst}
EOF
}

self_test() {
  require_command jq
  local tmp
  tmp="$(mktemp)"
  cat > "$tmp" <<'EOF'
service_not_fully_running desired=1 running=0
recent_error_log_count=1
crawl_l0_bucket_check=skipped reason=INTEL_CRAWL_L0_BUCKET_not_set
EOF
  local rendered
  rendered="$(message P1 "runtime check failed" "$tmp" "- ECS service와 최근 error log를 먼저 확인")"
  [[ "$rendered" == *"[P1][intel-crawl-app]"* ]] || die "self-test expected P1 title"
  [[ "$rendered" == *"raw intel 수집 상태"* ]] || die "self-test expected crawl context"
  [[ "$rendered" == *"안전 상태:"* ]] || die "self-test expected safety state"
  rm -f "$tmp"
  log "send-runtime-alert self-test passed"
}

main() {
  if is_true "${INTEL_CRAWL_ALERT_SELF_TEST:-false}"; then
    self_test
    return
  fi

  require_command aws
  require_command jq
  require_command sed
  require_command tail

  local output_file
  output_file="$(mktemp)"
  set +e
  check_runtime "$output_file"
  local status=$?
  set -e

  if [[ "$status" -ne 0 ]]; then
    send_pipeline_alert P1 "runtime check failed" "$(message P1 "runtime check failed" "$output_file" $'- ECS desired/running count와 최근 error log를 확인\n- source registry, public source fetch, S3 write, RAW_INTEL publish 경로를 확인')"
    rm -f "$output_file"
    return "$status"
  fi

  if is_true "$INCLUDE_SUCCESS"; then
    send_pipeline_alert P3 "runtime check summary" "$(message P3 "runtime check summary" "$output_file" "- 일반 성공 알림은 기본적으로 끄고, 필요할 때만 일시적으로 켭니다.")"
  fi
  rm -f "$output_file"
}

main "$@"
