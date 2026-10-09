FROM rust:1.95.0-alpine3.23@sha256:606fd313a0f49743ee2a7bd49a0914bab7deedb12791f3a846a34a4711db7ed2 AS builder
RUN apk add --no-cache build-base clang llvm-dev lld pkgconf protobuf protobuf-dev python3 git cmake linux-headers
ENV CARGO_TARGET_DIR=/build/target
WORKDIR /project
