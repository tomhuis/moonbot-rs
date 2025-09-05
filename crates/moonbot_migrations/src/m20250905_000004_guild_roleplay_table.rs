use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(GuildRoleplay::Table)
					.if_not_exists()
					.col(big_unsigned_uniq(GuildRoleplay::GuildId).primary_key())
					.col(text(GuildRoleplay::Persona))
					.col(timestamp(GuildRoleplay::UpdatedAt).default(Expr::current_timestamp()))
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(GuildRoleplay::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
enum GuildRoleplay {
	Table,
	GuildId,
	Persona,
	UpdatedAt,
}
