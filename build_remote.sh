#!/bin/bash
set -Eeuo pipefail

source ./.env

cargo sqlx migrate run
cargo sqlx prepare --workspace

commit_hash=$(git rev-parse --short=8 HEAD)
remote_build_dir=/opt/stacks/smart-home/build

docker_build_args=
if (($# > 0)); then
  printf -v docker_build_args ' %q' "$@"
fi

rsync -az --delete \
  -e 'ssh -o RequestTTY=no -o RemoteCommand=none' \
  --exclude .git \
  --exclude .env \
  --exclude target \
  ./ "home:${remote_build_dir}/"

ssh -o RequestTTY=no -o RemoteCommand=none home "cd '${remote_build_dir}' && \
  SQLX_OFFLINE=true cargo build --release --locked && \
  rm -rf .runtime-build && \
  mkdir .runtime-build && \
  cp target/release/app .runtime-build/app && \
  docker build${docker_build_args} \
    -f Dockerfile.runtime \
    -t ghcr.io/dfrommi/rusty-home:latest \
    -t ghcr.io/dfrommi/rusty-home:${commit_hash} \
    .runtime-build"

docker --context home push ghcr.io/dfrommi/rusty-home:latest
docker --context home push ghcr.io/dfrommi/rusty-home:"${commit_hash}"

ssh -o RequestTTY=no -o RemoteCommand=none home "cd /opt/stacks/smart-home && docker compose pull rusty-home && docker compose up -d rusty-home"

cargo sqlx migrate run -D "${LIVE_DATABASE_URL}"

docker --context home logs -f rusty-home
