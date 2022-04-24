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

# Package Stage
FROM --platform=linux/amd64 ubuntu:20.04

## TODO: Change <Path in Builder Stage>
RUN apt-get update
RUN apt-get install -y openssl ca-certificates curl gnupg lsb-release
RUN curl -fsSL https://github.com/apollographql/router/releases/download/v0.1.0-preview.6/router-0.1.0-preview.6-x86_64-linux.tar.gz -O apollo.tar.gz
RUN tar -xf apollo.tar.gz
RUN mkdir fuzz
COPY --from=builder /router/fuzz/supergraph.graphql /fuzz/supergraph.graphql
CMD ["/dist/router", "--supergraph", "/fuzz/supergraph.graphql"]
EXPOSE 4000

