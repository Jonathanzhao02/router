//! De-duplicate subgraph requests in flight. Implemented as a tower Layer.
//!
//! See [`Layer`] and [`tower::Service`] for more details.

use crate::{fetch::OperationKind, http_compat, Request, SubgraphRequest, SubgraphResponse};
use futures::{future::BoxFuture, lock::Mutex};
use std::{collections::HashMap, sync::Arc, task::Poll};
use tokio::sync::{
    broadcast::{self, Sender},
    oneshot,
};
use tower::{BoxError, Layer, ServiceExt};

#[derive(Default)]
pub struct QueryDeduplicationLayer;

impl<S> Layer<S> for QueryDeduplicationLayer
where
    S: tower::Service<SubgraphRequest, Response = SubgraphResponse, Error = BoxError> + Clone,
{
    type Service = QueryDeduplicationService<S>;

    fn layer(&self, service: S) -> Self::Service {
        QueryDeduplicationService::new(service)
    }
}

type WaitMap =
    Arc<Mutex<HashMap<http_compat::Request<Request>, Sender<Result<SubgraphResponse, String>>>>>;

pub struct QueryDeduplicationService<S> {
    service: S,
    wait_map: WaitMap,
}

impl<S> QueryDeduplicationService<S>
where
    S: tower::Service<SubgraphRequest, Response = SubgraphResponse, Error = BoxError> + Clone,
{
    fn new(service: S) -> Self {
        QueryDeduplicationService {
            service,
            wait_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn dedup(
        service: S,
        wait_map: WaitMap,
        request: SubgraphRequest,
    ) -> Result<SubgraphResponse, BoxError> {
        loop {
            let mut locked_wait_map = wait_map.lock().await;
            match locked_wait_map.get_mut(&request.subgraph_request) {
                Some(waiter) => {
                    // Register interest in key
                    let mut receiver = waiter.subscribe();
                    drop(locked_wait_map);

                    match receiver.recv().await {
                        Ok(value) => {
                            return value
                                .map(|response| {
                                    SubgraphResponse::new_from_response(
                                        response.response,
                                        request.context,
                                    )
                                })
                                .map_err(|e| e.into())
                        }
                        // there was an issue with the broadcast channel, retry fetching
                        Err(_) => continue,
                    }
                }
                None => {
                    let (tx, _rx) = broadcast::channel(1);

                    locked_wait_map.insert(request.subgraph_request.clone(), tx.clone());
                    drop(locked_wait_map);

                    let context = request.context.clone();
                    let http_request = request.subgraph_request.clone();
                    let res = {
                        // when _drop_signal is dropped, either by getting out of the block, returning
                        // the error from ready_oneshot or by cancellation, the drop_sentinel future will
                        // return with Err(), then we remove the entry from the wait map
                        let (_drop_signal, drop_sentinel) = oneshot::channel::<()>();
                        tokio::task::spawn(async move {
                            let _ = drop_sentinel.await;
                            let mut locked_wait_map = wait_map.lock().await;
                            locked_wait_map.remove(&http_request);
                        });

                        service.ready_oneshot().await?.call(request).await
                    };

                    // Let our waiters know
                    let broadcast_value = res
                        .as_ref()
                        .map(|response| response.clone())
                        .map_err(|e| e.to_string());

                    // We may get errors here, for instance if a task is cancelled,
                    // so just ignore the result of send
                    let _ = tokio::task::spawn_blocking(move || {
                        tx.send(broadcast_value)
                    }).await
                    .expect("can only fail if the task is aborted or if the internal code panics, neither is possible here; qed");

                    return res.map(|response| {
                        SubgraphResponse::new_from_response(response.response, context)
                    });
                }
            }
        }
    }
}

impl<S> tower::Service<SubgraphRequest> for QueryDeduplicationService<S>
where
    S: tower::Service<SubgraphRequest, Response = SubgraphResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    <S as tower::Service<SubgraphRequest>>::Future: Send + 'static,
{
    type Response = SubgraphResponse;
    type Error = BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: SubgraphRequest) -> Self::Future {
        let mut service = self.service.clone();

        if request.operation_kind == OperationKind::Query {
            let wait_map = self.wait_map.clone();

            Box::pin(async move { Self::dedup(service, wait_map, request).await })
        } else {
            Box::pin(async move { service.call(request).await })
        }
    }
}
