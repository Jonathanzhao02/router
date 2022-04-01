use crate::plugins::telemetry::config::Trace;
use crate::plugins::telemetry::tracing::apollo_telemetry;
use crate::plugins::telemetry::tracing::apollo_telemetry::{SpaceportConfig, StudioGraph};
use crate::plugins::telemetry::TracingConfigurator;
use opentelemetry::sdk::trace::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tower::BoxError;
use url::Url;

#[derive(Default, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub endpoint: Option<Url>,
    pub apollo_key: Option<String>,
    pub apollo_graph_ref: Option<String>,
}

impl TracingConfigurator for Config {
    fn apply(&self, builder: Builder, trace_config: &Trace) -> Result<Builder, BoxError> {
        Ok(match self {
            Config {
                endpoint: Some(endpoint),
                apollo_key: Some(key),
                apollo_graph_ref: Some(reference),
            } => {
                let exporter = apollo_telemetry::new_pipeline()
                    .with_trace_config(trace_config.into())
                    .with_graph_config(&Some(StudioGraph {
                        reference: reference.clone(),
                        key: key.clone(),
                    }))
                    .with_spaceport_config(&Some(SpaceportConfig {
                        collector: endpoint.to_string(),
                    }))
                    .build_exporter()?;
                builder.with_batch_exporter(exporter, opentelemetry::runtime::Tokio)
            }
            _ => builder,
        })
    }
}