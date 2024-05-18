use sea_orm_migration::prelude::*;

use crate::m20240223_053746_create_team::Team;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Schedule::Table)
                    .col(
                        ColumnDef::new(Schedule::Id)
                            .integer()
                            .primary_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Schedule::Name).string().not_null())
                    .col(
                        ColumnDef::new(Schedule::Created)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Schedule::LastEdit)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .take(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ScheduleGame::Table)
                    .col(
                        ColumnDef::new(ScheduleGame::Id)
                            .integer()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduleGame::ScheduleId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ScheduleGame::Start).timestamp().not_null())
                    .col(ColumnDef::new(ScheduleGame::End).timestamp().not_null())
                    .col(ColumnDef::new(ScheduleGame::TeamOne).integer())
                    .col(ColumnDef::new(ScheduleGame::TeamTwo).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .from(ScheduleGame::Table, ScheduleGame::ScheduleId)
                            .to(Schedule::Table, Schedule::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ScheduleGame::Table, ScheduleGame::TeamOne)
                            .to(Team::Table, Team::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ScheduleGame::Table, ScheduleGame::TeamTwo)
                            .to(Team::Table, Team::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .take(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ScheduleGame::Table).take())
            .await?;
        manager
            .drop_table(Table::drop().table(Schedule::Table).take())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Schedule {
    Table,
    Id,
    Name,
    Created,
    LastEdit,
}

#[derive(DeriveIden)]
pub(crate) enum ScheduleGame {
    Table,
    Id,
    ScheduleId,
    Start,
    End,
    TeamOne,
    TeamTwo,
}
