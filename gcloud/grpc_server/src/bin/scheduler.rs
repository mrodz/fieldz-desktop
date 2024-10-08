use std::net::SocketAddr;

use grpc_server::server::scheduler::{ScheduleManager, SchedulerServer};
use tonic::transport::Server;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn get_address() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let port = "50051";

    format!("[::0]:{port}")
        .parse::<SocketAddr>()
        .map_err(|e| e.into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    std::env::var("QUOTA_SERVER_URL").map_err(Box::new)?;

    let addr = get_address()?;

    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        .with_ansi(false)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing::info!("Hosting gRPC server on: {addr}");

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();

    health_reporter
        .set_serving::<SchedulerServer<ScheduleManager>>()
        .await;

    let scheduler = SchedulerServer::new(ScheduleManager);

    tracing::info!("Using server: {scheduler:?}");
    tracing::info!("Health check active");

    Server::builder()
        .add_service(health_service)
        .add_service(scheduler)
        .serve(addr)
        .await?;

    Ok(())
}
