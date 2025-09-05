use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(ChannelRoleplay::Table)
					.if_not_exists()
					.col(big_unsigned_uniq(ChannelRoleplay::ChannelId).primary_key())
					.col(text(ChannelRoleplay::Persona))
					.col(timestamp(ChannelRoleplay::UpdatedAt).default(Expr::current_timestamp()))
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(ChannelRoleplay::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
enum ChannelRoleplay {
	Table,
	ChannelId,
	Persona,
	UpdatedAt,
}
