//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.14

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, PartialOrd, Ord,
)]
#[sea_orm(table_name = "reservation_type_field_size_join")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub field: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub reservation_type: i32,
    pub size: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::field::Entity",
        from = "Column::Field",
        to = "super::field::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Field,
    #[sea_orm(
        belongs_to = "super::reservation_type::Entity",
        from = "Column::ReservationType",
        to = "super::reservation_type::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    ReservationType,
}

impl Related<super::field::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Field.def()
    }
}

impl Related<super::reservation_type::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ReservationType.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
