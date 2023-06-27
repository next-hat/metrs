#!/bin/sh
## name: build_images.sh

PROJECT=metrsd
VERSION=$(cat ./bin/metrsd/Cargo.toml | grep -m 1 "version = \"" | sed 's/[^0-9.]*\([0-9.]*\).*/\1/')
BUILDER=buildx-multi-arch

## Prepare buildx
docker buildx inspect $BUILDER || docker buildx create --name=$BUILDER --driver=docker-container --driver-opt=network=host

## Build and push image
docker buildx build --builder=$BUILDER --platform=linux/amd64,linux/arm64 --tag="ghcr.io/nxthat/$PROJECT:$VERSION" -f Dockerfile . --push
