mod rpc;

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
}

pub use tonic::transport::Error as TransportError;
pub use tonic::Request;
