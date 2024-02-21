pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_region;
mod m20240221_004555_create_field;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_region::Migration),
            Box::new(m20240221_004555_create_field::Migration),
        ]
    }
}
