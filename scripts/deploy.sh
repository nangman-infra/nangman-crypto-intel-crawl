#!/usr/bin/env bash
set -euo pipefail

cat >&2 <<'MSG'
intel-crawl local compose deploy is disabled.

Current deployment source of truth:
  /Volumes/WD/Developments/nangman-crypto/apps/intel-crawl-app/ecs
  AWS ECS service state
  AWS S3 raw intel artifacts
  NATS RAW_INTEL stream state
  CloudWatch logs and metrics

Use the ECS deployment workflow for runtime changes.
MSG

exit 2
