#!/usr/bin/env bash
set -euo pipefail

HOST_ROOT="/opt/nangman-crypto/intel-crawl"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
APP_DIR="$REPO_ROOT"
ENV_FILE="$APP_DIR/.env"

HOST_USER="${SUDO_USER:-${USER:-$(id -un)}}"

log() {
  printf '%s\n' "$*"
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

log "[1/3] create host directories"
sudo mkdir -p "$HOST_ROOT/data"
sudo chown "$HOST_USER:$HOST_USER" "$HOST_ROOT" "$HOST_ROOT/data"

log "[2/3] create app env file"
if [[ ! -e "$ENV_FILE" ]]; then
  cp "$APP_DIR/.env.example" "$ENV_FILE"
  log "created $ENV_FILE from .env.example"
else
  log "$ENV_FILE already exists; preserving local values"
fi

log "[3/3] pin checkout path"
set_env_value "INTEL_CRAWL_REPO_ROOT" "$REPO_ROOT"

log "setup complete. start the shared NATS service first, then run scripts/deploy.sh"
