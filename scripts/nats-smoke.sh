#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

: "${NATS_SMOKE_URL:?set NATS_SMOKE_URL, for example nats://127.0.0.1:4222}"

export NATS_SMOKE_STREAM="${NATS_SMOKE_STREAM:-RAW_INTEL_SMOKE}"
export NATS_SMOKE_SUBJECT="${NATS_SMOKE_SUBJECT:-raw_intel_event.created.smoke}"

cd "${repo_root}"
cargo test publisher::tests::publishes_with_jetstream_ack_and_stable_message_id -- --ignored --exact
