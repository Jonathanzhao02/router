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
RUN apt-get update
RUN apt-get install -y openssl ca-certificates curl gnupg lsb-release
RUN curl -fsSL https://get.docker.com -o get-docker.sh
RUN sh ./get-docker.sh
RUN mkdir fuzz
COPY --from=builder /router/target/debug/router /router
COPY --from=builder /router/fuzz/supergraph.graphql /fuzz/supergraph.graphql
COPY --from=builder /router/fuzz/fuzz.sh /fuzz.sh

ENTRYPOINT ./fuzz.sh

