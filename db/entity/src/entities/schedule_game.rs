//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.14

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, PartialOrd, Ord,
)]
#[sea_orm(table_name = "schedule_game")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    pub schedule_id: i32,
    pub start: String,
    pub end: String,
    pub team_one: Option<i32>,
    pub team_two: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::schedule::Entity",
        from = "Column::ScheduleId",
        to = "super::schedule::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Schedule,
    #[sea_orm(
        belongs_to = "super::team::Entity",
        from = "Column::TeamTwo",
        to = "super::team::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Team2,
    #[sea_orm(
        belongs_to = "super::team::Entity",
        from = "Column::TeamOne",
        to = "super::team::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Team1,
}

impl Related<super::schedule::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Schedule.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
