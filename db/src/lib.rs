use anyhow::{bail, Result};
use entity::region::ActiveModel as ActiveRegion;
use entity::region::Entity as RegionEntity;
use entity::region::Model as Region;
use migration::MigratorTrait;
use sea_orm::DbErr;
use sea_orm::DeleteResult;
use sea_orm::Set;
use sea_orm::{Database, DatabaseConnection, EntityTrait};

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

pub struct Client {
    connection: DatabaseConnection,
}

impl Client {
    pub async fn get_regions(&self) -> Result<Vec<Region>, DbErr> {
        RegionEntity::find().all(&self.connection).await
    }
    pub async fn create_region(&self, title: String) -> Result<Region, DbErr> {
        RegionEntity::insert(ActiveRegion {
            title: Set(title),
            ..Default::default()
        })
        .exec_with_returning(&self.connection)
        .await
    }
    pub async fn delete_regions(&self) -> Result<DeleteResult, DbErr> {
        RegionEntity::delete_many().exec(&self.connection).await
    }
}

pub async fn connect(config: &Config) -> Result<Client> {
    let db: DatabaseConnection = Database::connect(&config.connection_url).await?;

    if db.ping().await.is_err() {
        bail!("database did not respond to ping");
    }

    migration::Migrator::up(&db, None).await?;

    Ok(Client { connection: db })
}
