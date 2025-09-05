//! `SeaORM` Entity for guild_roleplay
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "guild_roleplay")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub guild_id: i64,
	pub persona: String,
	pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
