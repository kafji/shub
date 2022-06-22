use crate::{
    repository_id::IsRepositoryId,
    types::{BuildStatus, Repository},
};
use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    ToSql,
};
use std::{fmt, path::Path};
use tracing::info;

type Repositories = Vec<Repository>;

/// Sql for databse migrations.
///
/// All operations must be idempotent.
const MIGRATIONS: &'static str = "
    CREATE TABLE IF NOT EXISTS repositories (
        rid INTEGER PRIMARY KEY AUTOINCREMENT,
        owner TEXT NOT NULL,
        name TEXT NOT NULL,
        a_fork Boll NOT NULL DEFAULT FALSE,
        archived BOOL NOT NULL DEFAULT FALSE,
        build_status TEXT NULL,
        UNIQUE (owner, name) ON CONFLICT REPLACE
    );
";

pub struct Database(rusqlite::Connection);

impl Database {
    #[tracing::instrument]
    pub fn new(path: &Path) -> Result<Self, anyhow::Error> {
        let conn = rusqlite::Connection::open(path)?;
        let db = Self(conn);
        migrate(&db)?;
        Ok(db)
    }

    #[tracing::instrument(skip(self))]
    pub fn put_repositories(&mut self, repositories: &[Repository]) -> Result<(), anyhow::Error> {
        put_repositories(self, repositories)
    }

    #[tracing::instrument(skip(self))]
    pub fn get_dashboard_repositories(&self, owner: &str) -> Result<Repositories, anyhow::Error> {
        get_dashboard_repositories(self, owner)
    }

    /// Set build statuses of repositories.
    #[tracing::instrument(skip(self))]
    pub fn set_build_statuses(
        &mut self,
        build_statuses: &[(impl IsRepositoryId + fmt::Debug, BuildStatus)],
    ) -> Result<(), anyhow::Error> {
        let tx = self.0.transaction()?;
        let mut stmt = tx.prepare_cached(
            "UPDATE repositories
                SET build_status = ?
                WHERE
                    owner = ? AND
                    name = ?
            ;",
        )?;
        for (id, status) in build_statuses {
            stmt.execute(params![status, id.owner(), id.name()])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(())
    }
}

/// Migrates database.
fn migrate(db: &Database) -> Result<(), anyhow::Error> {
    db.0.execute_batch(MIGRATIONS)?;
    Ok(())
}

// todo(kfj): better name
fn get_dashboard_repositories(
    db: &Database,
    owner: &str,
) -> Result<Vec<Repository>, anyhow::Error> {
    let mut stmt = db.0.prepare_cached(
        "SELECT owner, name, build_status
            FROM repositories
            WHERE
                owner = ? AND
                a_fork = FALSE AND
                archived = FALSE
        ;",
    )?;
    let repositories = stmt
        .query_map([owner], |x| {
            let owner = x.get(0)?;
            let name = x.get(1)?;
            let build_status = x.get(2)?;
            let r = Repository {
                name,
                owner,
                a_fork: false,
                archived: false,
                build_status,
            };
            Ok(r)
        })?
        .collect::<Result<_, _>>()?;
    Ok(repositories)
}

/// Puts repositories into database.
///
/// On conflict, will replace the stored repository.
fn put_repositories(db: &mut Database, repositories: &[Repository]) -> Result<(), anyhow::Error> {
    let tx = db.0.transaction()?;
    for Repository {
        name,
        owner,
        a_fork,
        archived: acrhived,
        build_status,
    } in repositories
    {
        tx.execute(
            "INSERT INTO repositories (
                name,
                owner,
                a_fork,
                archived,
                build_status
            ) VALUES (?, ?, ?, ?, ?)
            ;",
            params![name, owner, a_fork, acrhived, build_status],
        )?;
    }
    tx.commit()?;
    Ok(())
}

// to/from sql conversions ------------------------------

impl ToSql for BuildStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let s = self.to_string();
        Ok(ToSqlOutput::Owned(Value::Text(s)))
    }
}

impl FromSql for BuildStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;
        s.parse().map_err(|x| FromSqlError::Other(Box::new(x)))
    }
}

// end: to/from sql conversions ------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use rusqlite::Connection;

    fn connect() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        Database(conn)
    }

    fn migrate_(db: &Database) {
        migrate(db).unwrap()
    }

    #[test]
    fn test_migration_safe_to_run_multiple_time() {
        let db = connect();
        for _ in 0..3 {
            migrate(&db).unwrap();
        }
    }

    #[test]
    fn test_get_dashboard_repositories() {
        let mut db = connect();
        migrate_(&db);

        {
            let rs = [Repository {
                name: "World".to_owned(),
                owner: "Hello".to_owned(),
                a_fork: false,
                archived: false,
                build_status: None,
            }];
            put_repositories(&mut db, &rs).unwrap();
        };

        let rs = get_dashboard_repositories(&db, "Hello").unwrap();
        assert_eq!(
            rs,
            [Repository {
                name: "World".to_owned(),
                owner: "Hello".to_owned(),
                a_fork: false,
                archived: false,
                build_status: None,
            }]
        );
    }
}
