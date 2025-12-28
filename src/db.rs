//! Database connection and query utilities

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// Opens a read-only connection to a SQLite database
pub fn connect(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("Failed to open database: {}", path.display()))?;
    Ok(conn)
}

/// Returns a list of all user tables in the database
pub fn get_tables(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    )?;

    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(tables)
}

/// Returns the row count for a table
pub fn get_row_count(conn: &Connection, table: &str) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM \"{}\"", escape_identifier(table));
    let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(count)
}

/// Escapes a SQL identifier to prevent injection
pub fn escape_identifier(name: &str) -> String {
    name.replace('"', "\"\"")
}

/// Checks if a table exists in the database
pub fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?",
        [table],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Returns the column names for a table
pub fn get_column_names(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let sql = format!("PRAGMA table_info(\"{}\")", escape_identifier(table));
    let mut stmt = conn.prepare(&sql)?;

    let names: Vec<String> = stmt
        .query_map([], |row| row.get(1))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(names)
}

/// Fetches rows from a table with a limit and offset
pub fn fetch_rows(
    conn: &Connection,
    table: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<Vec<String>>> {
    let columns = get_column_names(conn, table)?;
    let sql = format!(
        "SELECT * FROM \"{}\" LIMIT {} OFFSET {}",
        escape_identifier(table),
        limit,
        offset
    );

    let mut stmt = conn.prepare(&sql)?;
    let column_count = columns.len();

    let rows: Vec<Vec<String>> = stmt
        .query_map([], |row| {
            let mut values = Vec::with_capacity(column_count);
            for i in 0..column_count {
                let value: rusqlite::types::Value = row.get(i)?;
                values.push(format_value(&value));
            }
            Ok(values)
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

/// Formats a SQLite value as a display string
fn format_value(value: &rusqlite::types::Value) -> String {
    use rusqlite::types::Value;
    match value {
        Value::Null => "NULL".to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => format!("{:.4}", f),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => format!("<BLOB {} bytes>", b.len()),
    }
}

/// Fetches all values from a specific column
pub fn fetch_column_values(conn: &Connection, table: &str, column: &str) -> Result<Vec<f64>> {
    let sql = format!(
        "SELECT \"{}\" FROM \"{}\" WHERE \"{}\" IS NOT NULL AND typeof(\"{}\") IN ('integer', 'real')",
        escape_identifier(column),
        escape_identifier(table),
        escape_identifier(column),
        escape_identifier(column)
    );

    let mut stmt = conn.prepare(&sql)?;
    let values: Vec<f64> = stmt
        .query_map([], |row| row.get::<_, f64>(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(values)
}
