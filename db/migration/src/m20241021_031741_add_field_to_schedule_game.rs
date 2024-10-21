use sea_orm_migration::prelude::*;

use crate::m20240507_015703_create_schedule_results::ScheduleGame;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ScheduleGame::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Alias::new("field_id")).integer().null(),
                    )
                    .take(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
