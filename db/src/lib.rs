use anyhow::{bail, Result};
use entity::region::ActiveModel as ActiveRegion;
use entity::region::Entity as RegionEntity;
use entity::region::Model as Region;
use migration::MigratorTrait;
pub use sea_orm::{DbErr, DeleteResult};
use sea_orm::Set;
use sea_orm::{Database, DatabaseConnection, EntityTrait};

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

impl Client {
    pub async fn new(config: &Config) -> Result<Self> {
        let db: DatabaseConnection = Database::connect(&config.connection_url).await?;

        if db.ping().await.is_err() {
            bail!("database did not respond to ping");
        }

        migration::Migrator::up(&db, None).await?;

        Ok(Client { connection: db })
    }

    pub async fn get_regions(&self) -> DBResult<Vec<Region>> {
        RegionEntity::find().all(&self.connection).await
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

    pub async fn delete_region(&self, id: i32) -> DBResult<DeleteResult> {
        RegionEntity::delete(ActiveRegion {
            id: Set(id),
            ..Default::default()
        })
        .exec(&self.connection)
        .await
    }
}
