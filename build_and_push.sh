#!/bin/bash

set -e

cargo sqlx prepare --workspace

docker buildx build --platform linux/amd64 -t ghcr.io/dfrommi/rusty-home:latest .
docker push ghcr.io/dfrommi/rusty-home:latest
