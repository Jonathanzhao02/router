#!/bin/sh
docker run -p -d -v /var/run/docker.sock:/var/run/docker.sock --net=host --mount "type=bind,source=/fuzz/supergraph.graphql,target=/supergraph.graphql" --rm ghcr.io/apollographql/router:v0.1.0-preview.6 -s supergraph.graphql
exec "$@"
