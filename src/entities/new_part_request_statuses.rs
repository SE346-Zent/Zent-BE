use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "new_part_request_statuses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::new_part_forms::Entity")]
    NewPartForms,
}

impl Related<super::new_part_forms::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NewPartForms.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
