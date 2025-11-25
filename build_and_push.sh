#!/bin/bash
set -Eeuo pipefail

source ./.env

cargo sqlx migrate run
cargo sqlx prepare --workspace

cleanup() {
  colima stop -p x86 || true
}
trap cleanup EXIT INT TERM

colima start -p x86 || true

commit_hash=$(git rev-parse --short=8 HEAD)

docker --context colima-x86 build --platform linux/amd64 \
  -t ghcr.io/dfrommi/rusty-home:latest \
  -t ghcr.io/dfrommi/rusty-home:"${commit_hash}" \
  "$@" .
docker --context colima-x86 push ghcr.io/dfrommi/rusty-home:latest
docker --context colima-x86 push ghcr.io/dfrommi/rusty-home:"${commit_hash}"

ssh home -o RemoteCommand=none "cd /opt/stacks/smart-home && docker compose pull rusty-home && docker compose up -d rusty-home"

cargo sqlx migrate run -D "${LIVE_DATABASE_URL}"

docker --context home logs -f rusty-home
