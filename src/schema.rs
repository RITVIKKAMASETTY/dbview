//! Schema inspection and table metadata

use anyhow::Result;
use rusqlite::Connection;

use crate::db;

/// Information about a database column
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub default_value: Option<String>,
}

/// Information about a database table
#[derive(Debug)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub row_count: i64,
    pub indexes: Vec<IndexInfo>,
}

/// Information about a table index
#[derive(Debug)]
pub struct IndexInfo {
    pub name: String,
    pub unique: bool,
    pub columns: Vec<String>,
}

/// Gets detailed schema information for a table
pub fn get_table_info(conn: &Connection, table: &str) -> Result<TableInfo> {
    // Check if table exists
    if !db::table_exists(conn, table)? {
        anyhow::bail!("Table '{}' does not exist", table);
    }

    let columns = get_columns(conn, table)?;
    let row_count = db::get_row_count(conn, table)?;
    let indexes = get_indexes(conn, table)?;

    Ok(TableInfo {
        name: table.to_string(),
        columns,
        row_count,
        indexes,
    })
}

/// Gets column information for a table
pub fn get_columns(conn: &Connection, table: &str) -> Result<Vec<ColumnInfo>> {
    let sql = format!("PRAGMA table_info(\"{}\")", db::escape_identifier(table));
    let mut stmt = conn.prepare(&sql)?;

    let columns: Vec<ColumnInfo> = stmt
        .query_map([], |row| {
            Ok(ColumnInfo {
                name: row.get(1)?,
                data_type: row
                    .get::<_, String>(2)
                    .unwrap_or_else(|_| "UNKNOWN".to_string()),
                nullable: row.get::<_, i32>(3)? == 0,
                primary_key: row.get::<_, i32>(5)? > 0,
                default_value: row.get::<_, Option<String>>(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(columns)
}

/// Gets index information for a table
pub fn get_indexes(conn: &Connection, table: &str) -> Result<Vec<IndexInfo>> {
    let sql = format!("PRAGMA index_list(\"{}\")", db::escape_identifier(table));
    let mut stmt = conn.prepare(&sql)?;

    let indexes: Vec<IndexInfo> = stmt
        .query_map([], |row| {
            let name: String = row.get(1)?;
            let unique: bool = row.get::<_, i32>(2)? == 1;
            Ok((name, unique))
        })?
        .filter_map(|r| r.ok())
        .map(|(name, unique)| {
            let columns = get_index_columns(conn, &name).unwrap_or_default();
            IndexInfo {
                name,
                unique,
                columns,
            }
        })
        .collect();

    Ok(indexes)
}

/// Gets the columns in an index
fn get_index_columns(conn: &Connection, index_name: &str) -> Result<Vec<String>> {
    let sql = format!(
        "PRAGMA index_info(\"{}\")",
        db::escape_identifier(index_name)
    );
    let mut stmt = conn.prepare(&sql)?;

    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(2))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(columns)
}

/// Infers a human-readable description of what a column likely represents
pub fn infer_column_purpose(column: &ColumnInfo) -> String {
    let name_lower = column.name.to_lowercase();

    // Check for common patterns
    if column.primary_key && (name_lower == "id" || name_lower.ends_with("_id")) {
        return "Primary identifier".to_string();
    }

    if name_lower.ends_with("_id") || name_lower.starts_with("id_") {
        return "Foreign key reference".to_string();
    }

    if name_lower.contains("email") {
        return "Email address".to_string();
    }

    if name_lower.contains("name") {
        if name_lower.contains("first") {
            return "First name".to_string();
        }
        if name_lower.contains("last") {
            return "Last name".to_string();
        }
        return "Name field".to_string();
    }

    if name_lower.contains("phone") || name_lower.contains("tel") {
        return "Phone number".to_string();
    }

    if name_lower.contains("address") {
        return "Address information".to_string();
    }

    if name_lower.contains("date") || name_lower.contains("time") {
        if name_lower.contains("created") || name_lower.contains("create") {
            return "Creation timestamp".to_string();
        }
        if name_lower.contains("updated") || name_lower.contains("modified") {
            return "Last modification timestamp".to_string();
        }
        return "Date/time value".to_string();
    }

    if name_lower.contains("price") || name_lower.contains("cost") || name_lower.contains("amount")
    {
        return "Monetary value".to_string();
    }

    if name_lower.contains("qty") || name_lower.contains("quantity") || name_lower.contains("count")
    {
        return "Quantity/count".to_string();
    }

    if name_lower.contains("status") || name_lower.contains("state") {
        return "Status indicator".to_string();
    }

    if name_lower.contains("active") || name_lower.contains("enabled") || name_lower.contains("is_")
    {
        return "Boolean flag".to_string();
    }

    if name_lower.contains("description") || name_lower.contains("desc") {
        return "Description text".to_string();
    }

    if name_lower.contains("url") || name_lower.contains("link") {
        return "URL/link".to_string();
    }

    if name_lower.contains("image") || name_lower.contains("photo") || name_lower.contains("avatar")
    {
        return "Image reference".to_string();
    }

    // Default based on data type
    match column.data_type.to_uppercase().as_str() {
        "INTEGER" | "INT" => "Integer value".to_string(),
        "REAL" | "FLOAT" | "DOUBLE" => "Decimal number".to_string(),
        "TEXT" | "VARCHAR" => "Text content".to_string(),
        "BLOB" => "Binary data".to_string(),
        "BOOLEAN" => "True/false flag".to_string(),
        _ => "Data field".to_string(),
    }
}
