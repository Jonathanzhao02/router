# Build Stage
FROM --platform=linux/amd64 rustlang/rust:nightly as builder

## Install build dependencies.
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y cmake clang

## Add source code to the build stage.
ADD . /router
WORKDIR /router

## Build instructions
WORKDIR fuzz
RUN cargo +nightly rustc --bin router -- \
    -C passes='sancov-module' \
    -C llvm-args='-sanitizer-coverage-level=3' \
    -C llvm-args='-sanitizer-coverage-inline-8bit-counters' \
    -Z sanitizer=address

# Package Stage
FROM --platform=linux/amd64 ubuntu:20.04

## TODO: Change <Path in Builder Stage>
RUN apt-get install libssl
COPY --from=builder /router/target/debug/router /
COPY --from=builder /router/fuzz/supergraph.graphql /

CMD docker run -p -d --net=host --mount "type=bind,source=/supergraph.graphql,target=/supergraph.graphql" --r, ghcr.io/apollographql/router:v0.1.0-preview.6 -s supergraph.graphql

