//! `SeaORM` Entity for user_insight (profiles)
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_insight")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub user_id: i64,
	pub traits: String,
	pub preferences: String,
	pub summary: String,
	pub trust_level: i32,
	pub first_seen: DateTimeUtc,
	pub last_seen: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
