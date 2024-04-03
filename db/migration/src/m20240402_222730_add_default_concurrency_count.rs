use sea_orm_migration::prelude::*;

use crate::m20240331_003613_create_reservation_type::ReservationType;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ReservationType::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Alias::new("default_sizing"))
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .take(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
