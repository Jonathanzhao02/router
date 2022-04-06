//! Calls out to nodejs query planner

use crate::prelude::graphql::*;
use async_trait::async_trait;
use futures::future::BoxFuture;
use router_bridge::planner::Planner;
use serde::Deserialize;
use std::fmt::Debug;
use std::sync::Arc;
use tower::BoxError;
use tower::Service;

#[derive(Debug, Clone)]
/// A query planner that calls out to the nodejs router-bridge query planner.
///
/// No caching is performed. To cache, wrap in a [`CachingQueryPlanner`].
pub struct BridgeQueryPlanner {
    planner: Arc<Planner<QueryPlan>>,
}

impl BridgeQueryPlanner {
    pub async fn new(schema: Arc<Schema>) -> Result<Self, QueryPlannerError> {
        Ok(Self {
            planner: Arc::new(Planner::new(schema.as_str().to_string()).await?),
        })
    }
}

impl Service<QueryPlannerRequest> for BridgeQueryPlanner {
    type Response = QueryPlannerResponse;

    type Error = BoxError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: QueryPlannerRequest) -> Self::Future {
        let this = self.clone();
        let fut = async move {
            let body = req.context.request.body();
            match this
                .get(
                    body.query.clone().expect(
                        "presence of a query has been checked by the RouterService before; qed",
                    ),
                    body.operation_name.to_owned(),
                    Default::default(),
                )
                .await
            {
                Ok(query_plan) => Ok(QueryPlannerResponse {
                    query_plan,
                    context: req.context,
                }),
                Err(e) => Err(tower::BoxError::from(e)),
            }
        };

        // Return the response as an immediate future
        Box::pin(fut)
    }
}

#[async_trait]
impl QueryPlanner for BridgeQueryPlanner {
    #[tracing::instrument(skip_all, level = "info", name = "plan")]
    async fn get(
        &self,
        query: String,
        operation: Option<String>,
        _options: QueryPlanOptions,
    ) -> Result<Arc<query_planner::QueryPlan>, QueryPlannerError> {
        let planner_result = self
            .planner
            .plan(query, operation)
            .await
            .map_err(QueryPlannerError::RouterBridgeError)?
            .into_result()
            .map_err(QueryPlannerError::from)?;

        let usage_reporting_signature = planner_result.usage_reporting_signature;

        match planner_result.data {
            QueryPlan { node: Some(node) } => Ok(Arc::new(query_planner::QueryPlan {
                usage_reporting_signature,
                root: node,
            })),
            QueryPlan { node: None } => {
                failfast_debug!("empty query plan");
                Err(QueryPlannerError::EmptyPlan(usage_reporting_signature))
            }
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
/// The root query plan container.
struct QueryPlan {
    /// The hierarchical nodes that make up the query plan
    node: Option<PlanNode>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use test_log::test;

    #[test(tokio::test)]
    async fn test_plan() {
        let planner = BridgeQueryPlanner::new(Arc::new(example_schema()))
            .await
            .unwrap();
        let result = planner
            .get(
                include_str!("testdata/query.graphql").into(),
                None,
                Default::default(),
            )
            .await
            .unwrap();
        insta::assert_debug_snapshot!("plan", result);
        assert_eq!(
            r#"{me{name{first last}}}"#,
            result.usage_reporting_signature.clone().unwrap()
        );
    }

    #[test(tokio::test)]
    async fn test_plan_invalid_query() {
        let planner = BridgeQueryPlanner::new(Arc::new(example_schema()))
            .await
            .unwrap();
        let result = planner
            .get(
                "fragment UnusedTestFragment on User { id } query { me { id } }".to_string(),
                None,
                Default::default(),
            )
            .await
            .unwrap_err();
        insta::assert_debug_snapshot!("plan_invalid_query", result);
    }

    fn example_schema() -> Schema {
        include_str!("testdata/schema.graphql").parse().unwrap()
    }

    #[test]
    fn empty_query_plan() {
        serde_json::from_value::<QueryPlan>(json!({ "plan": { "kind": "QueryPlan"} } )).expect(
            "If this test fails, It probably means QueryPlan::node isn't an Option anymore.\n
                 Introspection queries return an empty QueryPlan, so the node field needs to remain optional.",
        );
    }

    #[test(tokio::test)]
    async fn empty_query_plan_should_be_a_planner_error() {
        insta::assert_debug_snapshot!(
            "empty_query_plan_should_be_a_planner_error",
            BridgeQueryPlanner::new(Arc::new(example_schema()))
                .await
                .unwrap()
                .get(
                    include_str!("testdata/unknown_introspection_query.graphql").into(),
                    None,
                    Default::default(),
                )
                .await
                .unwrap_err()
        )
    }

    #[test(tokio::test)]
    async fn test_plan_error() {
        let planner = BridgeQueryPlanner::new(Arc::new(example_schema()))
            .await
            .unwrap();
        let result = planner.get("".into(), None, Default::default()).await;

        assert_eq!(
            "query planning had errors: bridge errors: UNKNOWN: Syntax Error: Unexpected <EOF>.",
            result.unwrap_err().to_string()
        );
    }
}