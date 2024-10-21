mod pre_schedule_report;

use backend::{
    CoachConflictLike, FieldLike, PlayableTeamCollection, ProtobufAvailabilityWindow,
    ScheduledInput, TeamLike,
};
// use communication::{FieldLike, ProtobufAvailabilityWindow, TeamLike};
use itertools::Itertools;
pub use pre_schedule_report::*;

pub mod errors;
use errors::*;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ops::Deref;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{serde::ts_milliseconds, DateTime};
use chrono::{Local, Utc};

#[allow(unused_imports)]
pub(crate) mod entity_local_exports {
    pub use coach_conflict::{
        ActiveModel as ActiveCoachConflict, Entity as CoachConflictEntity,
        Model as CoachConflictModel,
    };
    use entity::*;
    pub use field::{ActiveModel as ActiveField, Entity as FieldEntity, Model as Field};
    pub use region::{ActiveModel as ActiveRegion, Entity as RegionEntity, Model as Region};
    pub use reservation_type::{
        ActiveModel as ActiveReservationType, Entity as ReservationTypeEntity,
        Model as ReservationType,
    };
    pub use schedule::{
        ActiveModel as ActiveSchedule, Entity as ScheduleEntity, Model as Schedule,
    };
    pub use schedule_game::{
        ActiveModel as ActiveScheduleGame, Entity as ScheduleGameEntity, Model as ScheduleGame,
    };
    pub use target::{ActiveModel as ActiveTarget, Entity as TargetEntity, Model as Target};
    pub use team::{ActiveModel as ActiveTeam, Entity as TeamEntity, Model as Team};
    pub use team_group::{
        ActiveModel as ActiveTeamGroup, Entity as TeamGroupEntity, Model as TeamGroup,
    };
    pub use time_slot::{
        ActiveModel as ActiveTimeSlot, Entity as TimeSlotEntity, Model as TimeSlot,
    };
}

use entity_local_exports::*;

use migration::{Expr, IntoCondition, Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, FromQueryResult, IntoActiveModel,
    JoinType, Order, PaginatorTrait, QueryFilter, QuerySelect, RelationTrait, Select, Set,
    TransactionError, TransactionTrait, TryIntoModel, UpdateResult, Value,
};
use sea_orm::{Database, DatabaseConnection, EntityTrait};
pub use sea_orm::{DbErr, DeleteResult};
use sea_orm::{EntityOrSelect, ModelTrait};
use sea_orm::{IntoSimpleExpr, QueryOrder};

pub use entity::*;
use serde::{Deserialize, Serialize};

pub type DBResult<T> = Result<T, DbErr>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    connection_url: String,
}

impl Config {
    pub fn new(connection_url: impl Into<String>) -> Self {
        Self {
            connection_url: connection_url.into(),
        }
    }
}

