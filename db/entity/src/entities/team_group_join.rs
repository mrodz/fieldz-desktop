//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.14

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, PartialOrd, Ord,
)]
#[sea_orm(table_name = "team_group_join")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub team: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub group: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::team::Entity",
        from = "Column::Team",
        to = "super::team::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Team,
    #[sea_orm(
        belongs_to = "super::team_group::Entity",
        from = "Column::Group",
        to = "super::team_group::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TeamGroup,
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
    }
}

impl Related<super::team_group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TeamGroup.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
