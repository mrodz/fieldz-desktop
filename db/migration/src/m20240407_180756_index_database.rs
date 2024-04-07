use sea_orm_migration::prelude::*;

use crate::{
    m20240302_033333_create_time_slots::TimeSlot,
    m20240331_003613_create_reservation_type::{
        ReservationType, ReservationTypeFieldSizeJoin, ReservationTypeTimeSlotJoin,
    },
};

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn up_time_slot<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("IX_TimeSlot_start")
                .table(TimeSlot::Table)
                .col(TimeSlot::Start)
                .take(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("IX_TimeSlot_end")
                .table(TimeSlot::Table)
                .col(TimeSlot::End)
                .take(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("IX_TimeSlot_field-id")
                .table(TimeSlot::Table)
                .col(TimeSlot::FieldId)
                .take(),
        )
        .await
}

async fn down_time_slot<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .drop_index(Index::drop().name("IX_TimeSlot_start").to_owned())
        .await?;

    manager
        .drop_index(Index::drop().name("IX_TimeSlot_end").to_owned())
        .await?;

    manager
        .drop_index(Index::drop().name("IX_TimeSlot_field-id").to_owned())
        .await
}

async fn up_reservation_type_time_slot_join<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("UX_ReservationTypeTimeSlotJoin_time-slot")
                .table(ReservationTypeTimeSlotJoin::Table)
                .col(ReservationTypeTimeSlotJoin::TimeSlot)
                .unique()
                .take(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("IX_ReservationTypeTimeSlotJoin_reservation-type")
                .table(ReservationTypeTimeSlotJoin::Table)
                .col(ReservationTypeTimeSlotJoin::ReservationType)
                .take(),
        )
        .await
}

async fn down_reservation_type_time_slot_join<'a>(
    manager: &SchemaManager<'a>,
) -> Result<(), DbErr> {
    manager
        .drop_index(
            Index::drop()
                .name("UX_ReservationTypeTimeSlotJoin_time-slot")
                .to_owned(),
        )
        .await?;

    manager
        .drop_index(
            Index::drop()
                .name("IX_ReservationTypeTimeSlotJoin_reservation-type")
                .to_owned(),
        )
        .await
}

async fn up_reservation_type<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("IX_ReservationType_default-sizing")
                .table(ReservationType::Table)
                .col(Alias::new("default_sizing"))
                .take(),
        )
        .await
}

async fn down_reservation_type<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .drop_index(
            Index::drop()
                .name("IX_ReservationType_default-sizing")
                .to_owned(),
        )
        .await
}

async fn up_reservation_type_field_size_join<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager
        .create_index(
            Index::create()
                .name("UX_ReservationTypeFieldSizeJoin_COMPOSITE_reservation-type-field")
                .table(ReservationTypeFieldSizeJoin::Table)
                .col(ReservationTypeFieldSizeJoin::Field)
                .col(ReservationTypeFieldSizeJoin::ReservationType)
                .take(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("IX_ReservationTypeFieldSizeJoin_reservation-type")
                .table(ReservationTypeFieldSizeJoin::Table)
                .col(ReservationTypeFieldSizeJoin::ReservationType)
                .take(),
        )
        .await
}

async fn down_reservation_type_field_size_join<'a>(
    manager: &SchemaManager<'a>,
) -> Result<(), DbErr> {
    manager
        .drop_index(
            Index::drop()
                .name("UX_ReservationTypeFieldSizeJoin_COMPOSITE_reservation-type-field")
                .to_owned(),
        )
        .await?;

    manager
        .drop_index(
            Index::drop()
                .name("IX_ReservationTypeFieldSizeJoin_reservation-type")
                .to_owned(),
        )
        .await
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        up_time_slot(manager).await?;
        up_reservation_type(manager).await?;
        up_reservation_type_time_slot_join(manager).await?;
        up_reservation_type_field_size_join(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        down_time_slot(manager).await?;
        down_reservation_type(manager).await?;
        down_reservation_type_time_slot_join(manager).await?;
        down_reservation_type_field_size_join(manager).await
    }
}
