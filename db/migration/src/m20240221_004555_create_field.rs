use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_region::Region;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .create_table(
                Table::create()
                    .table(Field::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Field::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Field::Name).string().not_null())
                    .col(ColumnDef::new(Field::RegionOwner).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_field_region")
                            .from(Field::Table, Field::RegionOwner)
                            .to(Region::Table, Region::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Field::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Field {
    Table,
    RegionOwner,
    Id,
    Name,
}
