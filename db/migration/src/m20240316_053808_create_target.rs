use sea_orm_migration::prelude::*;

use crate::{
    m20240223_053746_create_team::*, m20240331_003613_create_reservation_type::ReservationType,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Target::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Target::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Target::MaybeReservationType).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_maybe_reservation_type")
                            .from(Target::Table, Target::MaybeReservationType)
                            .to(ReservationType::Table, ReservationType::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TargetGroupJoin::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TargetGroupJoin::Target).integer().not_null())
                    .col(ColumnDef::new(TargetGroupJoin::Group).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(TargetGroupJoin::Target)
                            .col(TargetGroupJoin::Group),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_group")
                            .from(TargetGroupJoin::Table, TargetGroupJoin::Group)
                            .to(TeamGroup::Table, TeamGroup::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_target")
                            .from(TargetGroupJoin::Table, TargetGroupJoin::Target)
                            .to(Target::Table, Target::Id)
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
            .drop_table(Table::drop().table(TargetGroupJoin::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Target::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Target {
    Table,
    Id,
    MaybeReservationType,
}

#[derive(DeriveIden)]
pub(crate) enum TargetGroupJoin {
    Table,
    Target,
    Group,
}
