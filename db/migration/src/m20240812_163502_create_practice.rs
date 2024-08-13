use sea_orm_migration::prelude::*;

use crate::m20240331_003613_create_reservation_type::ReservationType;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
                .table(ReservationType::Table)
                .add_column_if_not_exists(
                    ColumnDef::new(Alias::new("is_practice"))
                        .boolean()
                        .not_null()
                        .check(Expr::col(Alias::new("is_practice")).is_in([0, 1]))
                        .default(Value::Int(Some(0)))
                )
                .take(),
        ).await
    }
}
