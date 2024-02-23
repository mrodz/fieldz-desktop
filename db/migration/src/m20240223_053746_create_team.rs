use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_region::Region;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TeamGroup::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TeamGroup::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TeamGroup::Name).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Team::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Team::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Team::Name).string().not_null())
                    .col(ColumnDef::new(Team::RegionOwner).integer().not_null())
                    .col(ColumnDef::new(Team::TeamGroup).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_team_region")
                            .from(Team::Table, Team::RegionOwner)
                            .to(Region::Table, Region::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_team_group")
                            .from(Team::Table, Team::TeamGroup)
                            .to(TeamGroup::Table, TeamGroup::Id)
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
            .drop_table(Table::drop().table(Team::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(TeamGroup::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum TeamGroup {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
pub(crate) enum Team {
    Table,
    RegionOwner,
    TeamGroup,
    Id,
    Name,
}
