use std::fmt::Debug;

use backend::{FieldLike, PlayableTeamCollection, ScheduledInput, TeamLike};
use db::{errors::SaveScheduleError, CompiledSchedule};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ScheduleRequestError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("network rpc operation failed: `{0}`")]
    RPCError(String),
    #[error("could not save schedule to %APPDATA% folder")]
    NoSaveAppData,
    #[error(transparent)]
    SaveScheduleError(#[from] SaveScheduleError),
}

pub(crate) async fn send_grpc_schedule_request<T, P, F>(
    input: impl AsRef<[ScheduledInput<T, P, F>]>,
    authorization_token: String,
) -> Result<CompiledSchedule, ScheduleRequestError>
where
    T: TeamLike + Clone + Debug + PartialEq + Send,
    P: PlayableTeamCollection<Team = T> + Send,
    F: FieldLike + Clone + Debug + PartialEq + Send,
{
    let scheduler_endpoint = std::env::var("SCHEDULER_SERVER_URL")
        .expect("this app was not built with the correct setup to talk to the scheduler server");

    let messages = input
        .as_ref()
        .iter()
        .enumerate()
        .map(
            |(i, non_message)| grpc_server::proto::algo_input::ScheduledInput {
                fields: non_message
                    .fields()
                    .iter()
                    .map(|field| grpc_server::proto::algo_input::Field {
                        unique_id: field.unique_id().try_into().expect("field id"),
                        time_slots: field
                            .time_slots()
                            .as_ref()
                            .iter()
                            .map(|(time_slot, concurrency)| {
                                grpc_server::proto::algo_input::TimeSlot {
                                    start: time_slot.0,
                                    end: time_slot.1,
                                    concurrency: *concurrency as u32,
                                }
                            })
                            .collect(),
                    })
                    .collect(),
                team_groups: non_message
                    .team_groups()
                    .iter()
                    .map(
                        |team_group| grpc_server::proto::algo_input::PlayableTeamCollection {
                            teams: team_group
                                .teams()
                                .as_ref()
                                .iter()
                                .map(|team| grpc_server::proto::algo_input::Team {
                                    unique_id: team.unique_id().try_into().expect("team id"),
                                })
                                .collect(),
                        },
                    )
                    .collect(),
                unique_id: i as u32,
            },
        )
        .collect::<Vec<_>>();

    let mut client = grpc_server::client::SchedulerClient::connect(scheduler_endpoint)
        .await
        .map_err(|e| {
            ScheduleRequestError::RPCError(format!(
                "could not establish client: {e} ({}:{})",
                line!(),
                column!()
            ))
        })?;

    let outbound = async_stream::stream! {
        for message in messages {
            yield message;
        }
    };

    let mut request = grpc_server::Request::new(outbound);

    request.metadata_mut().append(
        "authorization",
        format!("Bearer {authorization_token}")
            .parse()
            .expect("bearer jwt token"),
    );

    let response = client
        .schedule(request)
        .await
        .map_err(|e| ScheduleRequestError::RPCError(e.to_string()))?;

    let mut inbound = response.into_inner();

    let mut reservations = vec![];

    while let Some(schedule) = inbound.next().await {
        let schedule = schedule.map_err(|e| ScheduleRequestError::RPCError(format!("{e}")))?;
        println!("{schedule:?}");
        reservations.push(schedule);
    }

    Ok(CompiledSchedule::new(reservations))
}
