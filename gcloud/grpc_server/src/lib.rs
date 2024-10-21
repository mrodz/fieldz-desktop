mod jwt;
mod rpc;

use serde::Deserialize;
use serde::Serialize;
pub use tonic::metadata;
pub use tonic::transport::{Channel, Error as TransportError};
pub use tonic::{Request, Status};
pub use tonic_health::pb::{HealthCheckRequest, HealthCheckResponse};

pub mod proto {
    pub use super::rpc::algo_input;
}

pub mod server {
    use super::*;

    pub mod scheduler {
        pub use super::rpc::{algo_input::scheduler_server::SchedulerServer, ScheduleManager};
    }
}

pub mod client {
    pub use super::rpc::algo_input::scheduler_client::SchedulerClient;
    pub use tonic_health::pb::health_client::HealthClient;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchedulerUsage {
    runs: u32,
}

pub(crate) async fn signal_usage(
    user_id: String,
) -> Result<SchedulerUsage, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let usage_endpoint = std::env::var("QUOTA_SERVER_URL").unwrap();

    let response = reqwest::get(format!("{usage_endpoint}?uid={user_id}"))
        .await
        .map_err(Box::new)?;

    let usage: SchedulerUsage = response.json().await.map_err(Box::new)?;

    tracing::info!("User {user_id} has usage {usage:?}");

    Ok(usage)
}
