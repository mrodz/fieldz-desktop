mod rpc;

pub mod proto {
	pub use super::rpc::algo_input;
}

pub mod server {
	pub use super::*;

	pub mod scheduler {
		pub use super::rpc::{ScheduleManager, algo_input::scheduler_server::SchedulerServer};
	}
}

pub mod client {
	pub use super::rpc::algo_input::scheduler_client::SchedulerClient;
}

pub use tonic::transport::Error as TransportError;
pub use tonic::Request;