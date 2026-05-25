#!/usr/bin/env bash
set -euo pipefail

cat >&2 <<'MSG'
intel-crawl local host setup is disabled.

The current runtime contract is AWS ECS + IAM + AWS S3 + runtime NATS. Do not
create local .env files or compose host state from this app repository.
MSG

exit 2
