//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.14

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "time_slot")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub field_id: i32,
    pub start: String,
    pub end: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::field::Entity",
        from = "Column::FieldId",
        to = "super::field::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Field,
}

impl Related<super::field::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Field.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
