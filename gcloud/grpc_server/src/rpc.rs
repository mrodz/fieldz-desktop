use std::fmt::Debug;
use std::pin::Pin;
use std::time::Instant;

use algo_input::scheduler_server::Scheduler;
use algo_input::{ScheduledInput, ScheduledOutput};
use backend::{FieldLike, PlayableTeamCollection, TeamLike};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

pub mod algo_input {
    tonic::include_proto!("algo_input");
}

#[derive(Debug, Default)]
pub struct ScheduleManager;

impl TeamLike for algo_input::Team {
    fn unique_id(&self) -> i32 {
        self.unique_id
            .try_into()
            .expect("unique team id could not fit in a 32-bit int")
    }
}

impl FieldLike for algo_input::Field {
    fn unique_id(&self) -> i32 {
        self.unique_id
            .try_into()
            .expect("unique field id could not fit in a 32-bit int")
    }

    fn time_slots(&self) -> impl AsRef<[(backend::ProtobufAvailabilityWindow, u8)]> {
        self.time_slots
            .iter()
            .map(|time_slot| {
                let start = time_slot.start;
                let end = time_slot.end;
                (
                    (start, end),
                    time_slot
                        .concurrency
                        .try_into()
                        .expect("could not fit concurrency in an 8-bit unsigned int"),
                )
            })
            .collect::<Vec<_>>()
    }
}

impl PlayableTeamCollection for algo_input::PlayableTeamCollection {
    type Team = algo_input::Team;

    fn teams(&self) -> impl AsRef<[Self::Team]> {
        &self.teams
    }
}

impl From<algo_input::ScheduledInput>
    for backend::ScheduledInput<
        algo_input::Team,
        algo_input::PlayableTeamCollection,
        algo_input::Field,
    >
{
    fn from(value: algo_input::ScheduledInput) -> Self {
        backend::ScheduledInput::new(value.team_groups, value.fields)
    }
}

impl algo_input::Field {
    /// This can't be a normal `impl` because the [`From`] trait
    /// is reflexive and [`algo_input::Field`] implements [`FieldLike`]
    pub fn generic_from_impl<F>(value: F) -> Self
    where
        F: FieldLike + Clone + Debug + PartialEq,
    {
        Self {
            unique_id: value
                .unique_id()
                .try_into()
                .expect("field id could not fit"),
            time_slots: value
                .time_slots()
                .as_ref()
                .iter()
                .map(|(time_slot, concurrency)| algo_input::TimeSlot {
                    start: time_slot.0,
                    end: time_slot.1,
                    concurrency: *concurrency as u32,
                })
                .collect::<Vec<_>>(),
        }
    }
}

impl<T, F> From<backend::Output<T, F>> for algo_input::ScheduledOutput
where
    T: TeamLike + Clone + Debug + PartialEq,
    F: FieldLike + Clone + Debug + PartialEq,
{
    fn from(value: backend::Output<T, F>) -> Self {
        algo_input::ScheduledOutput {
            time_slots: value
                .time_slots()
                .iter()
                .map(|reservation| {
                    use backend::Booking::*;

                    let booking = match reservation.booking() {
                        Empty => None,
                        Booked {
                            home_team,
                            away_team,
                        } => Some(algo_input::reservation::Booked {
                            home_team: Some(algo_input::Team {
                                unique_id: home_team.unique_id().try_into().expect("home team"),
                            }),
                            away_team: Some(algo_input::Team {
                                unique_id: away_team.unique_id().try_into().expect("away team"),
                            }),
                        }),
                    };

                    algo_input::Reservation {
                        field: Some(algo_input::Field::generic_from_impl(
                            reservation.field().clone(),
                        )),
                        booking,
                        start: reservation.start(),
                        end: reservation.end(),
                    }
                })
                .collect::<Vec<_>>(),
        }
    }
}

#[tonic::async_trait]
impl Scheduler for ScheduleManager {
    type ScheduleStream =
        Pin<Box<dyn Stream<Item = Result<ScheduledOutput, Status>> + Send + 'static>>;

    async fn schedule(
        &self,
        request: Request<tonic::Streaming<ScheduledInput>>, // Accept request of type HelloRequest
    ) -> Result<Response<Self::ScheduleStream>, Status> {
        let mut stream = request.into_inner();

        let _span = tracing::info_span!("RPC Start");

        let output = async_stream::try_stream! {
            while let Some(schedule_payload) = stream.next().await {
                let schedule_payload: algo_input::ScheduledInput = schedule_payload?;

                let backend_payload: backend::ScheduledInput<_, _, _> = schedule_payload.into();

                tracing::info!("Recieved payload (fields: {}, teams: {})", backend_payload.fields().as_ref().len(), backend_payload.teams_len());

                let start = Instant::now();

                let result = backend::schedule(backend_payload);

                let end = Instant::now();

                tracing::info!("Scheduled in {:?}", end.duration_since(start));

                if let Err(ref e) = result {
                    tracing::error!("{e}");
                }

                yield
                    result
                    .map_err(|e| Status::new(tonic::Code::Cancelled, e.to_string()))?
                    .into();
            }
        };

        Ok(Response::new(Box::pin(output) as Self::ScheduleStream))
    }
}
