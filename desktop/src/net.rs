use std::{borrow::Cow, fmt::Debug};

use backend::{CoachConflictLike, FieldLike, PlayableTeamCollection, ScheduledInput, TeamLike};
use db::{errors::SaveScheduleError, CompiledSchedule};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::twitter::{TwitterOAuthFlow, TwitterOAuthFlowStageOne};

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

    let mut client = grpc_server::client::SchedulerClient::connect(scheduler_endpoint.into_owned())
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
    #[error("Missing environment variable: {0}")]
    BadEnvironment(String),
}

pub async fn health_probe() -> Result<ServerHealth, HealthProbeError> {
    let scheduler_endpoint = try_get_scheduler_url().map_err(|msg| {
        HealthProbeError::BadEnvironment(format!("environment:scheduler_url {msg}"))
    })?;

    let channel = grpc_server::Channel::from_shared(scheduler_endpoint.into_owned())
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

pub(crate) fn try_get_scheduler_url() -> Result<Cow<'static, str>, std::env::VarError> {
    if let Some(var) = option_env!("SCHEDULER_SERVER_URL").map(Cow::Borrowed) {
        return Ok(var);
    }

    println!("Warning: using runtime environment variable: SCHEDULER_SERVER_URL");

    std::env::var("SCHEDULER_SERVER_URL").map(Cow::Owned)
}

pub(crate) fn try_get_auth_url() -> Result<Cow<'static, str>, std::env::VarError> {
    if let Some(var) = option_env!("AUTH_SERVER_URL").map(Cow::Borrowed) {
        return Ok(var);
    }

    println!("Warning: using runtime environment variables: AUTH_SERVER_URL");

    std::env::var("AUTH_SERVER_URL").map(Cow::Owned)
}

pub(crate) fn try_get_twitter_signing_url() -> Result<Cow<'static, str>, std::env::VarError> {
    if let Some(var) = option_env!("TWITTER_SIGNING_URL").map(Cow::Borrowed) {
        return Ok(var);
    }

    println!("Warning: using runtime environment variables: TWITTER_SIGNING_URL");

    std::env::var("TWITTER_SIGNING_URL").map(Cow::Owned)
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct OAuthAccessTokenExchange {
    access_token: String,
    refresh_token: Option<String>,
}

pub async fn get_github_access_token(
    code: String,
    client: &reqwest::Client,
) -> Result<OAuthAccessTokenExchange, anyhow::Error> {
    let response = client
        .get(try_get_auth_url()?.as_ref())
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

// pub fn twitter_authorization_header(
//     oauth_callback: impl AsRef<str>,
//     oauth_consumer_key: impl AsRef<str>,
//     oauth_nonce: impl AsRef<str>,
//     oauth_signature: impl AsRef<str>,
// ) -> String {
//     // "OAuth ".to_owned()
//     //     + "oauth_callback="
//     //     + urlencoding::encode(oauth_callback.as_ref()).as_ref()
//     //     + ",oauth_consumer_key="
//     //     + urlencoding::encode(oauth_consumer_key.as_ref()).as_ref()
//     //     + ",oauth_nonce="
//     //     + urlencoding::encode(oauth_nonce.as_ref()).as_ref()
//     //     + ",oauth_signature="
//     //     + urlencoding
// }

pub async fn get_twitter_access_token(
    client_id: String,
    code: String,
    code_challenge: String,
    port: u32,
    client: &reqwest::Client,
) -> Result<OAuthAccessTokenExchange, anyhow::Error> {
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let headers = &[
        ("code", urlencoding::encode(&code)),
        ("client_id", urlencoding::encode(&client_id)),
        ("grant_type", Cow::Borrowed("authorization_code")),
        ("code_verifier", urlencoding::encode(&code_challenge)),
        ("redirect_uri", Cow::Owned(redirect_uri)),
    ];

    let response = client
        .post(format!("https://api.twitter.com/2/oauth2/token"))
        .form(&headers)
        .send()
        .await
        .inspect_err(|e| eprintln!("{e}"))?;

    let response_text = response.text().await.inspect_err(|e| eprintln!("{e}"))?;

    Ok(serde_json::from_str(&response_text)?)
}

pub async fn begin_twitter_oauth_transaction(
    port: u32,
    client: &reqwest::Client,
) -> Result<TwitterOAuthFlowStageOne, anyhow::Error> {
    let signing_endpoint = Url::parse(&try_get_twitter_signing_url()?)?;

    let auth_flow = TwitterOAuthFlow::new("oauth_consumer_key" /* TODO */, signing_endpoint);

    let redirect_uri = format!("http://127.0.0.1:{port}");

    Ok(auth_flow
        .get_request_token(Url::parse(&redirect_uri)?, client)
        .await?)
}
