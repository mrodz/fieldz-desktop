use backend::ReservationType;
use chrono::{DateTime, Utc};
use db::Client;

pub(crate) struct ScheduleCSVRecord {
	reservation_type: ReservationType,
	team_combination: Vec<db::team_group::Model>,
	region: db::region::Model,
	field: db::field::Model,
	start: DateTime<Utc>,
	end: DateTime<Utc>,
	home_team: db::team::Model,
	away_team: db::team::Model,
}

const COLUMN_LENGTH: usize = 10;
static CSV_COLUMNS: [&str; COLUMN_LENGTH] = ["reservation type", "team combination", "region", "field", "start", "end", "home name", "home region", "away name", "away region"];

impl ScheduleCSVRecord {
	pub fn columns() -> &'static [&'static str] {
		&CSV_COLUMNS
	}

	pub fn new(scheduled_output: &grpc_server::proto::algo_input::ScheduledOutput, client: &Client) -> Self {
		todo!()
	}
}

impl IntoIterator for ScheduleCSVRecord {
	type Item = String;
	type IntoIter = std::array::IntoIter<Self::Item, COLUMN_LENGTH>;

	fn into_iter(self) -> Self::IntoIter {
		let columns: [String; COLUMN_LENGTH] = todo!();
		columns.into_iter()
	}
}