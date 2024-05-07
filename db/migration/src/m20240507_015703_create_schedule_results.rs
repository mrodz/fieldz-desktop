use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(Schedule::Table)
                    .col(ColumnDef::new(Schedule::Id).integer().primary_key())
                    .col(ColumnDef::new(Schedule::Name).string().not_null())
                    .col(ColumnDef::new(Schedule::Created).timestamp().not_null())
                    .take(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ScheduleGameGroup::Table)
                    .col(
                        ColumnDef::new(ScheduleGameGroup::Id)
                            .integer()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ScheduleGameGroup::Name).string().not_null())
                    .col(
                        ColumnDef::new(ScheduleGameGroup::ScheduleId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ScheduleGameGroup::Table, ScheduleGameGroup::ScheduleId)
                            .to(Schedule::Table, Schedule::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .take(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ScheduleTeam::Table)
                    .col(ColumnDef::new(ScheduleTeam::Id).integer().primary_key())
                    .col(ColumnDef::new(ScheduleTeam::Name).string().not_null())
                    .col(
                        ColumnDef::new(ScheduleTeam::ScheduleId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduleTeam::ScheduleGameGroup)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ScheduleTeam::Table, ScheduleTeam::ScheduleId)
                            .to(Schedule::Table, Schedule::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ScheduleTeam::Table, ScheduleTeam::ScheduleGameGroup)
                            .to(ScheduleGameGroup::Table, ScheduleGameGroup::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IX_ScheduleTeam_schedule-game-group")
                    .table(ScheduleTeam::Table)
                    .col(ScheduleTeam::ScheduleGameGroup)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IX_ScheduleTeam_schedule-id")
                    .table(ScheduleTeam::Table)
                    .col(ScheduleTeam::ScheduleId)
                    .take(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ScheduleGame::Table)
                    .col(ColumnDef::new(ScheduleGame::Id).integer().primary_key())
                    .col(
                        ColumnDef::new(ScheduleGame::ScheduleId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ScheduleGame::Start).timestamp())
                    .col(ColumnDef::new(ScheduleGame::End).timestamp())
                    .col(ColumnDef::new(ScheduleGame::ScheduleGameGroup).integer())
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
                            .from(ScheduleGame::Table, ScheduleGame::ScheduleGameGroup)
                            .to(ScheduleGameGroup::Table, ScheduleGameGroup::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ScheduleGame::Table, ScheduleGame::TeamOne)
                            .to(ScheduleTeam::Table, ScheduleTeam::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ScheduleGame::Table, ScheduleGame::TeamTwo)
                            .to(ScheduleTeam::Table, ScheduleTeam::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IX_ScheduleGame_start")
                    .table(ScheduleGame::Table)
                    .col(ScheduleGame::Start)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IX_ScheduleGame_end")
                    .table(ScheduleGame::Table)
                    .col(ScheduleGame::End)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IX_ScheduleGame_schedule-game-group")
                    .table(ScheduleGame::Table)
                    .col(ScheduleGame::ScheduleGameGroup)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("IX_ScheduleGame_schedule-id")
                    .table(ScheduleGame::Table)
                    .col(ScheduleGame::ScheduleId)
                    .take(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("IX_ScheduleGame_schedule-id").to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("IX_ScheduleGame_schedule-game-group")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(Index::drop().name("IX_ScheduleGame_end").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("IX_ScheduleGame_start").to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ScheduleGame::Table).take())
            .await?;

        manager
            .drop_index(Index::drop().name("IX_ScheduleTeam_schedule-id").to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("IX_ScheduleTeam_schedule-game-group")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(ScheduleTeam::Table).take())
            .await?;

        manager
            .drop_table(Table::drop().table(ScheduleGameGroup::Table).take())
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
}

#[derive(DeriveIden)]
pub(crate) enum ScheduleGameGroup {
    Table,
    Id,
    ScheduleId,
    Name,
}

#[derive(DeriveIden)]
pub(crate) enum ScheduleTeam {
    Table,
    Id,
    ScheduleId,
    Name,
    ScheduleGameGroup,
}

#[derive(DeriveIden)]
pub(crate) enum ScheduleGame {
    Table,
    Id,
    ScheduleId,
    Start,
    End,
    ScheduleGameGroup,
    TeamOne,
    TeamTwo,
}
