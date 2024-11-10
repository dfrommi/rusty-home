#!/bin/bash

set -e

docker buildx build --platform linux/amd64 -t home:5000/rusty-home:latest .
docker push home:5000/rusty-home:latest