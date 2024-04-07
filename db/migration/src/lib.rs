pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_region;
mod m20240221_004555_create_field;
mod m20240223_053746_create_team;
mod m20240302_033333_create_time_slots;
mod m20240316_053808_create_target;
mod m20240331_003613_create_reservation_type;
mod m20240402_222730_add_default_concurrency_count;
mod m20240407_180756_index_database;

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
            Box::new(m20240331_003613_create_reservation_type::Migration),
            Box::new(m20240402_222730_add_default_concurrency_count::Migration),
            Box::new(m20240407_180756_index_database::Migration),
        ]
    }
}
