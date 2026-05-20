#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
APP_DIR="$REPO_ROOT"
ENV_FILE="$APP_DIR/.env"
ENV_EXAMPLE="$APP_DIR/.env.example"
COMPOSE="$APP_DIR/compose.yml"

log() {
  printf '%s\n' "$*"
}

require_file() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    printf 'missing required file: %s\n' "$file" >&2
    printf 'run scripts/setup-host.sh first\n' >&2
    exit 1
  fi
}

set_env_value() {
  local key="$1"
  local value="$2"
  if grep -q "^$key=" "$ENV_FILE"; then
    sed -i "s|^$key=.*|$key=$value|" "$ENV_FILE"
  else
    printf '%s=%s\n' "$key" "$value" >> "$ENV_FILE"
  fi
}

ensure_env_file() {
  if [[ ! -f "$ENV_FILE" ]]; then
    require_file "$ENV_EXAMPLE"
    cp "$ENV_EXAMPLE" "$ENV_FILE"
    log "created $ENV_FILE from .env.example"
  fi
  set_env_value "INTEL_CRAWL_REPO_ROOT" "$REPO_ROOT"
}

load_env_file() {
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a

  NANGMAN_NATS_DOCKER_NETWORK="${NANGMAN_NATS_DOCKER_NETWORK:-nangman-crypto-bus}"
}

require_env_value() {
  local key="$1"
  local value="${!key:-}"
  if [[ -z "$value" ]]; then
    printf 'missing required env value: %s\n' "$key" >&2
    exit 1
  fi
}

check_nats_network() {
  if ! sudo docker network inspect "$NANGMAN_NATS_DOCKER_NETWORK" >/dev/null 2>&1; then
    cat >&2 <<MSG
missing Docker network: $NANGMAN_NATS_DOCKER_NETWORK

Start the shared NATS server first:
  cd /opt/nangman-crypto/nats-server
  scripts/deploy.sh
MSG
    exit 1
  fi
}

log "[1/5] config check"
ensure_env_file
require_file "$ENV_FILE"
load_env_file
require_env_value "INTEL_CRAWL_L0_OBJECT_STORE_ENDPOINT"
require_env_value "INTEL_CRAWL_L0_OBJECT_STORE_BUCKET"
require_env_value "INTEL_CRAWL_L0_OBJECT_STORE_REGION"
require_env_value "INTEL_CRAWL_L0_OBJECT_STORE_ACCESS_KEY_ID"
require_env_value "INTEL_CRAWL_L0_OBJECT_STORE_SECRET_ACCESS_KEY"
check_nats_network

log "[2/5] compose config"
sudo docker compose -f "$COMPOSE" --env-file "$ENV_FILE" config >/dev/null

log "[3/5] build"
sudo docker compose -f "$COMPOSE" --env-file "$ENV_FILE" build

log "[4/5] recreate compose services"
sudo docker compose -f "$COMPOSE" --env-file "$ENV_FILE" up -d --force-recreate

log "[5/5] service status"
sudo docker compose -f "$COMPOSE" --env-file "$ENV_FILE" ps

cat <<EOF
Follow structured crawl summaries with:

sudo docker compose -f $COMPOSE --env-file $ENV_FILE logs -f rss-worker
EOF
