use sea_orm_migration::prelude::*;

use crate::m20240302_033333_create_time_slots::TimeSlot;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ReservationType::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ReservationType::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ReservationType::Name).string().not_null())
                    .col(ColumnDef::new(ReservationType::Description).string())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(ReservationTypeTimeSlotJoin::Table)
                    .col(
                        ColumnDef::new(ReservationTypeTimeSlotJoin::TimeSlot)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ReservationTypeTimeSlotJoin::ReservationType)
                            .integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ReservationTypeTimeSlotJoin::ReservationType)
                            .col(ReservationTypeTimeSlotJoin::TimeSlot),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_reservation_type")
                            .from(
                                ReservationTypeTimeSlotJoin::Table,
                                ReservationTypeTimeSlotJoin::ReservationType,
                            )
                            .to(ReservationType::Table, ReservationType::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_time_slot")
                            .from(
                                ReservationTypeTimeSlotJoin::Table,
                                ReservationTypeTimeSlotJoin::TimeSlot,
                            )
                            .to(TimeSlot::Table, TimeSlot::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(ReservationTypeTimeSlotJoin::Table)
                    .take(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(ReservationType::Table)
                    .take(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum ReservationType {
    Table,
    Id,
    Name,
    Description,
}

#[derive(DeriveIden)]
pub enum ReservationTypeTimeSlotJoin {
    Table,
    ReservationType,
    TimeSlot,
}
