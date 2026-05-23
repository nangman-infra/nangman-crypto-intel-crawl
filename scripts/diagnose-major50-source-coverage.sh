#!/usr/bin/env bash
set -euo pipefail

SOURCE_REGISTRY="${INTEL_CRAWL_SOURCE_REGISTRY:-${1:-}}"
OUTPUT_FILE="${INTEL_CRAWL_SOURCE_COVERAGE_OUTPUT:-${2:-}}"
CANDIDATE_GAP_FILE="${INTEL_CRAWL_CANDIDATE_GAP_FILE:-${3:-}}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

require_absolute_file() {
  local name="$1"
  local path="$2"
  if [[ -z "$path" || "$path" != /* ]]; then
    echo "$name must be an absolute file path" >&2
    exit 1
  fi
  if [[ ! -f "$path" ]]; then
    echo "$name does not exist: $path" >&2
    exit 1
  fi
}

require_absolute_output_path() {
  local name="$1"
  local path="$2"
  if [[ -z "$path" ]]; then
    return
  fi
  case "$path" in
    /*) ;;
    *)
      echo "$name must be an absolute path; got $path" >&2
      exit 1
      ;;
  esac
}

require_optional_absolute_file() {
  local name="$1"
  local path="$2"
  if [[ -z "$path" ]]; then
    return
  fi
  require_absolute_file "$name" "$path"
}

require_command date
require_command jq
require_command mktemp
require_absolute_file "INTEL_CRAWL_SOURCE_REGISTRY or first argument" "$SOURCE_REGISTRY"
require_absolute_output_path "INTEL_CRAWL_SOURCE_COVERAGE_OUTPUT or second argument" "$OUTPUT_FILE"
require_optional_absolute_file "INTEL_CRAWL_CANDIDATE_GAP_FILE or third argument" "$CANDIDATE_GAP_FILE"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

candidate_gap_input="$tmp_dir/candidate-gap.json"
if [[ -n "$CANDIDATE_GAP_FILE" ]]; then
  cp "$CANDIDATE_GAP_FILE" "$candidate_gap_input"
else
  printf '{}\n' > "$candidate_gap_input"
fi

tmp_output="$tmp_dir/output.json"

jq -n \
  --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg source_registry "$SOURCE_REGISTRY" \
  --arg candidate_gap_file "$CANDIDATE_GAP_FILE" \
  --slurpfile registry "$SOURCE_REGISTRY" \
  --slurpfile candidate_gap "$candidate_gap_input" \
  '
    def canonical_symbol:
      (tostring | ascii_upcase | gsub("[^A-Z0-9]"; "")) as $symbol
      | if (($symbol | length) > 4 and ($symbol | endswith("USDT"))) then $symbol[0:-4]
        elif (($symbol | length) > 6 and ($symbol | endswith("USDC"))) then $symbol[0:-4]
        else $symbol
        end;

    def applies_to_all_major50:
      (.applies_to_assets? == "all_major_50");

    def applies_directly_to($asset):
      (.applies_to_assets? | type) == "array"
      and (.applies_to_assets | map(canonical_symbol) | index($asset));

    def status_for($enabled_direct_count; $available_disabled_direct_count; $enabled_global_count):
      if $enabled_direct_count > 0 then "asset_specific_enabled"
      elif $available_disabled_direct_count > 0 then "asset_specific_available_disabled"
      elif $enabled_global_count > 0 then "global_symbol_match_only"
      else "missing_enabled_source"
      end;

    def histogram($values; $key_name):
      reduce $values[] as $value ({};
        .[$value] = (.[$value] // 0) + 1
      )
      | to_entries
      | sort_by([-.value, .key])
      | map({($key_name): .key, count: .value});

    def candidate_gap_symbols($gap):
      if ($gap.schema_version? == "intel_candidate_source_gap_diagnosis_v1") then
        [$gap.symbols[]?.symbol]
      elif (($gap.gaps? | type) == "object" and (($gap.gaps.approved_symbols_without_candidate? | type) == "array")) then
        $gap.gaps.approved_symbols_without_candidate
      elif (($gap.approved_symbols_without_candidate? | type) == "array") then
        $gap.approved_symbols_without_candidate
      else
        []
      end
      | map(canonical_symbol)
      | map(select(length > 0))
      | unique
      | sort;

    ($registry[0]) as $reg
    | ($candidate_gap[0]) as $gap
    | ($reg.universe_assets // [] | map(. + {__asset:(.asset | canonical_symbol)})) as $assets
    | ($reg.sources // []) as $sources
    | ($assets | map(.__asset) | unique | sort) as $universe_symbols
    | (candidate_gap_symbols($gap)) as $candidate_focus_symbols
    | [
        $assets[] as $asset
        | $asset.__asset as $symbol
        | ([
            $sources[]
            | select(.enabled == true)
            | select(applies_directly_to($symbol))
            | .source_id
          ] | unique | sort) as $enabled_direct_source_ids
        | ([
            $sources[]
            | select(.enabled != true)
            | select(.source_state? == "available_disabled")
            | select(applies_directly_to($symbol))
            | .source_id
          ] | unique | sort) as $available_disabled_direct_source_ids
        | ([
            $sources[]
            | select(.enabled == true)
            | select(applies_to_all_major50)
            | .source_id
          ] | unique | sort) as $enabled_global_source_ids
        | ([
            $sources[]
            | select(.enabled != true)
            | select(.source_state? == "available_disabled")
            | select(applies_to_all_major50)
            | .source_id
          ] | unique | sort) as $available_disabled_global_source_ids
        | {
            asset:$symbol,
            reference_symbol_native:($asset.reference_symbol_native // null),
            rss_seed_status:($asset.rss_seed_status // null),
            coverage_status:status_for(
              ($enabled_direct_source_ids | length);
              ($available_disabled_direct_source_ids | length);
              ($enabled_global_source_ids | length)
            ),
            enabled_direct_source_count:($enabled_direct_source_ids | length),
            available_disabled_direct_source_count:($available_disabled_direct_source_ids | length),
            enabled_global_source_count:($enabled_global_source_ids | length),
            available_disabled_global_source_count:($available_disabled_global_source_ids | length),
            enabled_direct_source_ids:$enabled_direct_source_ids,
            available_disabled_direct_source_ids:$available_disabled_direct_source_ids,
            quality_gaps:([
              if ($enabled_direct_source_ids | length) == 0
                then "missing_enabled_asset_specific_source"
                else empty
              end,
              if (($available_disabled_direct_source_ids | length) == 0)
                then "missing_available_disabled_asset_source_inventory"
                else empty
              end,
              if ($enabled_global_source_ids | length) == 0
                then "missing_global_news_source"
                else empty
              end
            ])
          }
      ] as $records
    | (
        if ($candidate_focus_symbols | length) > 0
        then [$records[] | select(.asset as $asset | $candidate_focus_symbols | index($asset))]
        else []
        end
      ) as $focus_records
    | ($candidate_focus_symbols - $universe_symbols) as $unexpected_focus_symbols
    | (
        $reg.coverage.asset_specific_coverage_count? //
        $reg.coverage.asset_specific_enabled_count? //
        null
      ) as $declared_asset_specific_count
    | ($records | map(select(.enabled_direct_source_count > 0)) | length) as $actual_asset_specific_count
    | {
        schema_version:"intel_crawl_major50_source_coverage_diagnosis_v1",
        generated_at:$generated_at,
        input:{
          source_registry_file:$source_registry,
          source_registry_schema:($reg.schema_version // null),
          candidate_gap_file:(if $candidate_gap_file == "" then null else $candidate_gap_file end),
          candidate_gap_schema:($gap.schema_version // null)
        },
        safety:{
          network_fetch:false,
          s3_read:false,
          s3_write:false,
          nats_publish:false,
          ecs_task_started:false,
          registry_modified:false,
          local_registry_only:true,
          shadow_paper_live_enabled:false
        },
        registry_consistency:{
          declared_universe_asset_count:($reg.coverage.universe_asset_count // null),
          actual_universe_asset_count:($records | length),
          declared_asset_specific_coverage_count:$declared_asset_specific_count,
          actual_asset_specific_enabled_count:$actual_asset_specific_count,
          declared_asset_specific_count_matches_actual:(
            if $declared_asset_specific_count == null
            then null
            else $declared_asset_specific_count == $actual_asset_specific_count
            end
          )
        },
        summary:{
          universe_asset_count:($records | length),
          status_counts:histogram(($records | map(.coverage_status)); "coverage_status"),
          global_only_symbols:([$records[] | select(.coverage_status == "global_symbol_match_only") | .asset]),
          missing_enabled_source_symbols:([$records[] | select(.coverage_status == "missing_enabled_source") | .asset]),
          candidate_focus_symbol_count:($candidate_focus_symbols | length),
          candidate_focus_status_counts:histogram(($focus_records | map(.coverage_status)); "coverage_status"),
          candidate_focus_missing_direct_symbols:([
            $focus_records[]
            | select(.enabled_direct_source_count == 0)
            | .asset
          ]),
          unexpected_candidate_focus_symbols:$unexpected_focus_symbols
        },
        assets:$records,
        recommended_actions:(
          [
            if (($records | map(select(.enabled_direct_source_count == 0)) | length) > 0)
              then "add_or_enable_asset_specific_project_governance_developer_sources_for_global_only_major50_symbols"
              else empty
            end,
            if (($records | map(select(.available_disabled_direct_source_count > 0 and .enabled_direct_source_count == 0)) | length) > 0)
              then "inspect_activation_blockers_for_available_disabled_direct_sources"
              else empty
            end,
            if (($candidate_focus_symbols | length) > 0 and ($focus_records | map(select(.enabled_direct_source_count == 0)) | length) > 0)
              then "prioritize_candidate_gap_symbols_when_expanding_source_registry"
              else empty
            end,
            if (($candidate_focus_symbols | length) == 0)
              then "optionally_pass_candidate_gap_diagnosis_to_focus_missing_candidate_symbols"
              else empty
            end,
            "treat_global_news_only_as_partial_alpha_input_coverage",
            "do_not_open_shadow_paper_live_from_source_registry_coverage"
          ]
          | unique
        )
      }
  ' > "$tmp_output"

if [[ -n "$OUTPUT_FILE" ]]; then
  cp "$tmp_output" "$OUTPUT_FILE"
  {
    echo "source_coverage_output=$OUTPUT_FILE"
    jq -r '
      "universe_asset_count=\(.summary.universe_asset_count)",
      "status_counts=\(.summary.status_counts | map("\(.coverage_status):\(.count)") | join(","))",
      "candidate_focus_missing_direct_symbols=\(.summary.candidate_focus_missing_direct_symbols | join(","))"
    ' "$tmp_output"
  } >&2
else
  cat "$tmp_output"
fi

echo "major-50 source coverage diagnosis completed" >&2
