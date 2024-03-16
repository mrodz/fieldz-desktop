pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_region;
mod m20240221_004555_create_field;
mod m20240223_053746_create_team;
mod m20240302_033333_create_time_slots;
mod m20240316_053808_create_target;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_region::Migration),
            Box::new(m20240221_004555_create_field::Migration),
            Box::new(m20240223_053746_create_team::Migration),
            Box::new(m20240302_033333_create_time_slots::Migration),
            Box::new(m20240316_053808_create_target::Migration),
        ]
    }
}
