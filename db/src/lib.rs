use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::num::NonZeroU8;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use chrono::{serde::ts_milliseconds, DateTime};
use entity::field::ActiveModel as ActiveField;
use entity::field::Entity as FieldEntity;
use entity::field::Model as Field;
use entity::region::ActiveModel as ActiveRegion;
use entity::region::Entity as RegionEntity;
use entity::region::Model as Region;
use entity::reservation_type::ActiveModel as ActiveReservationType;
use entity::reservation_type::Entity as ReservationTypeEntity;
use entity::reservation_type::Model as ReservationType;
use entity::target::ActiveModel as ActiveTarget;
use entity::target::Entity as TargetEntity;
use entity::target::Model as Target;
use entity::team::ActiveModel as ActiveTeam;
use entity::team::Entity as TeamEntity;
use entity::team::Model as Team;
use entity::team_group::ActiveModel as ActiveTeamGroup;
use entity::team_group::Entity as TeamGroupEntity;
use entity::team_group::Model as TeamGroup;
use entity::time_slot::ActiveModel as ActiveTimeSlot;
use entity::time_slot::Entity as TimeSlotEntity;
use entity::time_slot::Model as TimeSlot;
use migration::{Expr, Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, FromQueryResult, JoinType,
    PaginatorTrait, QueryFilter, QuerySelect, RelationTrait, Set, TransactionError,
    TransactionTrait, TryIntoModel, UpdateResult, Value,
};
use sea_orm::{Database, DatabaseConnection, EntityTrait};
pub use sea_orm::{DbErr, DeleteResult};
use sea_orm::{EntityOrSelect, ModelTrait};
use sea_orm::{IntoSimpleExpr, QueryOrder};

