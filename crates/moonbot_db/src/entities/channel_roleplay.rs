//! `SeaORM` Entity for channel_roleplay
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "channel_roleplay")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub channel_id: i64,
	pub persona: String,
	pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
