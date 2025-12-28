//! Statistical analysis for database columns

use anyhow::Result;
use rusqlite::Connection;

use crate::db;
use crate::schema::{self, ColumnInfo};

/// Statistics for a single column
#[derive(Debug)]
pub struct ColumnStats {
    pub column_name: String,
    pub data_type: String,
    pub total_count: i64,
    pub null_count: i64,
    pub unique_count: i64,
    pub numeric_stats: Option<NumericStats>,
    pub text_stats: Option<TextStats>,
}

/// Statistics for numeric columns
#[derive(Debug)]
pub struct NumericStats {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub sum: f64,
    pub std_dev: Option<f64>,
}

/// Statistics for text columns
#[derive(Debug)]
pub struct TextStats {
    pub min_length: i64,
    pub max_length: i64,
    pub avg_length: f64,
    pub most_common: Vec<(String, i64)>,
}

/// Computes statistics for all columns in a table
pub fn compute_table_stats(conn: &Connection, table: &str) -> Result<Vec<ColumnStats>> {
    let columns = schema::get_columns(conn, table)?;
    let row_count = db::get_row_count(conn, table)?;

    let mut stats = Vec::new();
    for column in columns {
        let col_stats = compute_column_stats(conn, table, &column, row_count)?;
        stats.push(col_stats);
    }

    Ok(stats)
}

/// Computes statistics for a single column
fn compute_column_stats(
    conn: &Connection,
    table: &str,
    column: &ColumnInfo,
    total_count: i64,
) -> Result<ColumnStats> {
    let col_name = &column.name;
    let escaped_table = db::escape_identifier(table);
    let escaped_col = db::escape_identifier(col_name);

    // Get null count
    let null_sql = format!(
        "SELECT COUNT(*) FROM \"{}\" WHERE \"{}\" IS NULL",
        escaped_table, escaped_col
    );
    let null_count: i64 = conn.query_row(&null_sql, [], |row| row.get(0))?;

    // Get unique count
    let unique_sql = format!(
        "SELECT COUNT(DISTINCT \"{}\") FROM \"{}\"",
        escaped_col, escaped_table
    );
    let unique_count: i64 = conn.query_row(&unique_sql, [], |row| row.get(0))?;

    // Determine if numeric or text based on type
    let data_type_upper = column.data_type.to_uppercase();
    let is_numeric = matches!(
        data_type_upper.as_str(),
        "INTEGER" | "INT" | "REAL" | "FLOAT" | "DOUBLE" | "NUMERIC" | "DECIMAL"
    );

    let numeric_stats = if is_numeric {
        compute_numeric_stats(conn, table, col_name).ok()
    } else {
        None
    };

    let text_stats = if !is_numeric {
        compute_text_stats(conn, table, col_name).ok()
    } else {
        None
    };

    Ok(ColumnStats {
        column_name: col_name.clone(),
        data_type: column.data_type.clone(),
        total_count,
        null_count,
        unique_count,
        numeric_stats,
        text_stats,
    })
}

/// Computes numeric statistics for a column
fn compute_numeric_stats(conn: &Connection, table: &str, column: &str) -> Result<NumericStats> {
    let escaped_table = db::escape_identifier(table);
    let escaped_col = db::escape_identifier(column);

    let sql = format!(
        "SELECT MIN(\"{}\"), MAX(\"{}\"), AVG(\"{}\"), SUM(\"{}\") FROM \"{}\" WHERE \"{}\" IS NOT NULL",
        escaped_col, escaped_col, escaped_col, escaped_col, escaped_table, escaped_col
    );

    let (min, max, avg, sum): (f64, f64, f64, f64) = conn.query_row(&sql, [], |row| {
        Ok((
            row.get(0).unwrap_or(0.0),
            row.get(1).unwrap_or(0.0),
            row.get(2).unwrap_or(0.0),
            row.get(3).unwrap_or(0.0),
        ))
    })?;

    // Calculate standard deviation
    let std_sql = format!(
        "SELECT AVG((\"{0}\" - {1}) * (\"{0}\" - {1})) FROM \"{2}\" WHERE \"{0}\" IS NOT NULL",
        escaped_col, avg, escaped_table
    );
    let variance: Option<f64> = conn.query_row(&std_sql, [], |row| row.get(0)).ok();
    let std_dev = variance.map(|v| v.sqrt());

    Ok(NumericStats {
        min,
        max,
        avg,
        sum,
        std_dev,
    })
}

/// Computes text statistics for a column
fn compute_text_stats(conn: &Connection, table: &str, column: &str) -> Result<TextStats> {
    let escaped_table = db::escape_identifier(table);
    let escaped_col = db::escape_identifier(column);

    // Length stats
    let length_sql = format!(
        "SELECT MIN(LENGTH(\"{}\")), MAX(LENGTH(\"{}\")), AVG(LENGTH(\"{}\"::TEXT)) FROM \"{}\" WHERE \"{}\" IS NOT NULL",
        escaped_col, escaped_col, escaped_col, escaped_table, escaped_col
    );

    // Fallback simpler query
    let simple_length_sql = format!(
        "SELECT MIN(LENGTH(\"{}\")), MAX(LENGTH(\"{}\")), AVG(LENGTH(\"{}\")) FROM \"{}\" WHERE \"{}\" IS NOT NULL",
        escaped_col, escaped_col, escaped_col, escaped_table, escaped_col
    );

    let (min_length, max_length, avg_length): (i64, i64, f64) = conn
        .query_row(&simple_length_sql, [], |row| {
            Ok((
                row.get(0).unwrap_or(0),
                row.get(1).unwrap_or(0),
                row.get(2).unwrap_or(0.0),
            ))
        })
        .unwrap_or((0, 0, 0.0));

    // Most common values (top 5)
    let common_sql = format!(
        "SELECT \"{}\", COUNT(*) as cnt FROM \"{}\" WHERE \"{}\" IS NOT NULL GROUP BY \"{}\" ORDER BY cnt DESC LIMIT 5",
        escaped_col, escaped_table, escaped_col, escaped_col
    );

    let mut stmt = conn.prepare(&common_sql)?;
    let most_common: Vec<(String, i64)> = stmt
        .query_map([], |row| {
            let value: String = row
                .get::<_, rusqlite::types::Value>(0)
                .map(|v| format!("{:?}", v))
                .unwrap_or_else(|_| "NULL".to_string());
            let count: i64 = row.get(1)?;
            Ok((value, count))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(TextStats {
        min_length,
        max_length,
        avg_length,
        most_common,
    })
}
