# Status code propagation

Demonstrates propagation of http status code from subgraph services to the overall response.
For instance, if a subgraph service fails and returns non 200 status code then it is propagated to the client.

## Usage
```bash
cargo run -- -s ../graphql/supergraph.graphql -c ./router.yaml
```

## Implementation
This implementation makes use of `map_response` and `Context` to pass information from subgraph requests to the overall
`router_response` service.

The request lifecycle looks like this:
```mermaid
sequenceDiagram
    actor Client
    participant RouterService
    participant QueryPlannerService
    participant ExecutionService
    participant SubgraphService(s)

    Client->>RouterService: request
    RouterService->>QueryPlannerService: plan
    QueryPlannerService-->>RouterService: 
    RouterService->>ExecutionService: execute
    par
    ExecutionService-)SubgraphService(s): sub-request
    SubgraphService(s)--)ExecutionService: 
    and 
    ExecutionService-)SubgraphService(s): sub-request
    SubgraphService(s)--)ExecutionService: 
    and 
    ExecutionService-)SubgraphService(s): sub-request
    SubgraphService(s)--)ExecutionService: 
    end
    ExecutionService-->>RouterService: response
    RouterService-->>Client: response
```

On each subgraph response the value `upsert` is called on the context to potentially place the http response code
into a known key.

When the overall response is returned at the router service, the response code is extracted from `Context` and set
on the response.
