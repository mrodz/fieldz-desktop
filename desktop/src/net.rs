use std::fmt::Debug;

use backend::{CoachConflictLike, FieldLike, PlayableTeamCollection, ScheduledInput, TeamLike};
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
    #[error("Missing environment variable: {0}")]
    BadEnvironment(String),
    #[error("The bearer JWT token is poorly encoded")]
    AuthorizationHeaderMissingOrBad,
}

pub(crate) async fn send_grpc_schedule_request<T, P, F, C>(
    input: impl AsRef<[ScheduledInput<T, P, F, C>]>,
    authorization_token: String,
) -> Result<CompiledSchedule, ScheduleRequestError>
where
    T: TeamLike + Clone + Debug + PartialEq + Send,
    P: PlayableTeamCollection<Team = T> + Send,
    F: FieldLike + Clone + Debug + PartialEq + Send,
    C: CoachConflictLike + Send,
{
    let scheduler_endpoint = try_get_scheduler_url().map_err(|msg| {
        ScheduleRequestError::BadEnvironment(format!("environment:scheduler_url {msg}"))
    })?;

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
                coach_conflicts: non_message
                    .coach_conflicts()
                    .iter()
                    .map(
                        |coach_conflict| grpc_server::proto::algo_input::CoachConflict {
                            region_id: coach_conflict
                                .region_id()
                                .try_into()
                                .expect("coach conflict region id"),
                            unique_id: coach_conflict
                                .unique_id()
                                .try_into()
                                .expect("coach conflict id"),
                            teams: coach_conflict
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
            .inspect_err(|e| eprintln!("{e} {}:{}", line!(), column!()))
            .map_err(|_err| ScheduleRequestError::AuthorizationHeaderMissingOrBad)?,
    );

    let response = client
        .schedule(request)
        .await
        .map_err(|e| ScheduleRequestError::RPCError(e.to_string()))?;

    let mut inbound = response.into_inner();

    let mut reservations = vec![];

    while let Some(schedule) = inbound.next().await {
        let schedule = schedule.map_err(|e| ScheduleRequestError::RPCError(format!("{e}")))?;
        reservations.push(schedule);
    }

    Ok(CompiledSchedule::new(reservations))
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerHealth {
    Unknown,
    Serving,
    NotServing,
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum HealthProbeError {
    #[error("could not establish a channel with the backend: transport error: {0}")]
    BadChannel(String),
    #[error("could not probe: service not found or unavailable: {0}")]
    ProbeFail(String),
}

pub async fn health_probe() -> Result<ServerHealth, HealthProbeError> {
    let scheduler_endpoint = get_scheduler_url();

    let channel = grpc_server::Channel::from_shared(scheduler_endpoint)
        .expect("invalid URI")
        .connect()
        .await
        .map_err(|err| HealthProbeError::BadChannel(err.to_string()))?;

    let mut health_client = grpc_server::client::HealthClient::new(channel);

    let response = health_client
        .check(grpc_server::HealthCheckRequest {
            service: "algo_input.Scheduler".into(),
        })
        .await
        .map_err(|e| HealthProbeError::ProbeFail(e.to_string()))?;

    let health_check_response = response.into_inner();

    Ok(match health_check_response.status {
        0 => ServerHealth::Unknown,
        1 => ServerHealth::Serving,
        2 => ServerHealth::NotServing,
        x => unreachable!("should not have received protobuf enum ident of {x} for non-watch"),
    })
}

pub(crate) fn try_get_scheduler_url() -> Result<String, std::env::VarError> {
    std::env::var("SCHEDULER_SERVER_URL")
}

pub(crate) fn get_scheduler_url() -> String {
    try_get_scheduler_url()
        .expect("this app was not built with the correct setup to talk to the scheduler server")
}

pub(crate) fn try_get_auth_url() -> Result<String, std::env::VarError> {
    std::env::var("AUTH_SERVER_URL")
}

#[allow(unused)]
pub(crate) fn get_auth_url() -> String {
    try_get_auth_url()
        .expect("this app was not built with the correct setup to talk to the auth server")
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct OAuthAccessTokenExchange {
    access_token: String,
}

pub async fn get_github_access_token(
    code: String,
) -> Result<OAuthAccessTokenExchange, anyhow::Error> {
    let client = reqwest::Client::new();

    let response = client
        .get(try_get_auth_url()?)
        .query(&[
            ("platform", "github"),
            ("code", urlencoding::encode(&code).as_ref()),
        ])
        .send()
        .await
        .inspect_err(|e| eprintln!("{e}"))?;

    let response_text = response.text().await.inspect_err(|e| eprintln!("{e}"))?;

    Ok(serde_json::from_str(&response_text)?)
}
