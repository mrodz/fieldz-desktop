use sea_orm_migration::prelude::*;

use crate::m20240221_004555_create_field::Field;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TimeSlot::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TimeSlot::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TimeSlot::FieldId).integer().not_null())
                    .col(ColumnDef::new(TimeSlot::Start).date_time().not_null())
                    .col(ColumnDef::new(TimeSlot::End).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_time_slot_field")
                            .from(TimeSlot::Table, TimeSlot::FieldId)
                            .to(Field::Table, Field::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TimeSlot::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum TimeSlot {
    Table,
    Id,
    FieldId,
    Start,
    End,
}
