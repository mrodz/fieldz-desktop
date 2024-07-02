use sea_orm_migration::prelude::*;

use crate::{m20220101_000001_create_region::Region, m20240223_053746_create_team::Team};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(CoachConflict::Table)
                    .col(
                        ColumnDef::new(CoachConflict::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CoachConflict::Region).integer().not_null())
                    .col(ColumnDef::new(CoachConflict::CoachName).string())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_coach-conflict_region")
                            .from(CoachConflict::Table, CoachConflict::Region)
                            .to(Region::Table, Region::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .take(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(CoachConflictTeamJoin::Table)
                    .col(
                        ColumnDef::new(CoachConflictTeamJoin::CoachConflict)
                            .not_null()
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(CoachConflictTeamJoin::Team)
                            .not_null()
                            .integer(),
                    )
                    .primary_key(
                        Index::create()
                            .col(CoachConflictTeamJoin::CoachConflict)
                            .col(CoachConflictTeamJoin::Team),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_coach-conflict")
                            .from(
                                CoachConflictTeamJoin::Table,
                                CoachConflictTeamJoin::CoachConflict,
                            )
                            .to(CoachConflict::Table, CoachConflict::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_coach-conflict_team")
                            .from(CoachConflictTeamJoin::Table, CoachConflictTeamJoin::Team)
                            .to(Team::Table, Team::Id),
                    )
                    .take(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(CoachConflictTeamJoin::Table)
                    .take(),
            )
            .await?;

        manager
            .drop_table(Table::drop().if_exists().table(CoachConflict::Table).take())
            .await
    }
}

#[derive(DeriveIden)]
pub enum CoachConflict {
    Table,
    Id,
    Region,
    CoachName,
}

#[derive(DeriveIden)]
pub enum CoachConflictTeamJoin {
    Table,
    CoachConflict,
    Team,
}
