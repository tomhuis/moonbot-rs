use sea_orm_migration::{prelude::*, schema::*, sea_orm::{Statement, DbBackend}};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// Base table
		manager
			.create_table(
				Table::create()
					.table(UserDirectory::Table)
					.if_not_exists()
					.col(big_unsigned_uniq(UserDirectory::UserId).primary_key())
					.col(string(UserDirectory::DisplayName))
					.col(text(UserDirectory::Aliases))
					.col(text(UserDirectory::Notes))
					.col(timestamp(UserDirectory::UpdatedAt).default(Expr::current_timestamp()))
					.to_owned(),
			)
			.await?;

		// External-content FTS5 table (raw SQL)
		let db = manager.get_connection();
		db.execute(Statement::from_string(
			DbBackend::Sqlite,
			"CREATE VIRTUAL TABLE IF NOT EXISTS user_directory_fts USING fts5(content, content_rowid='rowid')".to_string(),
		)).await?;

		// Populate trigger set
		for (name, body) in [
			(
				"user_directory_fts_ai",
				"INSERT INTO user_directory_fts(rowid, content) VALUES (new.rowid, coalesce(new.display_name,'') || ' ' || coalesce(new.aliases,'') || ' ' || coalesce(new.notes,''));",
			),
			(
				"user_directory_fts_ad",
				"INSERT INTO user_directory_fts(user_directory_fts, rowid, content) VALUES('delete', old.rowid, '');",
			),
			(
				"user_directory_fts_au",
				"INSERT INTO user_directory_fts(user_directory_fts, rowid, content) VALUES('delete', old.rowid, '');
				 INSERT INTO user_directory_fts(rowid, content) VALUES (new.rowid, coalesce(new.display_name,'') || ' ' || coalesce(new.aliases,'') || ' ' || coalesce(new.notes,''));",
			),
		] {
			let sql = format!(
				"CREATE TRIGGER IF NOT EXISTS {name} AFTER {event} ON user_directory BEGIN {body} END;",
				name = name,
				event = if name.ends_with("_ai") { "INSERT" } else if name.ends_with("_ad") { "DELETE" } else { "UPDATE" },
				body = body
			);
			db.execute(Statement::from_string(DbBackend::Sqlite, sql)).await?;
		}

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let db = manager.get_connection();
		for name in [
			"user_directory_fts_ai",
			"user_directory_fts_ad",
			"user_directory_fts_au",
		] {
			let sql = format!("DROP TRIGGER IF EXISTS {name};");
			db.execute(Statement::from_string(DbBackend::Sqlite, sql)).await?;
		}
		db.execute(Statement::from_string(DbBackend::Sqlite, "DROP TABLE IF EXISTS user_directory_fts".to_string())).await?;
		manager
			.drop_table(Table::drop().table(UserDirectory::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
enum UserDirectory {
	Table,
	UserId,
	DisplayName,
	Aliases,
	Notes,
	UpdatedAt,
}

// FTS table created via raw SQL
