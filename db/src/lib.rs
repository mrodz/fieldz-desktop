use std::collections::HashSet;

use anyhow::{anyhow, bail, Context, Result};
use entity::field::ActiveModel as ActiveField;
use entity::field::Entity as FieldEntity;
use entity::field::Model as Field;
use entity::region::ActiveModel as ActiveRegion;
use entity::region::Entity as RegionEntity;
use entity::region::Model as Region;
use entity::team::ActiveModel as ActiveTeam;
use entity::team::Entity as TeamEntity;
use entity::team::Model as Team;
use entity::team_group::ActiveModel as ActiveTeamGroup;
use entity::team_group::Entity as TeamGroupEntity;
use entity::team_group::Model as TeamGroup;
use migration::{Expr, Migrator, MigratorTrait};
use sea_orm::QueryOrder;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, JoinType, QueryFilter, QuerySelect,
    RelationTrait, Set, TransactionError, TransactionTrait, TryIntoModel, UpdateResult, Value,
};
use sea_orm::{Database, DatabaseConnection, EntityTrait};
pub use sea_orm::{DbErr, DeleteResult};
use sea_orm::{EntityOrSelect, ModelTrait};

pub use entity::*;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

pub type DBResult<T> = anyhow::Result<T, DbErr>;

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
pub struct CreateRegionInput {
    title: String,
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum RegionValidationError {
    #[error("region name cannot be empty")]
    EmptyName,
    #[error("region name is {len} characters which is larger than the max, 64")]
    NameTooLong { len: usize },
}

impl CreateRegionInput {
    pub fn validate(&self) -> Result<(), RegionValidationError> {
        let len = self.title.len();

        if self.title.is_empty() {
            return Err(RegionValidationError::EmptyName);
        }

        if len > 64 {
            return Err(RegionValidationError::NameTooLong { len });
        }

        // add more checks if the fields change...

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
pub struct CreateTeamInput {
    name: String,
    region_id: i32,
    tags: Vec<String>,
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
        let len = self.name.len();

        if self.name.is_empty() {
            return Err(TeamValidationError::EmptyName);
        }

        if len > 64 {
            return Err(TeamValidationError::NameTooLong { len });
        }

        // add more checks if the fields change...

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

impl Client {
    pub async fn new(config: &Config) -> Result<Self> {
        let db: DatabaseConnection = Database::connect(&config.connection_url).await?;

        if db.ping().await.is_err() {
            bail!("database did not respond to ping");
        }

        let result = Client { connection: db };

        result.refresh().await?;

        Ok(result)
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
            title: Set(input.title),
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
                        name: Set(input.name),
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

                    // let r = TeamGroupEntity::update_many()
                    // .col_expr(
                    //     team_group::Column::Usages,
                    //     Expr::sub(Expr::col(team_group::Column::Usages), 1),
                    // )
                    // .filter(
                    //     /*Expr::exists(
                    //         team_group_join::Entity::find()
                    //             .column(team_group_join::Column::Group)
                    //             .filter(team_group_join::Column::Team.eq(id))
                    //             .into_query(),
                    //     ),*/ Condition::all().add(
                    //            team_group::Column::Id
                    //                .in_subquery(
                    //                    team_group_join::Entity::find()
                    //                        .column(team_group_join::Column::Group)
                    //                        .filter(team_group_join::Column::Team.eq(id))
                    //                        .into_query(),
                    //                )
                    //        ),
                    // )

                    // .belongs_to()

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
}
