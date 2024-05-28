use std::fmt::Debug;

use anyhow::Context;
use backend::{FieldLike, PlayableTeamCollection, ScheduledInput, TeamLike};
use db::{errors::SaveScheduleError, CompiledSchedule};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use url::Url;

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
    let scheduler_endpoint = get_scheduler_url();

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
        x => unreachable!("should not have recieved protobuf enum ident of {x} for non-watch"),
    })
}

pub(crate) fn get_scheduler_url() -> String {
    std::env::var("SCHEDULER_SERVER_URL")
        .expect("this app was not built with the correct setup to talk to the scheduler server")
}

pub const fn keyring_service() -> &'static str {
    "fieldz-auth0-service"
}

pub fn keyring_account() -> String {
    whoami::username()
}

pub fn auth0_domain() -> String {
    std::env::var("AUTH0_DOMAIN").expect("Missing `AUTH0_DOMAIN`")
}

pub fn auth0_client_id() -> String {
    std::env::var("AUTH0_CLIENT_ID").expect("Missing `AUTH0_CLIENT_ID`")
}

pub const fn auth0_redirect_uri() -> &'static str {
    "fieldz:auth"
}

#[derive(Deserialize)]
struct TokenExchangeResponse {
    access_token: String,
    id_token: String,
    refresh_token: String,
}

#[derive(Debug)]
pub enum AuthError {
    RefreshTokenMissing,
    Http,
    InvalidUrl,
    InvalidJson,
}

pub async fn load_tokens(callback_url: &str) -> Result<(), anyhow::Error> {
    let parsed_url = Url::parse(callback_url)?;
    let mut hash_query = parsed_url.query_pairs();
    let code = hash_query
        .find_map(|(k, v)| if k == "code" { Some(v) } else { None })
        .context("callback URL did not contain query parameter `code`")?;

    // let refresh_token = refresh_token_entry.get_password()?;

    let token_exchange_body = json!({
        "grant_type": "authorization_code",
        "client_id": auth0_client_id(),
        "code": code,
        "redirect_uri": auth0_redirect_uri(),
    })
    .to_string();

    let client = reqwest::Client::new();

    let response = client
        .post(format!("https://{}/oauth/token", auth0_domain()))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(token_exchange_body)
        .send()
        .await?;

    let response_text = response.text().await?;
    let token_exchange: TokenExchangeResponse = serde_json::from_str(&response_text)?;

    let refresh_token_entry = keyring::Entry::new(keyring_service(), &keyring_account())?;
    refresh_token_entry.set_password(&token_exchange.refresh_token);


    Ok(())
}

pub fn get_auth_url() -> String {
    let audience = urlencoding::encode(
        &std::env::var("AUTH0_API_IDENTIFIER").expect("Missing `AUTH0_API_IDENTIFIER`"),
    )
    .into_owned();

    let scope = urlencoding::encode("openid profile offline_access").into_owned();
    let redirect_uri_encoded = urlencoding::encode(auth0_redirect_uri()).into_owned();
    let domain = auth0_domain();
    let client_id = auth0_client_id();

    format!("https://{domain}/authorize?audience={audience}&scope={scope}&response_type=code&client_id={client_id}&redirect_uri={redirect_uri_encoded}")
}
