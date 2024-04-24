use std::net::SocketAddr;

use grpc_server::server::scheduler::{ScheduleManager, SchedulerServer};
use tonic::transport::Server;

async fn get_address() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    "[::1]:10000".parse::<SocketAddr>().map_err(|e| e.into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_address().await?;

    let scheduler = SchedulerServer::new(ScheduleManager);

    Server::builder().add_service(scheduler).serve(addr).await?;

    Ok(())
}
