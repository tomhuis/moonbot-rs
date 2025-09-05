//! `SeaORM` Entity for user_directory (FTS external content present via triggers)
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_directory")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub user_id: i64,
	pub display_name: String,
	pub aliases: String,
	pub notes: String,
	pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