pub use entity::*;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

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

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum RegionNameValidationError {
    #[error("region name cannot be empty")]
    EmptyName,
    #[error("region name is {len} characters which is larger than the max, 64")]
    NameTooLong { len: usize },
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum RegionValidationError {
    #[error(transparent)]
    Name(#[from] RegionNameValidationError),
}

impl Validator for RegionName {
    type Error = RegionNameValidationError;
    fn validate(&self) -> Result<(), Self::Error> {
        let len = self.0.len();

        if self.0.is_empty() {
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

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum FieldValidationError {
    #[error("field name cannot be empty")]
    EmptyName,
    #[error("field name is {len} characters which is larger than the max, 64")]
    NameTooLong { len: usize },
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
struct TeamName(String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTeamInput {
    name: TeamName,
    region_id: i32,
    tags: Vec<String>,
}

impl Validator for TeamName {
    type Error = TeamValidationError;
    fn validate(&self) -> Result<(), Self::Error> {
        let len = self.0.len();

        if self.0.is_empty() {
            return Err(TeamValidationError::EmptyName);
        }

        if len > 64 {
            return Err(TeamValidationError::NameTooLong { len });
        }

        Ok(())
    }
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum TeamValidationError {
    #[error("field name cannot be empty")]
    EmptyName,
    #[error("field name is {len} characters which is larger than the max, 64")]
    NameTooLong { len: usize },
}

impl CreateTeamInput {
    pub fn validate(&self) -> Result<(), TeamValidationError> {
        self.name.validate()?;

        Ok(())
    }
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CreateGroupError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("this tag already exists")]
    DuplicateTag,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CreateTeamError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("bad input")]
    ValidationError(TeamValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("the following tags do not exist: {0:?}")]
    MissingTags(Vec<String>),
    #[error("the transaction to create a team failed")]
    TransactionError,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamExtension {
    team: Team,
    tags: Vec<TeamGroup>,
}

impl TeamExtension {
    pub const fn new(team: Team, tags: Vec<TeamGroup>) -> Self {
        Self { tags, team }
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

#[derive(Error, Debug, Serialize, Deserialize)]

pub enum TimeSlotError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("this time slot is booked from {o_start} to {o_end}")]
    Overlap {
        #[serde(with = "ts_milliseconds")]
        o_start: DateTime<Utc>,
        #[serde(with = "ts_milliseconds")]
        o_end: DateTime<Utc>,
    },
    #[error("the supplied reservation type id ({0}) does not exist")]
    ReservationTypeDoesNotExist(i32),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("could not parse date: `{0}`")]
    ParseError(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSlotExtension {
    time_slot: TimeSlot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveTimeSlotInput {
    field_id: i32,
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum EditRegionError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error(transparent)]
    Name(#[from] RegionNameValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("region with id {0} not found")]
    NotFound(i32),
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
    name: Option<TeamName>,
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

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum EditTeamError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("bad input")]
    ValidationError(TeamValidationError),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("the following tags do not exist: {0:?}")]
    MissingTags(Vec<String>),
    #[error("the transaction to create a team failed")]
    TransactionError,
    #[error("team with id {0} not found")]
    NotFound(i32),
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetOp {
    Insert,
    Delete,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum TargetOpError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("group with id {0} not found")]
    GroupNotFound(i32),
    #[error("team with id {0} not found")]
    TargetNotFound(i32),
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
    #[error("the transaction to create a team failed")]
    TransactionError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateEntry {
    team_groups: Vec<TeamGroup>,
    used_by: Vec<TargetExtension>,
    teams_with_group_set: u64,
}

impl DuplicateEntry {
    pub fn has_duplicates(&self) -> bool {
        self.used_by.len() > 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreScheduleReport {
    target_duplicates: Vec<DuplicateEntry>,
    target_has_duplicates: Vec<usize>,
    target_required_matches: Vec<(TargetExtension, u64)>,
    total_matches_required: u64,
    total_matches_supplied: u64,
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum PreScheduleReportError {
    #[error("database was not initialized")]
    NoDatabase,
    #[error("database operation failed: `{0}`")]
    DatabaseError(String),
}

fn ncr(n: u64, r: u64) -> u64 {
    fn factorial(num: u64) -> u64 {
        let mut f = 1;

        for i in 1..=num {
            f *= i;
        }

        f
    }

    if r > n {
        0
    } else {
        factorial(n) / (factorial(r) * factorial(n - r))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreScheduleReportInput {
    matches_to_play: NonZeroU8,
    total_matches_supplied: Option<u64>,
}

impl PreScheduleReport {
    pub fn new(target_duplicates: Vec<DuplicateEntry>, input: PreScheduleReportInput) -> Self {
        let target_has_duplicates = target_duplicates
            .iter()
            .filter(|d| d.has_duplicates())
            .flat_map(|d| &d.used_by)
            .map(|target| target.target.id.try_into().unwrap())
            .collect();

        let mut target_required_matches: BTreeMap<TargetExtension, u64> = BTreeMap::new();

        let mut total_matches_required = 0;

        for entry in &target_duplicates {
            let choices = ncr(entry.teams_with_group_set, 2);
            total_matches_required += choices;
            for target in &entry.used_by {
                let sum = target_required_matches.entry(target.clone()).or_default();
                *sum += choices;
            }
        }

        for m in &mut target_required_matches {
            *m.1 *= input.matches_to_play.get() as u64;
        }

        Self {
            target_duplicates,
            target_has_duplicates,
            target_required_matches: target_required_matches.into_iter().collect(),
            total_matches_required: total_matches_required * input.matches_to_play.get() as u64,
            total_matches_supplied: input.total_matches_supplied.unwrap(),
        }
    }

    pub async fn create<C>(
        connection: &C,
        mut input: PreScheduleReportInput,
    ) -> Result<Self, PreScheduleReportError>
    where
        C: ConnectionTrait,
    {
        let all_targets = TargetEntity::find().all(connection).await.map_err(|e| {
            PreScheduleReportError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
        })?;

        let all_targets_extended = TargetExtension::many_new(&all_targets, connection)
            .await
            .map_err(|e| {
                PreScheduleReportError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
            })?;

        let mut collision_map: BTreeMap<BTreeSet<&TeamGroup>, Vec<&TargetExtension>> =
            BTreeMap::new();

        for target in &all_targets_extended {
            let set_of_groups = BTreeSet::from_iter(&target.groups);
            let entry = collision_map.entry(set_of_groups).or_default();
            entry.push(target);
        }

        let mut target_duplicates = Vec::with_capacity(collision_map.len());

        for (groups, targets) in collision_map {
            let query = TeamEntity::find()
                .join(JoinType::LeftJoin, team::Relation::TeamGroupJoin.def())
                .join(
                    JoinType::LeftJoin,
                    team_group_join::Relation::TeamGroup.def(),
                )
                .filter(team_group::Column::Id.is_in(groups.iter().map(|g| g.id)))
                .group_by(team::Column::Id)
                .having(
                    team_group::Column::Id
                        .into_expr()
                        .count_distinct()
                        .eq(groups.len() as i32),
                );

            let teams_with_group_set = query.count(connection).await.map_err(|e| {
                PreScheduleReportError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
            })?;

            target_duplicates.push(DuplicateEntry {
                team_groups: groups.into_iter().cloned().collect(),
                used_by: targets.into_iter().cloned().collect(),
                teams_with_group_set,
            })
        }

        if input.total_matches_supplied.is_none() {
            input.total_matches_supplied = Some(
                TimeSlotEntity::find()
                    .count(connection)
                    .await
                    .map_err(|e| {
                        PreScheduleReportError::DatabaseError(format!(
                            "{}:{} {e}",
                            file!(),
                            line!()
                        ))
                    })?,
            );
        };

        Ok(Self::new(target_duplicates, input))
    }
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

    pub async fn get_time_slots(&self, field_id: i32) -> Result<Vec<TimeSlot>, DbErr> {
        TimeSlotEntity::find()
            .join(JoinType::LeftJoin, time_slot::Relation::Field.def())
            .filter(Condition::all().add(field::Column::Id.eq(field_id)))
            .all(&self.connection)
            .await
    }

    async fn conflicts(
        connection: &impl ConnectionTrait,
        field_id: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        exclude_from_conflicts: Option<i32>,
    ) -> Result<(), TimeSlotError> {
        let mut condition = Condition::all().add(time_slot::Column::FieldId.eq(field_id));

        if let Some(id) = exclude_from_conflicts {
            condition = condition.add(time_slot::Column::Id.ne(id))
        }

        let time_slots = TimeSlotEntity::find()
            .inner_join(FieldEntity)
            .filter(condition)
            .all(connection)
            .await
            .map_err(|e| TimeSlotError::DatabaseError(e.to_string()))?;

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
    ) -> Result<TimeSlot, TimeSlotError> {
        /*
         * Potential denial of service if the database gets filled with lots
         * of time slots.
         */
        Self::conflicts(
            &self.connection,
            input.field_id,
            input.start,
            input.end,
            None,
        )
        .await?;

        if ReservationTypeEntity::find_by_id(input.reservation_type_id)
            .one(&self.connection)
            .await
            .map_err(|e| TimeSlotError::DatabaseError(format!("{e} {}:{}", line!(), column!())))?
            .is_none()
        {
            return Err(TimeSlotError::ReservationTypeDoesNotExist(
                input.reservation_type_id,
            ));
        }

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

                    Ok(time_slot)
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

    pub async fn delete_time_slot(&self, id: i32) -> DBResult<DeleteResult> {
        TimeSlotEntity::delete(ActiveTimeSlot {
            id: Set(id),
            ..Default::default()
        })
        .exec(&self.connection)
        .await
    }

    pub async fn move_time_slot(&self, input: MoveTimeSlotInput) -> Result<(), TimeSlotError> {
        Self::conflicts(
            &self.connection,
            input.field_id,
            input.new_start,
            input.new_end,
            Some(input.id),
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
            .map_err(|e| TimeSlotError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn list_reservations_between(
        &self,
        input: ListReservationsBetweenInput,
    ) -> DBResult<Vec<TimeSlot>> {
        TimeSlotEntity::find()
            .filter(time_slot::Column::Start.between(input.start, input.end))
            .all(&self.connection)
            .await
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
                     * for creation, and absense in join table for deletion.
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
}
