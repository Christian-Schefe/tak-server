use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::ToSql;

use crate::DatabaseError;

pub mod games;
pub mod players;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

pub fn to_sql_option<T>(value: &Option<T>) -> Option<&dyn ToSql>
where
    T: ToSql,
{
    value.as_ref().map(|v| v as &dyn ToSql)
}

pub fn get_connection(
    pool: &Pool<SqliteConnectionManager>,
) -> DatabaseResult<PooledConnection<SqliteConnectionManager>> {
    pool.get().map_err(|e| DatabaseError::ConnectionError(e))
}

fn update_entry(
    pool: &Pool<SqliteConnectionManager>,
    table: &str,
    id: (&str, &dyn ToSql),
    value_pairs: Vec<(&str, Option<&dyn ToSql>)>,
) -> DatabaseResult<()> {
    let mut query = format!("UPDATE {} SET ", table);
    let mut conditions = Vec::new();
    let mut params: Vec<&dyn ToSql> = Vec::new();

    for (field, value) in value_pairs {
        if let Some(v) = value {
            conditions.push(format!("{} = ?", field));
            params.push(v);
        }
    }
    if params.is_empty() {
        return Ok(());
    }
    query.push_str(&conditions.join(", "));
    query.push_str(&format!(" WHERE {} = ?", id.0));
    params.push(id.1);
    let conn = get_connection(pool)?;
    conn.execute(&query, rusqlite::params_from_iter(params.iter()))
        .map_err(|e| DatabaseError::QueryError(e))?;
    Ok(())
}
