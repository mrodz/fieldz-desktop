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
                    .col(
                        ColumnDef::new(TeamGroup::Name)
                            .string()
                            .not_null()
                            .extra("COLLATE nocase"),
                    )
                    .col(
                        ColumnDef::new(TeamGroup::Usages)
                            .integer()
                            .default(0)
                            .not_null(),
                    )
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
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_team_region")
                            .from(Team::Table, Team::RegionOwner)
                            .to(Region::Table, Region::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TeamGroupJoin::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TeamGroupJoin::Team).integer().not_null())
                    .col(ColumnDef::new(TeamGroupJoin::Group).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(TeamGroupJoin::Team)
                            .col(TeamGroupJoin::Group),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_group")
                            .from(TeamGroupJoin::Table, TeamGroupJoin::Group)
                            .to(TeamGroup::Table, TeamGroup::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_join_team")
                            .from(TeamGroupJoin::Table, TeamGroupJoin::Team)
                            .to(Team::Table, Team::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // the join table needs to be dropped first
        manager
            .drop_table(
                Table::drop()
                    .if_exists()
                    .table(TeamGroupJoin::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().if_exists().table(Team::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().if_exists().table(TeamGroup::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub(crate) enum TeamGroup {
    Table,
    Id,
    Name,
    Usages,
}

#[derive(DeriveIden)]
pub(crate) enum Team {
    Table,
    RegionOwner,
    Id,
    Name,
}

#[derive(DeriveIden)]
pub(crate) enum TeamGroupJoin {
    Table,
    Team,
    Group,
}
