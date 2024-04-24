use std::net::SocketAddr;

use grpc_server::server::scheduler::{ScheduleManager, SchedulerServer};
use tonic::transport::Server;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

async fn get_address() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    "[::1]:10000".parse::<SocketAddr>().map_err(|e| e.into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_address().await?;

    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing::info!("Hosting gRPC server on: {addr}");

    let scheduler = SchedulerServer::new(ScheduleManager);

    tracing::info!("Using server: {scheduler:?}");

    Server::builder().add_service(scheduler).serve(addr).await?;

    Ok(())
}
