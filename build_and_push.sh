#!/bin/bash
set -Eeuo pipefail

cargo sqlx migrate run
cargo sqlx prepare --workspace

cleanup() {
  colima stop -p x86 || true
}
trap cleanup EXIT INT TERM

colima start -p x86 || true

docker --context colima-x86 build --platform linux/amd64 -t ghcr.io/dfrommi/rusty-home:latest $@ .
docker --context colima-x86 push ghcr.io/dfrommi/rusty-home:latest

ssh home -o RemoteCommand=none "cd /opt/stacks/smart-home && docker compose pull rusty-home && docker compose up -d rusty-home"

docker --context home logs -f rusty-home

