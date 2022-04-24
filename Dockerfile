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
COPY --from=builder /router/target/debug/router /
COPY --from=builder /router/fuzz/supergraph.graphql /
COPY --from=builder /router/fuzz/start_fuzz /
RUN chmod +x /start_fuzz

