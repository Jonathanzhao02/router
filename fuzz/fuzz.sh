#!/bin/bash
docker run -p -d --net=host --mount "type=bind,source=/supergraph.graphql,target=/supergraph.graphql" --r, ghcr.io/apollographql/router:v0.1.0-preview.6 -s supergraph.graphql

/router < /dev/stdin

