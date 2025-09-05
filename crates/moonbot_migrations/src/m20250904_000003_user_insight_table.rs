use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(UserInsight::Table)
					.if_not_exists()
					.col(big_unsigned_uniq(UserInsight::UserId).primary_key())
					.col(text(UserInsight::Traits))
					.col(text(UserInsight::Preferences))
					.col(text(UserInsight::Summary))
					.col(integer(UserInsight::TrustLevel).default(0))
					.col(timestamp(UserInsight::FirstSeen).default(Expr::current_timestamp()))
					.col(timestamp(UserInsight::LastSeen).default(Expr::current_timestamp()))
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(UserInsight::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
enum UserInsight {
	Table,
	UserId,
	Traits,
	Preferences,
	Summary,
	TrustLevel,
	FirstSeen,
	LastSeen,
}
