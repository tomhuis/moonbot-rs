use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(BotDisposition::Table)
					.if_not_exists()
					.col(integer(BotDisposition::Id).primary_key().not_null())
					.col(string(BotDisposition::Mood))
					.col(integer(BotDisposition::MoodLevel))
					.col(text(BotDisposition::Notes))
					.col(timestamp(BotDisposition::UpdatedAt).default(Expr::current_timestamp()))
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(BotDisposition::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
enum BotDisposition {
	Table,
	Id,
	Mood,
	MoodLevel,
	Notes,
	UpdatedAt,
}