#[derive(Debug)]
pub struct Client {
    connection: DatabaseConnection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
struct RegionName(String);

pub trait Validator {
    type Error;
    fn validate(&self) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRegionInput {
    title: RegionName,
}

impl Validator for RegionName {
    type Error = RegionNameValidationError;
    fn validate(&self) -> Result<(), Self::Error> {
        let content = self
            .0
            .trim_start_matches(char::is_whitespace)
            .trim_end_matches(char::is_whitespace);
        let len = content.len();

        if content.is_empty() {
            return Err(RegionNameValidationError::EmptyName);
        }

        if len > 64 {
            return Err(RegionNameValidationError::NameTooLong { len });
        }

        Ok(())
    }
}

impl Validator for CreateRegionInput {
    type Error = RegionValidationError;
    fn validate(&self) -> Result<(), Self::Error> {
        self.title.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateFieldInput {
    name: String,
    region_id: i32,
}

impl CreateFieldInput {
    pub fn validate(&self) -> Result<(), FieldValidationError> {
        let len = self.name.len();

        if self.name.is_empty() {
            return Err(FieldValidationError::EmptyName);
        }

        if len > 64 {
            return Err(FieldValidationError::NameTooLong { len });
        }

        // add more checks if the fields change...

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct NameMax64(String);

impl Deref for NameMax64 {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTeamInput {
    name: NameMax64,
    region_id: i32,
    tags: Vec<String>,
}

impl Validator for NameMax64 {
    type Error = NameMax64ValidationError;
    fn validate(&self) -> Result<(), Self::Error> {
        let content = self
            .0
            .trim_start_matches(char::is_whitespace)
            .trim_end_matches(char::is_whitespace);

        let len = content.len();

        if content.is_empty() {
            return Err(NameMax64ValidationError::EmptyName);
        }

        if len > 64 {
            return Err(NameMax64ValidationError::NameTooLong { len });
        }

        Ok(())
    }
}

impl CreateTeamInput {
    pub fn validate(&self) -> Result<(), NameMax64ValidationError> {
        self.name.validate()?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TeamExtension {
    team: Team,
    tags: Vec<TeamGroup>,
}

impl TeamLike for TeamExtension {
    fn unique_id(&self) -> i32 {
        self.team.id
    }
}

impl TeamExtension {
    pub fn new(team: Team, mut tags: Vec<TeamGroup>) -> Self {
        tags.sort_by_key(|group| group.id);
        Self { tags, team }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldExtension {
    field_id: i32,
    time_slots: Vec<TimeSlotExtension>,
}

impl FieldLike for FieldExtension {
    fn time_slots(&self) -> impl AsRef<[(ProtobufAvailabilityWindow, u8)]> {
        self.time_slots
            .iter()
            .map(|not_ready| {
                let (window, concurrency) = not_ready.to_scheduler_input();
                (window.to_protobuf_window(), concurrency)
            })
            .collect::<Vec<_>>()
    }

    fn unique_id(&self) -> i32 {
        self.field_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TeamCollection {
    tags: Vec<TeamGroup>,
    teams: Vec<TeamExtension>,
}

impl TeamCollection {
    pub fn new(tags: Vec<TeamGroup>, teams: Vec<TeamExtension>) -> Self {
        Self { tags, teams }
    }
}

impl PlayableTeamCollection for TeamCollection {
    type Team = TeamExtension;
    fn teams(&self) -> impl AsRef<[Self::Team]> {
        &self.teams
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTimeSlotInput {
    field_id: i32,
    reservation_type_id: i32,
    #[serde(with = "ts_milliseconds")]
    start: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    end: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSlotExtension {
    time_slot: TimeSlot,
    reservation_type: ReservationType,
    /// [`Option#None`] means use [`reservation_type`]
    custom_matches: Option<i32>,
}

impl TimeSlotExtension {
    pub(crate) fn matches_played(&self) -> i32 {
        if let Some(matches) = self.custom_matches {
            matches
        } else {
            self.reservation_type.default_sizing
        }
    }

    pub(crate) fn to_scheduler_input(&self) -> (backend::AvailabilityWindow, u8) {
        let start_str = &self.time_slot.start;
        let end_str = &self.time_slot.end;

        let start = DateTime::parse_from_rfc3339(start_str)
            .expect("time slot START is malformatted")
            .to_utc();
        let end = DateTime::parse_from_rfc3339(end_str)
            .expect("time slot END is malformatted")
            .to_utc();

        (
            backend::AvailabilityWindow::new(start, end)
                .context("Could not transform time slot from record to scheduler parameter")
                .unwrap(),
            self.matches_played()
                .try_into()
                .expect("matches played could not fit into an 8-bit int"),
        )
    }
}

#[derive(FromQueryResult)]
pub(crate) struct TimeSlotSelectionTypeAggregate {
    /// [`TimeSlot`]
    time_slot_id: i32,
    /// [`TimeSlot`]
    field_id: i32,
    /// [`TimeSlot`]
    start: String,
    /// [`TimeSlot`]
    end: String,
    /// [`TimeSlot`]
    custom_matches: Option<i32>,
    /// [`ReservationType`]
    reservation_type_id: i32,
    /// [`ReservationType`]
    name: String,
    /// [`ReservationType`]
    description: Option<String>,
    /// [`ReservationType`]
    color: String,
    /// [`ReservationType`]
    default_sizing: i32,
    /// [`ReservationType`]
    is_practice: bool,
}

/// To selects everything needed to build a [`TimeSlotSelectionTypeAggregate`].
///
/// # Tables made available:
/// - `time_slot`
/// - `field`
/// - `reservation_type`
/// - `reservation_type_time_slot_join`
/// - `reservation_type_field_size_join`
///
/// To execute the `SELECT`, use:
/// ```rs
/// select_time_slot_extension()
///     /*
///      * ... Add conditions and more here.
///      * For example:
///      *
///      * .filter(Condition::all().add(field::Column::Id.eq(field_id)))
///      *
///      */
///     .into_model::<TimeSlotSelectionTypeAggregate>()
///     /*
///      * ... Your execution here.
///      * For example:
///      *
///      * .all(&self.connection)
///      *
///      */
///     .await
///     .map(|v| v.into_iter().map(Into::into).collect())
/// ```
pub(crate) fn select_time_slot_extension() -> Select<TimeSlotEntity> {
    use reservation_type::Column as R;
    use time_slot::Column as T;

    TimeSlotEntity::find()
        .select_only()
        .column_as(T::Id, "time_slot_id")
        .column_as(T::FieldId, "field_id")
        .column_as(T::Start, "start")
        .column_as(T::End, "end")
        .column_as(R::Id, "reservation_type_id")
        .column_as(R::Name, "name")
        .column_as(R::Description, "description")
        .column_as(R::Color, "color")
        .column_as(R::DefaultSizing, "default_sizing")
        .column_as(
            reservation_type_field_size_join::Column::Size,
            "custom_matches",
        )
        .column_as(R::IsPractice, "is_practice")
        .join(JoinType::LeftJoin, time_slot::Relation::Field.def())
        .join(
            JoinType::LeftJoin,
            time_slot::Relation::ReservationTypeTimeSlotJoin.def(),
        )
        .join(
            JoinType::LeftJoin,
            reservation_type_time_slot_join::Relation::ReservationType.def(),
        )
        .join(
            JoinType::LeftJoin,
            field::Relation::ReservationTypeFieldSizeJoin
                .def()
                .on_condition(|_left, right| {
                    Expr::col((
                        right,
                        reservation_type_field_size_join::Column::ReservationType,
                    ))
                    .equals(reservation_type::Column::Id.as_column_ref())
                    .into_condition()
                }),
        )
}

impl From<TimeSlotSelectionTypeAggregate> for TimeSlotExtension {
    fn from(value: TimeSlotSelectionTypeAggregate) -> Self {
        Self {
            reservation_type: ReservationType {
                id: value.reservation_type_id,
                color: value.color,
                description: value.description,
                name: value.name,
                default_sizing: value.default_sizing,
                is_practice: value.is_practice,
            },
            time_slot: TimeSlot {
                id: value.time_slot_id,
                field_id: value.field_id,
                start: value.start,
                end: value.end,
            },
            custom_matches: value.custom_matches,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveTimeSlotInput {
    field_id: Option<i32>,
    schedule_id: Option<i32>,
    id: i32,
    #[serde(with = "ts_milliseconds")]
    new_start: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    new_end: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListReservationsBetweenInput {
    #[serde(with = "ts_milliseconds")]
    start: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    end: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditRegionInput {
    id: i32,
    name: Option<RegionName>,
}

impl Validator for EditRegionInput {
    type Error = EditRegionError;
    fn validate(&self) -> Result<(), Self::Error> {
        if let Some(ref name) = self.name {
            name.validate()?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditTeamInput {
    id: i32,
    name: Option<NameMax64>,
    tags: Option<Vec<String>>,
}

impl Validator for EditTeamInput {
    type Error = EditTeamError;

    fn validate(&self) -> Result<(), Self::Error> {
        if let Some(ref name) = self.name {
            name.validate().map_err(EditTeamError::ValidationError)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditScheduleInput {
    id: i32,
    name: Option<NameMax64>,
}

impl Validator for EditScheduleInput {
    type Error = EditScheduleError;

    fn validate(&self) -> Result<(), Self::Error> {
        if let Some(ref name) = self.name {
            name.validate()?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TargetExtension {
    target: Target,
    groups: Vec<TeamGroup>,
}

impl TargetExtension {
    pub async fn new<C>(target: Target, connection: &C) -> DBResult<Self>
    where
        C: ConnectionTrait,
    {
        let groups = TeamGroupEntity::find()
            .join(
                JoinType::RightJoin,
                team_group::Relation::TargetGroupJoin.def(),
            )
            .join(
                JoinType::LeftJoin,
                target_group_join::Relation::Target.def(),
            )
            .filter(target::Column::Id.eq(target.id))
            .all(connection)
            .await?;

        Ok(Self { target, groups })
    }

    pub async fn many_new<C>(targets: impl AsRef<[Target]>, connection: &C) -> DBResult<Vec<Self>>
    where
        C: ConnectionTrait,
    {
        let id_vec = targets.as_ref().iter().map(|x| x.id).collect::<Vec<_>>();
        let mut result = HashMap::<i32, Vec<TeamGroup>>::with_capacity(id_vec.len());

        #[derive(FromQueryResult)]
        struct TeamGroupAndTargetId {
            id: i32,
            name: String,
            usages: i32,
            target_id: i32,
        }

        let groups: Vec<TeamGroupAndTargetId> = TeamGroupEntity::find()
            .column_as(target::Column::Id, "target_id")
            .join(
                JoinType::RightJoin,
                team_group::Relation::TargetGroupJoin.def(),
            )
            .join(
                JoinType::InnerJoin,
                target_group_join::Relation::Target.def(),
            )
            .filter(target::Column::Id.is_in(id_vec))
            .into_model::<TeamGroupAndTargetId>()
            .all(connection)
            .await?;

        for group in groups {
            let groups = result.entry(group.target_id).or_default();
            groups.push(TeamGroup {
                id: group.id,
                name: group.name,
                usages: group.usages,
            });
        }

        Ok(targets
            .as_ref()
            .iter()
            .map(|target| Self {
                target: target.clone(),
                groups: result.remove(&target.id).unwrap_or_default(),
            })
            .collect())
    }

    pub async fn is_practice<C>(&self, connection: &C) -> DBResult<Option<bool>>
    where
        C: ConnectionTrait,
    {
        println!("practice query");
        if let Some(reservation_type_id) = self.target.maybe_reservation_type {
            let maybe_reservation_type = ReservationTypeEntity::find_by_id(reservation_type_id)
                .one(connection)
                .await?;

            let Some(reservation_type) = maybe_reservation_type else {
                eprintln!(
                    "Attempting to search for non-existent reservation type: {reservation_type_id}"
                );

                return Ok(None);
            };

            return Ok(Some(reservation_type.is_practice));
        }

        Ok(None)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetOp {
    Insert,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color(String);

impl Color {
    pub fn new(input: String) -> Option<Self> {
        if input.len() != 7 {
            return None;
        }

        let mut bytes = input.bytes();

        let Some(b'#') = bytes.next() else {
            return None;
        };

        for byte in bytes {
            if !byte.is_ascii_digit() {
                return None;
            }
        }

        Some(Self(input))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateReservationTypeInput {
    name: NameMax64,
    description: Option<String>,
    color: Color,
}

impl Validator for CreateReservationTypeInput {
    type Error = CreateReservationTypeError;
    fn validate(&self) -> Result<(), Self::Error> {
        self.name
            .validate()
            .map_err(CreateReservationTypeError::Name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSupportedConcurrencyInput {
    reservation_type_ids: Vec<i32>,
    field_id: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldConcurrency {
    reservation_type_id: i32,
    field_id: i32,
    concurrency: i32,
    is_custom: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateReservationTypeConcurrencyForFieldInput {
    reservation_type_id: i32,
    field_id: i32,
    new_concurrency: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateTargetReservationTypeInput {
    target_id: i32,
    new_reservation_type_id: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledScheduleDependents {
    field_ids: BTreeSet<i32>,
    team_ids: BTreeSet<i32>,
}

impl CompiledScheduleDependents {
    pub fn new(field_ids: BTreeSet<i32>, team_ids: BTreeSet<i32>) -> Self {
        Self {
            field_ids,
            team_ids,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledSchedule {
    outputs: Vec<grpc_server::proto::algo_input::ScheduledOutput>,
}

impl CompiledSchedule {
    pub const fn new(outputs: Vec<grpc_server::proto::algo_input::ScheduledOutput>) -> Self {
        Self { outputs }
    }

    pub fn dependents(&self) -> CompiledScheduleDependents {
        let mut field_ids = BTreeSet::new();
        let mut team_ids = BTreeSet::new();

        for output in &self.outputs {
            for reservation in &output.time_slots {
                field_ids.insert(
                    reservation
                        .field
                        .as_ref()
                        .expect("no field")
                        .unique_id
                        .try_into()
                        .expect("field id too big"),
                );

                if let Some(ref booking) = reservation.booking {
                    team_ids.insert(
                        booking
                            .home_team
                            .as_ref()
                            .expect("no home team")
                            .unique_id
                            .try_into()
                            .expect("team id too big"),
                    );
                    team_ids.insert(
                        booking
                            .away_team
                            .as_ref()
                            .expect("no away team")
                            .unique_id
                            .try_into()
                            .expect("team id too big"),
                    );
                }
            }
        }

        CompiledScheduleDependents::new(field_ids, team_ids)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CopyTimeSlotsInput {
    src_start_id: i32,
    src_end_id: i32,
    #[serde(with = "ts_milliseconds")]
    dst_start: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoachConflict {
    id: i32,
    region: i32,
    coach_name: Option<String>,
    teams: Vec<Team>,
}

#[derive(Clone, PartialEq, PartialOrd)]
pub struct TeamModelWrapper(Team);

impl Deref for TeamModelWrapper {
    type Target = Team;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TeamLike for TeamModelWrapper {
    fn unique_id(&self) -> i32 {
        self.id
    }
}

impl CoachConflictLike for CoachConflict {
    type Team = TeamModelWrapper;
    fn teams(&self) -> impl AsRef<[Self::Team]> {
        self.teams
            .iter()
            .cloned()
            .map(TeamModelWrapper)
            .collect_vec()
    }

    fn region_id(&self) -> i32 {
        self.region
    }

    fn unique_id(&self) -> i32 {
        self.id
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CreateCoachConflictInput {
    region_id: i32,
    coach_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum CoachConflictTeamInputOp {
    Create,
    Delete,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoachConflictTeamInput {
    coach_conflict_id: i32,
    team_id: i32,
    op: CoachConflictTeamInputOp,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RegionMetadata {
    region_id: i32,
    team_count: u64,
    field_count: u64,
    time_slot_count: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ConflictTimeSlotSource {
    Field,
    Schedule,
}

impl Client {
    pub async fn new(config: &Config) -> Result<Self> {
        let db: DatabaseConnection = Database::connect(&config.connection_url).await?;

        if db.ping().await.is_err() {
            bail!("database did not respond to ping");
        }

        let result = Client { connection: db };

        result.up().await?;

        Ok(result)
    }

    pub async fn up(&self) -> DBResult<()> {
        Migrator::up(&self.connection, None).await
    }

    pub async fn refresh(&self) -> DBResult<()> {
        Migrator::refresh(&self.connection).await
    }

    pub async fn get_regions(&self) -> DBResult<Vec<Region>> {
        RegionEntity::find().all(&self.connection).await
    }

    pub async fn load_region(&self, id: i32) -> DBResult<Vec<Region>> {
        RegionEntity::find_by_id(id).all(&self.connection).await
    }

    pub async fn create_region(&self, input: CreateRegionInput) -> DBResult<Region> {
        RegionEntity::insert(ActiveRegion {
            title: Set(input.title.0),
            ..Default::default()
        })
        .exec_with_returning(&self.connection)
        .await
    }

    pub async fn delete_regions(&self) -> DBResult<DeleteResult> {
        RegionEntity::delete_many().exec(&self.connection).await
    }

    pub async fn delete_region(&self, id: i32) -> Result<DeleteResult, TransactionError<DbErr>> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    let stmt = TeamGroupEntity::find()
                        .join(
                            JoinType::LeftJoin,
                            team_group::Relation::TeamGroupJoin.def(),
                        )
                        .join(JoinType::LeftJoin, team_group_join::Relation::Team.def())
                        .join(JoinType::LeftJoin, team::Relation::Region.def())
                        .filter(Condition::all().add(region::Column::Id.eq(id)))
                        .order_by_asc(team_group::Column::Id)
                        .all(transaction)
                        .await?;

                    let mut iterable = stmt.iter().map(|x| x.id);

                    if let Some(mut last) = iterable.next() {
                        let mut to_sweep = 1;

                        for id in iterable {
                            if id != last {
                                Self::decrement_group_count(transaction, [last], to_sweep).await?;

                                last = id;
                                to_sweep = 1;
                            } else {
                                to_sweep += 1;
                            }
                        }

                        if to_sweep > 1 {
                            Self::decrement_group_count(transaction, [last], to_sweep).await?;
                        }
                    }

                    RegionEntity::delete(ActiveRegion {
                        id: Set(id),
                        ..Default::default()
                    })
                    .exec(transaction)
                    .await
                })
            })
            .await
    }

    pub async fn get_fields(&self, region_id: i32) -> Result<Vec<Field>> {
        let region = RegionEntity::find_by_id(region_id)
            .one(&self.connection)
            .await?
            .context("not found")?;
        region
            .find_related(FieldEntity)
            .all(&self.connection)
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn get_field(&self, field_id: i32) -> Result<Vec<Field>> {
        FieldEntity::find_by_id(field_id)
            .all(&self.connection)
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn create_field(&self, input: CreateFieldInput) -> DBResult<Field> {
        FieldEntity::insert(ActiveField {
            name: Set(input.name),
            region_owner: Set(input.region_id),
            ..Default::default()
        })
        .exec_with_returning(&self.connection)
        .await
    }

    pub async fn delete_field(&self, id: i32) -> DBResult<DeleteResult> {
        FieldEntity::delete(ActiveField {
            id: Set(id),
            ..Default::default()
        })
        .exec(&self.connection)
        .await
    }

    pub async fn create_team(
        &self,
        input: CreateTeamInput,
    ) -> Result<TeamExtension, CreateTeamError> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    if !input.tags.is_empty() {
                        let _ = TeamGroupEntity::update_many()
                            .filter(team_group::Column::Name.is_in(&input.tags))
                            .col_expr(
                                team_group::Column::Usages,
                                Expr::add(Expr::col(team_group::Column::Usages), 1),
                            )
                            .exec(transaction)
                            .await
                            .map_err(|e| CreateTeamError::DatabaseError(e.to_string()))?;
                    }

                    // This is not slow, since the result of the update (if carried out) was cached.
                    let groups = TeamGroupEntity::find()
                        .filter(team_group::Column::Name.is_in(&input.tags))
                        .all(transaction)
                        .await
                        .map_err(|e| CreateTeamError::DatabaseError(e.to_string()))?;

                    if groups.len() != input.tags.len() {
                        // Tag does not exist
                        let tags: HashSet<&String> = input.tags.iter().collect();
                        let groups: HashSet<&String> = groups.iter().map(|x| &x.name).collect();

                        let out: Vec<String> =
                            tags.difference(&groups).map(|x| (*x).clone()).collect();
                        return Err(CreateTeamError::MissingTags(out));
                    }
                    let team = ActiveTeam {
                        name: Set(input.name.0),
                        region_owner: Set(input.region_id),
                        ..Default::default()
                    }
                    .save(transaction)
                    .await
                    .map_err(|e| {
                        CreateTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                    })?;

                    let Value::Int(Some(team_id)) =
                        team.id
                            .clone()
                            .into_value()
                            .ok_or(CreateTeamError::DatabaseError(
                                "team id was not set".to_owned(),
                            ))?
                    else {
                        return Err(CreateTeamError::DatabaseError(
                            "team id is not an int or null".to_owned(),
                        ));
                    };

                    let (team, tags) = if !groups.is_empty() {
                        let mut active_models = Vec::with_capacity(groups.len());

                        for group in groups {
                            active_models.push(team_group_join::ActiveModel {
                                group: Set(group.id),
                                team: Set(team_id),
                            });
                        }

                        team_group_join::Entity::insert_many(active_models)
                            .exec(transaction)
                            .await
                            .map_err(|e| CreateTeamError::DatabaseError(e.to_string()))?;

                        let mut result = TeamEntity::find_by_id(team_id)
                            .find_with_related(TeamGroupEntity)
                            .all(transaction)
                            .await
                            .map_err(|e| CreateTeamError::DatabaseError(e.to_string()))?;

                        if result.len() != 1 {
                            return Err(CreateTeamError::DatabaseError(format!(
                                "Did not select one team/tags pair. Got: {result:?}"
                            )));
                        }

                        result.remove(0)
                    } else {
                        (
                            team.try_into_model()
                                .map_err(|e| CreateTeamError::DatabaseError(e.to_string()))?,
                            vec![],
                        )
                    };

                    Ok(TeamExtension { team, tags })
                })
            })
            .await
            .map_err(|e| match e {
                TransactionError::Connection(db) => CreateTeamError::DatabaseError(db.to_string()),
                TransactionError::Transaction(t) => t,
            })
    }

    pub async fn get_teams(&self, region_id: i32) -> Result<Vec<Team>> {
        let region = RegionEntity::find_by_id(region_id)
            .one(&self.connection)
            .await?
            .context("not found")?;

        region
            .find_related(TeamEntity)
            .all(&self.connection)
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn get_teams_with_tags(&self, region_id: i32) -> Result<Vec<TeamExtension>> {
        let region = RegionEntity::find_by_id(region_id)
            .one(&self.connection)
            .await?
            .context("not found")?;

        Ok(region
            .find_related(TeamEntity)
            .find_with_related(TeamGroupEntity)
            .all(&self.connection)
            .await
            .map_err(|e| anyhow!(e))?
            .into_iter()
            .map(|(team, tags)| TeamExtension::new(team, tags))
            .collect())
    }

    async fn decrement_group_count<V, I>(
        connection: &impl ConnectionTrait,
        ids: I,
        n: i32,
    ) -> Result<UpdateResult, DbErr>
    where
        V: Into<Value>,
        I: IntoIterator<Item = V>,
    {
        TeamGroupEntity::update_many()
            .filter(team_group::Column::Id.is_in(ids))
            .col_expr(
                team_group::Column::Usages,
                Expr::sub(Expr::col(team_group::Column::Usages), n),
            )
            .exec(connection)
            .await
    }

    pub async fn delete_team(&self, id: i32) -> Result<DeleteResult, TransactionError<DbErr>> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    // SQLite does not universally support `JOIN` statements in updates.
                    let ids_to_decrement = team_group_join::Entity::find()
                        .filter(team_group_join::Column::Team.eq(id))
                        .all(transaction)
                        .await?
                        .iter()
                        .map(|jt| jt.group)
                        .collect::<Vec<_>>();

                    Self::decrement_group_count(transaction, ids_to_decrement, 1).await?;

                    TeamEntity::delete(ActiveTeam {
                        id: Set(id),
                        ..Default::default()
                    })
                    .exec(transaction)
                    .await
                })
            })
            .await
    }

    pub async fn get_groups(&self) -> DBResult<Vec<TeamGroup>> {
        TeamGroupEntity.select().all(&self.connection).await
    }

    pub async fn create_group(&self, tag: String) -> Result<TeamGroup, CreateGroupError> {
        let all_groups = self
            .get_groups()
            .await
            .map_err(|e| CreateGroupError::DatabaseError(e.to_string()))?;

        if all_groups.iter().any(|x| x.name.eq_ignore_ascii_case(&tag)) {
            return Err(CreateGroupError::DuplicateTag);
        }

        TeamGroupEntity::insert(ActiveTeamGroup {
            name: Set(tag),
            ..Default::default()
        })
        .exec_with_returning(&self.connection)
        .await
        .map_err(|e| CreateGroupError::DatabaseError(e.to_string()))
    }

    pub async fn delete_group(&self, id: i32) -> DBResult<DeleteResult> {
        TeamGroupEntity::delete_by_id(id)
            .exec(&self.connection)
            .await
    }

    pub async fn get_time_slots(&self, field_id: i32) -> Result<Vec<TimeSlotExtension>, DbErr> {
        select_time_slot_extension()
            .filter(Condition::all().add(field::Column::Id.eq(field_id)))
            .into_model::<TimeSlotSelectionTypeAggregate>()
            .all(&self.connection)
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    async fn conflicts(
        connection: &impl ConnectionTrait,
        field_id: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        exclude_from_conflicts: Option<impl AsRef<[i32]>>,
    ) -> Result<(), TimeSlotError> {
        Self::conflicts_generic(
            connection,
            field_id,
            ConflictTimeSlotSource::Field,
            start,
            end,
            exclude_from_conflicts,
        )
        .await
    }

    async fn conflicts_generic(
        connection: &impl ConnectionTrait,
        id: i32,
        search: ConflictTimeSlotSource,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        exclude_from_conflicts: Option<impl AsRef<[i32]>>,
    ) -> Result<(), TimeSlotError> {
        let mut condition = match search {
            ConflictTimeSlotSource::Field => {
                Condition::all().add(time_slot::Column::FieldId.eq(id))
            }
            ConflictTimeSlotSource::Schedule => {
                Condition::all().add(schedule_game::Column::ScheduleId.eq(id))
            }
        };

        if let Some(ids) = exclude_from_conflicts {
            match search {
                ConflictTimeSlotSource::Field => {
                    for id in ids.as_ref().iter() {
                        condition = condition.add(time_slot::Column::Id.ne(*id))
                    }
                }
                ConflictTimeSlotSource::Schedule => {
                    for id in ids.as_ref().iter() {
                        condition = condition.add(schedule_game::Column::Id.ne(*id))
                    }
                }
            }
        }

        #[derive(FromQueryResult)]
        struct StartEnd {
            start: String,
            end: String,
        }

        let time_slots = match search {
            ConflictTimeSlotSource::Field => TimeSlotEntity::find()
                .select_only()
                .column(time_slot::Column::Start)
                .column(time_slot::Column::End)
                .inner_join(FieldEntity)
                .filter(condition)
                .into_model::<StartEnd>()
                .all(connection)
                .await
                .map_err(|e| TimeSlotError::DatabaseError(e.to_string()))?,
            ConflictTimeSlotSource::Schedule => ScheduleGameEntity::find()
                .select_only()
                .column(schedule_game::Column::Start)
                .column(schedule_game::Column::End)
                .inner_join(ScheduleEntity)
                .filter(condition)
                .into_model::<StartEnd>()
                .all(connection)
                .await
                .map_err(|e| TimeSlotError::DatabaseError(e.to_string()))?,
        };

        for time_slot in time_slots {
            let o_start = DateTime::<Utc>::from_str(&time_slot.start) //, FMT)
                .map_err(|e| {
                    TimeSlotError::ParseError(format!("bad input: {e} (`{}`)", time_slot.start))
                })?
                .to_utc();
            let o_end = DateTime::<Utc>::from_str(&time_slot.end) //, FMT)
                .map_err(|e| {
                    TimeSlotError::ParseError(format!("bad input: {e} (`{}`)", time_slot.end))
                })?
                .to_utc();

            if o_start < end && o_end > start {
                return Err(TimeSlotError::Overlap { o_start, o_end });
            }
        }

        Ok(())
    }

    pub async fn create_time_slot(
        &self,
        input: CreateTimeSlotInput,
    ) -> Result<TimeSlotExtension, TimeSlotError> {
        /*
         * Potential denial of service if the database gets filled with lots
         * of time slots.
         */
        Self::conflicts(
            &self.connection,
            input.field_id,
            input.start,
            input.end,
            None::<&[i32; 0]>,
        )
        .await?;

        let Some(reservation_type) = ReservationTypeEntity::find_by_id(input.reservation_type_id)
            .one(&self.connection)
            .await
            .map_err(|e| TimeSlotError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?
        else {
            return Err(TimeSlotError::ReservationTypeDoesNotExist(
                input.reservation_type_id,
            ));
        };

        let has_custom_size = entity::reservation_type_field_size_join::Entity::find()
            .filter(
                entity::reservation_type_field_size_join::Column::Field
                    .eq(input.field_id)
                    .and(
                        entity::reservation_type_field_size_join::Column::ReservationType
                            .eq(input.reservation_type_id),
                    ),
            )
            .one(&self.connection)
            .await
            .map_err(|e| TimeSlotError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?;

        /*
         * No conflicts; good to go.
         */

        self.connection
            .transaction(move |connection| {
                Box::pin(async move {
                    let time_slot = TimeSlotEntity::insert(ActiveTimeSlot {
                        start: Set(input.start.to_utc().to_rfc3339()),
                        end: Set(input.end.to_utc().to_rfc3339()),
                        field_id: Set(input.field_id),
                        ..Default::default()
                    })
                    .exec_with_returning(connection)
                    .await?;

                    let new_join_table_record =
                        entity::reservation_type_time_slot_join::ActiveModel {
                            time_slot: Set(time_slot.id),
                            reservation_type: Set(input.reservation_type_id),
                        };

                    new_join_table_record.insert(connection).await?;

                    Ok(TimeSlotExtension {
                        time_slot,
                        reservation_type,
                        custom_matches: has_custom_size.map(|jt_record| jt_record.size),
                    })
                })
            })
            .await
            .map_err(|e: TransactionError<DbErr>| match e {
                TransactionError::Connection(db) => {
                    TimeSlotError::DatabaseError(format!("{db} {}:{}", line!(), column!()))
                }
                TransactionError::Transaction(transaction) => TimeSlotError::DatabaseError(
                    format!("transaction error: {transaction} {}:{}", line!(), column!()),
                ),
            })
    }

    pub async fn delete_time_slot(
        &self,
        id: i32,
        schedule_id: Option<i32>,
    ) -> Result<(), TransactionError<DbErr>> {
        self.connection
            .transaction(|connection| {
                Box::pin(async move {
                    if let Some(schedule_id) = schedule_id {
                        ScheduleGameEntity::delete(ActiveScheduleGame {
                            id: Set(id),
                            ..Default::default()
                        })
                        .exec(connection)
                        .await?;

                        ScheduleEntity::update(ActiveSchedule {
                            id: Set(schedule_id),
                            last_edit: Set(Utc::now().to_rfc3339()),
                            ..Default::default()
                        })
                        .exec(connection)
                        .await?;

                        Ok(())
                    } else {
                        TimeSlotEntity::delete(ActiveTimeSlot {
                            id: Set(id),
                            ..Default::default()
                        })
                        .exec(connection)
                        .await?;

                        entity::reservation_type_time_slot_join::Entity::delete_many()
                            .filter(
                                entity::reservation_type_time_slot_join::Column::TimeSlot.eq(id),
                            )
                            .exec(connection)
                            .await?;

                        Ok(())
                    }
                })
            })
            .await
            .map(|_| ())
    }

    pub async fn move_time_slot(&self, input: MoveTimeSlotInput) -> Result<(), TimeSlotError> {
        if let Some(field_id) = input.field_id {
            Self::conflicts(
                &self.connection,
                field_id,
                input.new_start,
                input.new_end,
                Some([input.id]),
            )
            .await?;

            TimeSlotEntity::update_many()
                .col_expr(
                    time_slot::Column::Start,
                    Expr::val(Value::String(Some(Box::new(input.new_start.to_rfc3339()))))
                        .into_simple_expr(),
                )
                .col_expr(
                    time_slot::Column::End,
                    Expr::val(Value::String(Some(Box::new(input.new_end.to_rfc3339()))))
                        .into_simple_expr(),
                )
                .filter(time_slot::Column::Id.eq(input.id))
                .exec(&self.connection)
                .await
                .map_err(|e| {
                    TimeSlotError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                })?;
        } else if let Some(schedule_id) = input.schedule_id {
            Self::conflicts_generic(
                &self.connection,
                schedule_id,
                ConflictTimeSlotSource::Schedule,
                input.new_start,
                input.new_end,
                Some([input.id]),
            )
            .await?;

            ScheduleGameEntity::update_many()
                .col_expr(
                    schedule_game::Column::Start,
                    Expr::val(Value::String(Some(Box::new(input.new_start.to_rfc3339()))))
                        .into_simple_expr(),
                )
                .col_expr(
                    schedule_game::Column::End,
                    Expr::val(Value::String(Some(Box::new(input.new_end.to_rfc3339()))))
                        .into_simple_expr(),
                )
                .filter(schedule_game::Column::Id.eq(input.id))
                .exec(&self.connection)
                .await
                .map_err(|e| {
                    TimeSlotError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                })?;

            ScheduleEntity::update(ActiveSchedule {
                id: Set(schedule_id),
                last_edit: Set(Utc::now().to_rfc3339()),
                ..Default::default()
            })
            .exec(&self.connection)
            .await
            .map_err(|e| TimeSlotError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?;
        } else {
            return Err(TimeSlotError::ParseError(
                "missing id argument for move".to_owned(),
            ));
        }

        Ok(())
    }

    pub async fn list_reservations_between(
        &self,
        input: ListReservationsBetweenInput,
    ) -> DBResult<Vec<TimeSlotExtension>> {
        select_time_slot_extension()
            .filter(time_slot::Column::Start.between(input.start, input.end))
            .into_model::<TimeSlotSelectionTypeAggregate>()
            .all(&self.connection)
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
    }

    pub async fn load_all_teams(&self) -> DBResult<Vec<TeamExtension>> {
        Ok(TeamEntity::find()
            .find_with_related(TeamGroupEntity)
            .all(&self.connection)
            .await?
            .into_iter()
            .map(|(team, tags)| TeamExtension::new(team, tags))
            .collect())
    }

    pub async fn edit_region(&self, input: EditRegionInput) -> Result<Region, EditRegionError> {
        input.validate()?;

        let region_to_update = RegionEntity::find_by_id(input.id)
            .one(&self.connection)
            .await
            .map_err(|e| EditRegionError::DatabaseError(e.to_string()))?
            .ok_or(EditRegionError::NotFound(input.id))?;

        let mut active_model: ActiveRegion = region_to_update.into();

        if let Some(name) = input.name {
            active_model.title = Set(name.0);
        }

        /*
         * Add more updated fields later!
         */

        active_model
            .update(&self.connection)
            .await
            .map_err(|e| EditRegionError::DatabaseError(e.to_string()))?;

        // It's okay to look up again because the value is hot and cached.
        Ok(RegionEntity::find_by_id(input.id)
            .one(&self.connection)
            .await
            .map_err(|e| EditRegionError::DatabaseError(e.to_string()))?
            .expect("this will never fail to select because we updated an existing record"))
    }

    pub async fn edit_team(&self, input: EditTeamInput) -> Result<TeamExtension, EditTeamError> {
        input.validate()?;

        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    let mut team_to_edit = TeamEntity::find_by_id(input.id)
                        .one(transaction)
                        .await
                        .map_err(|e| {
                            EditTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                        })?
                        .ok_or(EditTeamError::NotFound(input.id))?;

                    let tags = team_to_edit
                        .find_related(TeamGroupEntity)
                        .all(transaction)
                        .await
                        .map_err(|e| {
                            EditTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                        })?
                        .into_iter()
                        .collect::<Vec<_>>();

                    if let Some(new_tags) = input.tags {
                        let new_tags = HashSet::from_iter(&new_tags);

                        let hashset = tags.iter().map(|m| &m.name).collect::<HashSet<_>>();

                        let deleted = hashset.difference(&new_tags);

                        // make all the tags hot
                        let groups = TeamGroupEntity::find()
                            .filter(team_group::Column::Name.is_in(new_tags.iter().cloned()))
                            .all(transaction)
                            .await
                            .map_err(|e| {
                                EditTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                            })?;

                        // tags that were removed
                        TeamGroupEntity::update_many()
                            .filter(team_group::Column::Name.is_in(deleted.cloned()))
                            .col_expr(
                                team_group::Column::Usages,
                                Expr::sub(Expr::col(team_group::Column::Usages), 1),
                            )
                            .exec(transaction)
                            .await
                            .map_err(|e| {
                                EditTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                            })?;

                        // tags that were added
                        TeamGroupEntity::update_many()
                            .filter(
                                team_group::Column::Name
                                    .is_in(new_tags.difference(&hashset).cloned()),
                            )
                            .col_expr(
                                team_group::Column::Usages,
                                Expr::add(Expr::col(team_group::Column::Usages), 1),
                            )
                            .exec(transaction)
                            .await
                            .map_err(|e| {
                                EditTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                            })?;

                        team_group_join::Entity::delete_many()
                            .filter(team_group_join::Column::Team.eq(input.id))
                            .exec(transaction)
                            .await
                            .map_err(|e| {
                                EditTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                            })?;

                        let mut active_models = Vec::with_capacity(groups.len());

                        for group in groups {
                            active_models.push(team_group_join::ActiveModel {
                                group: Set(group.id),
                                team: Set(input.id),
                            });
                        }

                        if !active_models.is_empty() {
                            team_group_join::Entity::insert_many(active_models)
                                .exec(transaction)
                                .await
                                .map_err(|e| {
                                    EditTeamError::DatabaseError(format!(
                                        "{}:{} {e}",
                                        file!(),
                                        line!()
                                    ))
                                })?;
                        }
                    }

                    if let Some(new_name) = input.name {
                        if team_to_edit.name != new_name.0 {
                            let mut active_team: ActiveTeam = team_to_edit.into();

                            active_team.name = Set(new_name.0);

                            team_to_edit = active_team.update(transaction).await.map_err(|e| {
                                EditTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                            })?;
                        }
                    }

                    let final_tags = team_to_edit
                        .find_related(TeamGroupEntity)
                        .all(transaction)
                        .await
                        .map_err(|e| {
                            EditTeamError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                        })?;

                    Ok(TeamExtension {
                        team: team_to_edit,
                        tags: final_tags,
                    })
                })
            })
            .await
            .map_err(|e| match e {
                TransactionError::Connection(db) => {
                    EditTeamError::DatabaseError(format!("{}:{} {db}", file!(), line!()))
                }
                TransactionError::Transaction(t) => t,
            })
    }

    pub async fn create_target(&self) -> Result<TargetExtension, TransactionError<DbErr>> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    let target = TargetEntity::insert(ActiveTarget {
                        ..Default::default()
                    })
                    .exec_with_returning(transaction)
                    .await?;

                    TargetExtension::new(target, transaction).await
                })
            })
            .await
    }

    pub async fn target_group_op(
        &self,
        target_id: i32,
        group_id: i32,
        op: TargetOp,
    ) -> Result<TargetExtension, TargetOpError> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    let target = TargetEntity::find_by_id(target_id)
                        .one(transaction)
                        .await
                        .map_err(|e| {
                            TargetOpError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                        })?
                        .ok_or(TargetOpError::TargetNotFound(target_id))?;

                    let _group = TeamGroupEntity::find_by_id(group_id)
                        .one(transaction)
                        .await
                        .map_err(|e| {
                            TargetOpError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                        })?
                        .ok_or(TargetOpError::GroupNotFound(group_id))?;

                    /*
                     * Missing: checks on existing primary key in join table
                     * for creation, and absence in join table for deletion.
                     *
                     * Not a problem because we control inputs but should be
                     * fixed if this is deployed as a docker container.
                     */

                    match op {
                        TargetOp::Insert => {
                            target_group_join::Entity::insert(target_group_join::ActiveModel {
                                group: Set(group_id),
                                target: Set(target_id),
                            })
                            .exec(transaction)
                            .await
                            .map_err(|e| {
                                TargetOpError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                            })?;
                        }
                        TargetOp::Delete => {
                            target_group_join::Entity::delete(target_group_join::ActiveModel {
                                group: Set(group_id),
                                target: Set(target_id),
                            })
                            .exec(transaction)
                            .await
                            .map_err(|e| {
                                TargetOpError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                            })?;
                        }
                    }

                    TargetExtension::new(target, transaction)
                        .await
                        .map_err(|e| {
                            TargetOpError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                        })
                })
            })
            .await
            .map_err(|e| match e {
                TransactionError::Connection(db) => {
                    TargetOpError::DatabaseError(format!("{}:{} {db}", file!(), line!()))
                }
                TransactionError::Transaction(t) => t,
            })
    }

    pub async fn get_targets(&self) -> DBResult<Vec<TargetExtension>> {
        let targets = TargetEntity::find().all(&self.connection).await?;
        TargetExtension::many_new(targets, &self.connection).await
    }

    pub async fn delete_target(&self, id: i32) -> DBResult<()> {
        TargetEntity::delete_by_id(id)
            .exec(&self.connection)
            .await
            .map(|_| ())
    }

    pub async fn generate_pre_schedule_report(
        &self,
        input: PreScheduleReportInput,
    ) -> Result<PreScheduleReport, PreScheduleReportError> {
        PreScheduleReport::create(&self.connection, input).await
    }

    pub async fn create_reservation_type(
        &self,
        input: CreateReservationTypeInput,
    ) -> Result<ReservationType, CreateReservationTypeError> {
        input.validate()?;
        ReservationTypeEntity::insert(ActiveReservationType {
            name: Set(input.name.0),
            description: Set(input.description),
            color: Set(input.color.0),
            ..Default::default()
        })
        .exec_with_returning(&self.connection)
        .await
        .map_err(|e| CreateReservationTypeError::DatabaseError(e.to_string()))
    }

    pub async fn get_reservation_types(
        &self,
        ids: Option<Vec<i32>>,
    ) -> DBResult<Vec<ReservationType>> {
        let ids = ids.unwrap_or_default();
        if ids.is_empty() {
            ReservationTypeEntity::find()
        } else {
            ReservationTypeEntity::find()
                .filter(Condition::all().add(reservation_type::Column::Id.is_in(ids)))
        }
        .all(&self.connection)
        .await
    }

    pub async fn delete_reservation_type(&self, id: i32) -> Result<(), String> {
        /*
         * In SQLite, you cannot join on a DELETE.
         * We must search by ID using a JOIN, which
         * will cache the result and thus speed up the
         * subsequent DELETE.
         */

        let time_slots_to_delete = TimeSlotEntity::find()
            .join(
                JoinType::LeftJoin,
                time_slot::Relation::ReservationTypeTimeSlotJoin.def(),
            )
            .join(
                JoinType::LeftJoin,
                reservation_type_time_slot_join::Relation::ReservationType.def(),
            )
            .filter(reservation_type::Column::Id.eq(id))
            .all(&self.connection)
            .await
            .map_err(|e| format!("{e} {}:{}", line!(), column!()))?;

        let time_slot_ids = time_slots_to_delete
            .iter()
            .map(|t| t.id)
            .collect::<Vec<_>>();

        self.connection
            .transaction(|connection| {
                Box::pin(async move {
                    TimeSlotEntity::delete_many()
                        .filter(time_slot::Column::Id.is_in(time_slot_ids))
                        .exec(connection)
                        .await?;

                    ReservationTypeEntity::delete_by_id(id)
                        .exec(connection)
                        .await
                        .map(|_| ())
                })
            })
            .await
            .map_err(|e| match e {
                TransactionError::Connection(e) => format!("{e} {}:{}", line!(), column!()),
                TransactionError::Transaction(trans) => {
                    format!("transaction: {trans} {}:{}", line!(), column!())
                }
            })
    }

    pub async fn edit_reservation_type(
        &self,
        mut reservation_type: ReservationType,
    ) -> Result<(), CreateReservationTypeError> {
        let name_to_validate = NameMax64(reservation_type.name);
        name_to_validate.validate()?;
        reservation_type.name = name_to_validate.0;

        match reservation_type
            .into_active_model()
            .reset_all()
            .update(&self.connection)
            .await
        {
            Ok(..) => Ok(()),
            Err(e) => Err(CreateReservationTypeError::DatabaseError(e.to_string())),
        }
    }

    pub async fn get_supported_concurrency_for_field(
        &self,
        input: FieldSupportedConcurrencyInput,
    ) -> DBResult<Vec<FieldConcurrency>> {
        let mut result = HashMap::with_capacity(input.reservation_type_ids.len());

        let custom_associations = reservation_type_field_size_join::Entity::find()
            .filter(
                Condition::all()
                    .add(reservation_type_field_size_join::Column::Field.eq(input.field_id))
                    .add(
                        reservation_type_field_size_join::Column::ReservationType
                            .is_in(input.reservation_type_ids.clone()),
                    ),
            )
            .all(&self.connection)
            .await?;

        for custom_association in custom_associations {
            result.insert(
                custom_association.reservation_type,
                (custom_association.size, true),
            );
        }

        let ids_non_custom = input
            .reservation_type_ids
            .into_iter()
            .filter(|id| !result.contains_key(id))
            .collect::<Vec<_>>();

        let default_associations = ReservationTypeEntity::find()
            .filter(reservation_type::Column::Id.is_in(ids_non_custom))
            .all(&self.connection)
            .await?;

        for default_association in default_associations {
            result.insert(
                default_association.id,
                (default_association.default_sizing, false),
            );
        }

        Ok(result
            .into_iter()
            .map(
                |(reservation_type_id, (concurrency, is_custom))| FieldConcurrency {
                    reservation_type_id,
                    concurrency,
                    is_custom,
                    field_id: input.field_id,
                },
            )
            .collect())
    }

    pub async fn update_reservation_type_concurrency_for_field(
        &self,
        input: UpdateReservationTypeConcurrencyForFieldInput,
    ) -> DBResult<()> {
        /*
         * There is no UPSERT support in SeaORM :(
         * At least we are hot with caching.
         */

        let maybe_join_table_record = reservation_type_field_size_join::Entity::find()
            .filter(
                Condition::all()
                    .add(reservation_type_field_size_join::Column::Field.eq(input.field_id))
                    .add(
                        reservation_type_field_size_join::Column::ReservationType
                            .eq(input.reservation_type_id),
                    ),
            )
            .one(&self.connection)
            .await?;

        if let Some(join_table_record) = maybe_join_table_record {
            let mut to_update = join_table_record.into_active_model();

            to_update.set(
                reservation_type_field_size_join::Column::Size,
                input.new_concurrency.into(),
            );

            to_update.update(&self.connection).await?;
        } else {
            reservation_type_field_size_join::ActiveModel {
                field: Set(input.field_id),
                reservation_type: Set(input.reservation_type_id),
                size: Set(input.new_concurrency),
            }
            .insert(&self.connection)
            .await?;
        }

        Ok(())
    }

    pub async fn get_non_default_reservation_type_concurrency_associations(
        &self,
    ) -> DBResult<Vec<FieldConcurrency>> {
        reservation_type_field_size_join::Entity::find()
            .group_by(reservation_type_field_size_join::Column::ReservationType)
            .join(
                JoinType::LeftJoin,
                reservation_type_field_size_join::Relation::ReservationType.def(),
            )
            .having(
                reservation_type_field_size_join::Column::Size
                    .into_expr()
                    .not_equals(reservation_type::Column::DefaultSizing),
            )
            .all(&self.connection)
            .await
            .map(|m| {
                m.into_iter()
                    .map(|m| FieldConcurrency {
                        concurrency: m.size,
                        is_custom: true,
                        reservation_type_id: m.reservation_type,
                        field_id: m.field,
                    })
                    .collect::<Vec<_>>()
            })
    }

    pub async fn update_target_reservation_type(
        &self,
        input: UpdateTargetReservationTypeInput,
    ) -> DBResult<()> {
        ActiveTarget {
            id: Set(input.target_id),
            maybe_reservation_type: Set(input.new_reservation_type_id),
        }
        .update(&self.connection)
        .await
        .map(|_| ())
    }

    pub async fn get_scheduled_inputs(
        &self,
    ) -> Result<
        Vec<ScheduledInput<TeamExtension, TeamCollection, FieldExtension, CoachConflict>>,
        GetScheduledInputsError,
    > {
        let mut result = vec![];

        let reservation_types = ReservationTypeEntity::find()
            .all(&self.connection)
            .await
            .map_err(|e| {
                GetScheduledInputsError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;

        let targets = TargetExtension::many_new(
            TargetEntity::find()
                .all(&self.connection)
                .await
                .map_err(|e| {
                    GetScheduledInputsError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                })?,
            &self.connection,
        )
        .await
        .map_err(|e| {
            GetScheduledInputsError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
        })?;

        for (i, reservation_type) in reservation_types.into_iter().enumerate() {
            let field_id_with_time_slots = select_time_slot_extension()
                .filter(reservation_type::Column::Id.eq(reservation_type.id))
                .order_by(field::Column::Id, sea_orm::Order::Asc)
                .into_model::<TimeSlotSelectionTypeAggregate>()
                .all(&self.connection)
                .await
                .map_err(|e| {
                    GetScheduledInputsError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                })?
                .into_iter()
                .map(Into::<TimeSlotExtension>::into)
                .group_by(|time_slot_extension| time_slot_extension.time_slot.field_id);

            let fields = field_id_with_time_slots
                .into_iter()
                .map(|(field_id, time_slots)| FieldExtension {
                    field_id,
                    time_slots: time_slots.collect_vec(),
                })
                .collect_vec();

            let mut teams = vec![];

            let targets_for_this_reservation_type = targets.iter().filter(|target| {
                target
                    .target
                    .maybe_reservation_type
                    .is_some_and(|x| x == reservation_type.id)
            });

            for target in targets_for_this_reservation_type {
                let teams_for_target = TeamEntity::find()
                    .find_with_related(TeamGroupEntity)
                    .filter(team_group::Column::Id.is_in(target.groups.iter().map(|g| g.id)))
                    .group_by(team::Column::Id)
                    .having(
                        team_group::Column::Id
                            .into_expr()
                            .count_distinct()
                            .eq(i32::try_from(target.groups.len()).unwrap()),
                    )
                    .all(&self.connection)
                    .await
                    .map_err(|e| {
                        GetScheduledInputsError::DatabaseError(format!(
                            "{e} {}:{}",
                            line!(),
                            column!()
                        ))
                    })?
                    .into_iter()
                    .map(|(team, tags)| TeamExtension::new(team, tags))
                    .collect_vec();

                teams.push(TeamCollection::new(target.groups.clone(), teams_for_target));
            }

            let unique_teams = teams
                .iter()
                .flat_map(|team_collection| &team_collection.teams)
                .unique_by(|team_ext| team_ext.team.id)
                .map(|team_ext| team_ext.team.id);

            let coach_conflicts_to_keep_in_mind = CoachConflictEntity::find()
                .find_with_related(TeamEntity)
                .filter(team::Column::Id.is_in(unique_teams))
                .all(&self.connection)
                .await
                .map_err(|e| {
                    GetScheduledInputsError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                })?
                .into_iter()
                .map(|(coach_conflict, teams)| CoachConflict {
                    coach_name: coach_conflict.coach_name,
                    id: coach_conflict.id,
                    region: coach_conflict.region,
                    teams,
                })
                .collect_vec();

            if reservation_type.is_practice {
                result.push(ScheduledInput::new_practice(
                    i.try_into().unwrap(),
                    teams,
                    fields,
                    coach_conflicts_to_keep_in_mind,
                ));
            } else {
                result.push(ScheduledInput::new(
                    i.try_into().unwrap(),
                    teams,
                    fields,
                    coach_conflicts_to_keep_in_mind,
                ));
            }
        }

        Ok(result)
    }

    pub fn generate_schedule_name() -> String {
        const ADJECTIVES: [&str; 12] = [
            "Funky",
            "Rambunctious",
            "Awesome",
            "Splendid",
            "Tubular",
            "Wonderful",
            "Radical",
            "Great",
            "Stupendous",
            "Remarkable",
            "Fashionable",
            "Elegant",
        ];

        use rand::seq::SliceRandom;
        let random_adjective = ADJECTIVES.choose(&mut rand::thread_rng()).unwrap();
        format!("New {random_adjective} Schedule")
    }

    pub async fn save_schedule(
        &self,
        schedule: CompiledSchedule,
    ) -> Result<Schedule, SaveScheduleError> {
        let mut active_games = vec![];

        for output in schedule.outputs {
            for reservation in output.time_slots {
                active_games.push(ActiveScheduleGame {
                    start: Set(DateTime::from_timestamp(reservation.start, 0)
                        .ok_or(SaveScheduleError::InvalidDateError(0))?
                        .to_rfc3339()),
                    end: Set(DateTime::from_timestamp(reservation.end, 0)
                        .ok_or(SaveScheduleError::InvalidDateError(1))?
                        .to_rfc3339()),
                    team_one: Set(reservation
                        .booking
                        .as_ref()
                        .and_then(|b| b.home_team.as_ref().map(TeamLike::unique_id))),
                    team_two: Set(reservation
                        .booking
                        .as_ref()
                        .and_then(|b| b.away_team.as_ref().map(TeamLike::unique_id))),
                    field_id: Set(reservation
                        .field
                        .as_ref()
                        .map(FieldLike::unique_id)
                        .and_then(|id| {
                            dbg!(id)
                                .try_into()
                                .inspect_err(|e| {
                                    eprintln!("field id overflow: {e:?}");
                                })
                                .ok()
                        })
                        .ok_or_else(|| SaveScheduleError::OverflowError("field id".into()))?),
                    ..Default::default()
                });
            }
        }

        self.connection
            .transaction(|connection| {
                Box::pin(async move {
                    let now = Local::now().to_rfc3339();

                    let new_schedule = ActiveSchedule {
                        name: Set(Self::generate_schedule_name()),
                        created: Set(now.clone()),
                        last_edit: Set(now),
                        ..Default::default()
                    }
                    .insert(connection)
                    .await?;

                    for game in &mut active_games {
                        game.schedule_id = Set(new_schedule.id);
                    }

                    ScheduleGameEntity::insert_many(active_games)
                        .exec(connection)
                        .await?;

                    Ok(new_schedule)
                })
            })
            .await
            .map_err(|e: TransactionError<DbErr>| match e {
                TransactionError::Connection(e) => {
                    SaveScheduleError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                }
                TransactionError::Transaction(e) => SaveScheduleError::DatabaseError(format!(
                    "transaction failed: {e} {}:{}",
                    line!(),
                    column!()
                )),
            })
    }

    pub async fn get_schedules(&self) -> DBResult<Vec<Schedule>> {
        ScheduleEntity::find()
            .order_by(schedule::Column::LastEdit, sea_orm::Order::Desc)
            .all(&self.connection)
            .await
    }

    pub async fn delete_schedule(&self, id: i32) -> DBResult<()> {
        ScheduleEntity::delete_by_id(id)
            .exec(&self.connection)
            .await
            .map(|_| ())
    }

    pub async fn edit_schedule(
        &self,
        input: EditScheduleInput,
    ) -> Result<Schedule, EditScheduleError> {
        input.validate()?;

        if let Some(name) = input.name {
            ActiveSchedule {
                id: Set(input.id),
                name: Set(name.0),
                last_edit: Set(Utc::now().to_rfc3339()),
                ..Default::default()
            }
            .update(&self.connection)
            .await
            .map_err(|e| {
                EditScheduleError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;
        }

        ScheduleEntity::find_by_id(input.id)
            .one(&self.connection)
            .await
            .map_err(|e| {
                EditScheduleError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?
            .ok_or(EditScheduleError::NotFound(input.id))
    }

    pub async fn get_schedule(&self, id: i32) -> Result<Schedule, LoadScheduleError> {
        ScheduleEntity::find_by_id(id)
            .one(&self.connection)
            .await
            .map_err(|e| {
                LoadScheduleError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?
            .ok_or(LoadScheduleError::NotFound(id))
    }

    pub async fn get_schedule_games(
        &self,
        schedule_id: i32,
    ) -> anyhow::Result<(Schedule, Vec<ScheduleGame>)> {
        let mut results = ScheduleEntity::find_by_id(schedule_id)
            .find_with_related(ScheduleGameEntity)
            .all(&self.connection)
            .await
            .context("could execute database query")?;

        if results.len() != 1 {
            bail!("missing schedule with id {schedule_id}");
        }

        Ok(results.remove(0))
    }

    pub async fn get_team(&self, team_id: i32) -> Result<TeamExtension, LoadTeamsError> {
        let mut teams_with_id = TeamEntity::find_by_id(team_id)
            .find_with_related(TeamGroupEntity)
            .all(&self.connection)
            .await
            .map_err(|e| LoadTeamsError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?
            .into_iter()
            .map(|(team, tags)| TeamExtension::new(team, tags))
            .collect_vec();

        if teams_with_id.len() != 1 {
            return Err(LoadTeamsError::NotFound(team_id, teams_with_id.len()));
        }

        Ok(teams_with_id.remove(0))
    }

    pub async fn copy_time_slots(
        &self,
        input: CopyTimeSlotsInput,
    ) -> Result<Vec<TimeSlotExtension>, CopyTimeSlotsError> {
        let start = TimeSlotEntity::find_by_id(input.src_start_id)
            .one(&self.connection)
            .await
            .map_err(|e| CopyTimeSlotsError::DatabaseError(e.to_string()))?
            .ok_or(CopyTimeSlotsError::NotFound(input.src_start_id))?;

        let end = TimeSlotEntity::find_by_id(input.src_end_id)
            .one(&self.connection)
            .await
            .map_err(|e| CopyTimeSlotsError::DatabaseError(e.to_string()))?
            .ok_or(CopyTimeSlotsError::NotFound(input.src_end_id))?;

        if start.field_id != end.field_id {
            return Err(CopyTimeSlotsError::FieldMismatch);
        }

        let first_time_chrono = DateTime::parse_from_rfc3339(&start.start)
            .expect("improperly stored date in database")
            .to_utc();

        let end_time_chrono = DateTime::parse_from_rfc3339(&end.start)
            .expect("improperly stored date in database")
            .to_utc();

        if first_time_chrono > end_time_chrono {
            return Err(CopyTimeSlotsError::OutOfOrder {
                start: first_time_chrono,
                end: end_time_chrono,
            });
        }

        let chrono_delta = input.dst_start - first_time_chrono;

        let src_time_slots = select_time_slot_extension()
            .filter(
                Condition::all()
                    .add(time_slot::Column::FieldId.eq(start.field_id))
                    .add(time_slot::Column::Start.between(start.start, end.start)),
            )
            .order_by(time_slot::Column::Start, Order::Asc)
            .into_model::<TimeSlotSelectionTypeAggregate>()
            .all(&self.connection)
            .await
            .map_err(|e| {
                CopyTimeSlotsError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?
            .into_iter()
            .map(Into::<TimeSlotExtension>::into)
            .collect_vec();

        let models_to_insert = src_time_slots
            .into_iter()
            .map(|time_slot_ext| {
                let start = DateTime::parse_from_rfc3339(&time_slot_ext.time_slot.start)
                    .expect("improperly stored date in database (start)")
                    .to_utc()
                    + chrono_delta;

                let end = DateTime::parse_from_rfc3339(&time_slot_ext.time_slot.end)
                    .expect("improperly stored date in database (end)")
                    .to_utc()
                    + chrono_delta;

                (
                    ActiveTimeSlot {
                        field_id: Set(time_slot_ext.time_slot.field_id),
                        start: Set(start.to_rfc3339()),
                        end: Set(end.to_rfc3339()),
                        ..Default::default()
                    },
                    start,
                    end,
                    time_slot_ext,
                )
            })
            .collect_vec();

        let Some(first) = models_to_insert.first() else {
            // there are no models to copy
            return Ok(vec![]);
        };

        let last = models_to_insert.last().unwrap();

        let potential_conflicts = TimeSlotEntity::find()
            .filter(
                Condition::all()
                    .add(time_slot::Column::FieldId.eq(start.field_id))
                    .add(time_slot::Column::End.gt(first.1.to_rfc3339()))
                    .add(time_slot::Column::Start.lt(last.2.to_rfc3339())),
            )
            .all(&self.connection)
            .await
            .map_err(|e| {
                CopyTimeSlotsError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;

        for existing_time_slot in &potential_conflicts {
            let o_start = DateTime::parse_from_rfc3339(&existing_time_slot.start)
                .unwrap()
                .to_utc();
            let o_end = DateTime::parse_from_rfc3339(&existing_time_slot.end)
                .unwrap()
                .to_utc();

            for (_, start, end, _) in &models_to_insert {
                if &o_start < end && &o_end > start {
                    return Err(CopyTimeSlotsError::Overlap { o_start, o_end });
                }
            }
        }

        let result = self
            .connection
            .transaction(|connection| {
                Box::pin(async move {
                    let mut ret_buf = Vec::with_capacity(models_to_insert.len());
                    for (model, .., time_slot_ext) in models_to_insert {
                        let new_time_slot = TimeSlotEntity::insert(model)
                            .exec_with_returning(connection)
                            .await?;

                        use reservation_type_time_slot_join::{
                            ActiveModel as RTTJM, Entity as RTTJE,
                        };

                        RTTJE::insert(RTTJM {
                            time_slot: Set(new_time_slot.id),
                            reservation_type: Set(time_slot_ext.reservation_type.id),
                        })
                        .exec_with_returning(connection)
                        .await?;

                        ret_buf.push(TimeSlotExtension {
                            custom_matches: time_slot_ext.custom_matches,
                            reservation_type: time_slot_ext.reservation_type,
                            time_slot: new_time_slot,
                        });
                    }

                    Ok(ret_buf)
                })
            })
            .await
            .map_err(|e: TransactionError<DbErr>| match e {
                TransactionError::Connection(db) => {
                    CopyTimeSlotsError::DatabaseError(format!("{db} {}:{}", line!(), column!()))
                }
                TransactionError::Transaction(t) => CopyTimeSlotsError::DatabaseError(format!(
                    "transaction failed: {t} {}:{}",
                    line!(),
                    column!()
                )),
            })?;

        Ok(result)
    }

    pub async fn delete_time_slots(
        &self,
        start_id: i32,
        end_id: i32,
    ) -> Result<(), DeleteTimeSlotsError> {
        let start = TimeSlotEntity::find_by_id(start_id)
            .one(&self.connection)
            .await
            .map_err(|e| DeleteTimeSlotsError::DatabaseError(e.to_string()))?
            .ok_or(DeleteTimeSlotsError::NotFound(start_id))?;

        let end = TimeSlotEntity::find_by_id(end_id)
            .one(&self.connection)
            .await
            .map_err(|e| DeleteTimeSlotsError::DatabaseError(e.to_string()))?
            .ok_or(DeleteTimeSlotsError::NotFound(end_id))?;

        if start.field_id != end.field_id {
            return Err(DeleteTimeSlotsError::FieldMismatch);
        }

        let first_time_chrono = DateTime::parse_from_rfc3339(&start.start)
            .expect("improperly stored date in database")
            .to_utc();

        let end_time_chrono = DateTime::parse_from_rfc3339(&end.start)
            .expect("improperly stored date in database")
            .to_utc();

        if first_time_chrono > end_time_chrono {
            return Err(DeleteTimeSlotsError::OutOfOrder {
                start: first_time_chrono,
                end: end_time_chrono,
            });
        }

        TimeSlotEntity::delete_many()
            .filter(
                Condition::all()
                    .add(time_slot::Column::FieldId.eq(start.field_id))
                    .add(time_slot::Column::Start.between(start.start, end.start)),
            )
            .exec(&self.connection)
            .await
            .map_err(|e| {
                DeleteTimeSlotsError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })
            .map(|_| ())
    }

    pub async fn create_coaching_conflict(
        &self,
        input: CreateCoachConflictInput,
    ) -> Result<CoachConflict, CoachConflictError> {
        let model = ActiveCoachConflict {
            coach_name: Set(input.coach_name),
            region: Set(input.region_id),
            ..Default::default()
        }
        .insert(&self.connection)
        .await
        .map_err(|e| CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?;

        Ok(CoachConflict {
            id: model.id,
            coach_name: model.coach_name,
            region: model.region,
            teams: vec![],
        })
    }

    pub async fn delete_coaching_conflict(&self, id: i32) -> Result<(), CoachConflictError> {
        let maybe_deleted = CoachConflictEntity::delete_by_id(id)
            .exec(&self.connection)
            .await
            .map_err(|e| {
                CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;

        if maybe_deleted.rows_affected != 1 {
            return Err(CoachConflictError::CoachConflictNotFound(id));
        }

        Ok(())
    }

    pub async fn coaching_conflict_team_op(
        &self,
        input: CoachConflictTeamInput,
    ) -> Result<(), CoachConflictError> {
        let coach_conflict = CoachConflictEntity::find_by_id(input.coach_conflict_id)
            .one(&self.connection)
            .await
            .map_err(|e| {
                CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?
            .ok_or(CoachConflictError::CoachConflictNotFound(
                input.coach_conflict_id,
            ))?;

        let team = TeamEntity::find_by_id(input.team_id)
            .one(&self.connection)
            .await
            .map_err(|e| {
                CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?
            .ok_or(CoachConflictError::TeamNotFound(input.coach_conflict_id))?;

        if coach_conflict.region != team.region_owner {
            return Err(CoachConflictError::RegionMismatch);
        }

        use coach_conflict_team_join as CCTJ;

        let join_table_record = CCTJ::Entity::find()
            .filter(
                Condition::all()
                    .add(CCTJ::Column::CoachConflict.eq(coach_conflict.id))
                    .add(CCTJ::Column::Team.eq(team.id)),
            )
            .one(&self.connection)
            .await
            .map_err(|e| {
                CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;

        match input.op {
            CoachConflictTeamInputOp::Create if join_table_record.is_none() => {
                CCTJ::ActiveModel {
                    coach_conflict: Set(coach_conflict.id),
                    team: Set(team.id),
                }
                .insert(&self.connection)
                .await
                .map_err(|e| {
                    CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                })?;
            }
            CoachConflictTeamInputOp::Delete if join_table_record.is_some() => {
                join_table_record
                    .unwrap()
                    .delete(&self.connection)
                    .await
                    .map_err(|e| {
                        CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                    })?;
            }
            _ => (),
        }

        Ok(())
    }

    pub async fn coaching_conflict_rename(
        &self,
        id: i32,
        new_name: String,
    ) -> Result<(), CoachConflictError> {
        let model = CoachConflictEntity::find_by_id(id)
            .one(&self.connection)
            .await
            .map_err(|e| {
                CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?
            .ok_or(CoachConflictError::CoachConflictNotFound(id))?;

        if model
            .coach_name
            .as_ref()
            .is_some_and(|name| name == &new_name)
        {
            return Ok(());
        }

        let mut active_model = model.into_active_model();

        active_model.set(coach_conflict::Column::CoachName, new_name.into());

        active_model.update(&self.connection).await.map_err(|e| {
            CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
        })?;

        Ok(())
    }

    pub async fn get_coach_conflicts(
        &self,
        region_id: i32,
    ) -> Result<Vec<CoachConflict>, CoachConflictError> {
        Ok(CoachConflictEntity::find()
            .find_with_related(TeamEntity)
            .filter(coach_conflict::Column::Region.eq(region_id))
            .all(&self.connection)
            .await
            .map_err(|e| {
                CoachConflictError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?
            .into_iter()
            .map(|(conflict, teams)| CoachConflict {
                id: conflict.id,
                region: conflict.region,
                coach_name: conflict.coach_name,
                teams,
            })
            .collect())
    }

    pub async fn get_region_metadata(
        &self,
        region_id: i32,
    ) -> Result<RegionMetadata, LoadRegionError> {
        let region = RegionEntity::find_by_id(region_id)
            .one(&self.connection)
            .await
            .map_err(|e| LoadRegionError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?
            .ok_or(LoadRegionError::NotFound(region_id))?;

        let team_count = TeamEntity::find()
            .filter(team::Column::RegionOwner.eq(region.id))
            .count(&self.connection)
            .await
            .map_err(|e| {
                LoadRegionError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;

        let field_count = FieldEntity::find()
            .filter(field::Column::RegionOwner.eq(region.id))
            .count(&self.connection)
            .await
            .map_err(|e| {
                LoadRegionError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;

        // the fields we need should be cached internally after this `COUNT(*)`

        let time_slot_count = TimeSlotEntity::find()
            .join(JoinType::LeftJoin, time_slot::Relation::Field.def())
            .filter(field::Column::RegionOwner.eq(region.id))
            .count(&self.connection)
            .await
            .map_err(|e| {
                LoadRegionError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;

        Ok(RegionMetadata {
            region_id,
            team_count,
            field_count,
            time_slot_count,
        })
    }

    pub async fn set_reservation_type_practice(
        &self,
        reservation_type_id: i32,
        is_practice: bool,
    ) -> DBResult<ReservationType> {
        ReservationTypeEntity::update(ActiveReservationType {
            id: Set(reservation_type_id),
            is_practice: Set(is_practice),
            ..Default::default()
        })
        .exec(&self.connection)
        .await
    }

    pub async fn swap_schedule_games(&self, a: i32, b: i32) -> Result<bool, TimeSlotError> {
        let game_one = ScheduleGameEntity::find_by_id(a)
            .one(&self.connection)
            .await
            .map_err(|e| TimeSlotError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?;

        let game_two = ScheduleGameEntity::find_by_id(b)
            .one(&self.connection)
            .await
            .map_err(|e| TimeSlotError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?;

        let (Some(game_one), Some(game_two)) = (game_one, game_two) else {
            return Ok(false);
        };

        if let Some(field_id) = game_one.field_id {
            Self::conflicts(
                &self.connection,
                field_id,
                DateTime::<Utc>::from_str(&game_two.start).unwrap(),
                DateTime::<Utc>::from_str(&game_two.end).unwrap(),
                Some([game_one.id, game_two.id]),
            )
            .await?;
        }

        if let Some(field_id) = game_two.field_id {
            Self::conflicts(
                &self.connection,
                field_id,
                DateTime::<Utc>::from_str(&game_one.start).unwrap(),
                DateTime::<Utc>::from_str(&game_one.end).unwrap(),
                Some([game_two.id, game_one.id]),
            )
            .await?;
        }

        let game_one_clone = game_one.clone();

        let mut game_one_am = game_one.into_active_model();

        game_one_am.start = Set(game_two.start.clone());
        game_one_am.end = Set(game_two.end.clone());
        game_one_am.field_id = Set(game_two.field_id);

        let mut game_two_am = game_two.into_active_model();

        game_two_am.start = Set(game_one_clone.start);
        game_two_am.end = Set(game_one_clone.end);
        game_one_am.field_id = Set(game_one_clone.field_id);

        self.connection
            .transaction(|connection| {
                Box::pin(async move {
                    game_one_am.update(connection).await?;
                    game_two_am.update(connection).await
                })
            })
            .await
            .map_err(|e| match e {
                TransactionError::Connection(db) | TransactionError::Transaction(db) => {
                    TimeSlotError::DatabaseError(format!("{db} {}:{}", line!(), column!()))
                }
            })?;

        Ok(true)
    }
}
