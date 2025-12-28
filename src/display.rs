//! Display formatting and output rendering

use anyhow::Result;
use colored::*;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use num_format::{Locale, ToFormattedString};
use rusqlite::Connection;

use crate::db;
use crate::schema::{self, ColumnInfo};
use crate::stats;

/// Displays a list of all tables with row counts
pub fn show_tables(conn: &Connection) -> Result<()> {
    let tables = db::get_tables(conn)?;

    if tables.is_empty() {
        println!("{}", "No tables found in database.".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "📊 Database Tables".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Table Name").fg(Color::Cyan),
            Cell::new("Rows").fg(Color::Cyan),
            Cell::new("Columns").fg(Color::Cyan),
        ]);

    for table_name in &tables {
        let row_count = db::get_row_count(conn, table_name).unwrap_or(0);
        let columns = schema::get_columns(conn, table_name).unwrap_or_default();

        table.add_row(vec![
            Cell::new(table_name).fg(Color::White),
            Cell::new(row_count.to_formatted_string(&Locale::en)).fg(Color::Green),
            Cell::new(columns.len()).fg(Color::Yellow),
        ]);
    }

    println!("{table}");
    println!("{}", format!("Total: {} tables", tables.len()).dimmed());
    println!();

    Ok(())
}

/// Displays the schema of a specific table
pub fn show_schema(conn: &Connection, table_name: &str) -> Result<()> {
    let info = schema::get_table_info(conn, table_name)?;

    println!();
    println!("{}", format!("📋 Schema: {}", table_name).cyan().bold());
    println!(
        "{}",
        format!("{} rows", info.row_count.to_formatted_string(&Locale::en)).dimmed()
    );
    println!("{}", "─".repeat(70).dimmed());

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Column").fg(Color::Cyan),
            Cell::new("Type").fg(Color::Cyan),
            Cell::new("Nullable").fg(Color::Cyan),
            Cell::new("PK").fg(Color::Cyan),
            Cell::new("Default").fg(Color::Cyan),
        ]);

    for col in &info.columns {
        let nullable = if col.nullable { "YES" } else { "NO" };
        let pk = if col.primary_key { "✓" } else { "" };
        let default = col.default_value.as_deref().unwrap_or("-");

        table.add_row(vec![
            Cell::new(&col.name).fg(Color::White),
            Cell::new(&col.data_type).fg(Color::Yellow),
            Cell::new(nullable).fg(if col.nullable {
                Color::Green
            } else {
                Color::Red
            }),
            Cell::new(pk).fg(Color::Magenta),
            Cell::new(default).fg(Color::DarkGrey),
        ]);
    }

    println!("{table}");

    // Show indexes if any
    if !info.indexes.is_empty() {
        println!();
        println!("{}", "Indexes:".cyan());
        for idx in &info.indexes {
            let unique_str = if idx.unique { " (unique)" } else { "" };
            println!(
                "  • {}: [{}]{}",
                idx.name.white(),
                idx.columns.join(", ").yellow(),
                unique_str.dimmed()
            );
        }
    }

    println!();
    Ok(())
}

/// Displays records from a table in a formatted table
pub fn show_records(
    conn: &Connection,
    table_name: &str,
    limit: usize,
    offset: usize,
) -> Result<()> {
    let columns = db::get_column_names(conn, table_name)?;
    let rows = db::fetch_rows(conn, table_name, limit, offset)?;
    let total_count = db::get_row_count(conn, table_name)?;

    println!();
    println!("{}", format!("📄 Records: {}", table_name).cyan().bold());
    println!(
        "{}",
        format!(
            "Showing {} of {} rows (offset: {})",
            rows.len().min(limit),
            total_count.to_formatted_string(&Locale::en),
            offset
        )
        .dimmed()
    );
    println!("{}", "─".repeat(80).dimmed());

    if rows.is_empty() {
        println!("{}", "No records found.".yellow());
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    // Header
    let header_cells: Vec<Cell> = columns
        .iter()
        .map(|c| Cell::new(c).fg(Color::Cyan))
        .collect();
    table.set_header(header_cells);

    // Rows
    for row in &rows {
        let cells: Vec<Cell> = row
            .iter()
            .map(|v| {
                if v == "NULL" {
                    Cell::new(v).fg(Color::DarkGrey)
                } else {
                    Cell::new(truncate_str(v, 30))
                }
            })
            .collect();
        table.add_row(cells);
    }

    println!("{table}");

    // Show pagination hint if there are more rows
    if offset + rows.len() < total_count as usize {
        println!(
            "{}",
            format!("Use --offset {} to see more rows", offset + limit).dimmed()
        );
    }

    println!();
    Ok(())
}

/// Displays statistics for a table
pub fn show_stats(conn: &Connection, table_name: &str) -> Result<()> {
    let stats = stats::compute_table_stats(conn, table_name)?;

    println!();
    println!("{}", format!("📈 Statistics: {}", table_name).cyan().bold());
    println!("{}", "─".repeat(80).dimmed());

    for col_stat in &stats {
        println!();
        println!(
            "{} ({})",
            col_stat.column_name.white().bold(),
            col_stat.data_type.yellow()
        );

        let null_pct = if col_stat.total_count > 0 {
            (col_stat.null_count as f64 / col_stat.total_count as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "  Total: {} | Unique: {} | Nulls: {} ({:.1}%)",
            col_stat
                .total_count
                .to_formatted_string(&Locale::en)
                .green(),
            col_stat
                .unique_count
                .to_formatted_string(&Locale::en)
                .blue(),
            col_stat.null_count.to_formatted_string(&Locale::en).red(),
            null_pct
        );

        if let Some(ref num_stats) = col_stat.numeric_stats {
            println!(
                "  Min: {:.2} | Max: {:.2} | Avg: {:.2} | Sum: {:.2}",
                num_stats.min, num_stats.max, num_stats.avg, num_stats.sum
            );
            if let Some(std_dev) = num_stats.std_dev {
                println!("  Std Dev: {:.4}", std_dev);
            }
        }

        if let Some(ref text_stats) = col_stat.text_stats {
            println!(
                "  Length: min {} | max {} | avg {:.1}",
                text_stats.min_length, text_stats.max_length, text_stats.avg_length
            );
            if !text_stats.most_common.is_empty() {
                println!("  Most common values:");
                for (value, count) in text_stats.most_common.iter().take(3) {
                    let display_val = truncate_str(value, 30);
                    println!("    • {} ({})", display_val.dimmed(), count);
                }
            }
        }
    }

    println!();
    Ok(())
}

/// Describes a table in human-readable form
pub fn describe_table(conn: &Connection, table_name: &str) -> Result<()> {
    let info = schema::get_table_info(conn, table_name)?;

    println!();
    println!(
        "{}",
        format!("🔍 Description: {}", table_name).cyan().bold()
    );
    println!("{}", "─".repeat(60).dimmed());

    // Infer purpose
    let table_purpose = infer_table_purpose(&info.columns, table_name);
    println!();
    println!("{}", table_purpose.white());
    println!();
    println!(
        "This table contains {} records with {} columns:",
        info.row_count.to_formatted_string(&Locale::en).green(),
        info.columns.len().to_string().blue()
    );
    println!();

    for col in &info.columns {
        let purpose = schema::infer_column_purpose(col);
        let nullable_str = if col.nullable { "" } else { " (required)" };
        let pk_str = if col.primary_key {
            " [PRIMARY KEY]"
        } else {
            ""
        };

        println!(
            "  • {}{}{}: {}",
            col.name.white().bold(),
            pk_str.magenta(),
            nullable_str.dimmed(),
            purpose.dimmed()
        );
    }

    println!();
    Ok(())
}

/// Shows a summary of the entire database
pub fn show_summary(conn: &Connection) -> Result<()> {
    let tables = db::get_tables(conn)?;

    let mut total_rows: i64 = 0;
    let mut table_info: Vec<(String, i64, usize)> = Vec::new();

    for table_name in &tables {
        let row_count = db::get_row_count(conn, &table_name).unwrap_or(0);
        let columns = schema::get_columns(conn, &table_name).unwrap_or_default();
        total_rows += row_count;
        table_info.push((table_name.clone(), row_count, columns.len()));
    }

    println!();
    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║               📊 DATABASE SUMMARY                            ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    println!(
        "  {} {}",
        "Tables:".dimmed(),
        tables.len().to_string().green().bold()
    );
    println!(
        "  {} {}",
        "Total Rows:".dimmed(),
        total_rows.to_formatted_string(&Locale::en).green().bold()
    );

    // Find largest table
    if let Some((name, rows, _)) = table_info.iter().max_by_key(|(_, r, _)| r) {
        println!(
            "  {} {} ({} rows)",
            "Largest Table:".dimmed(),
            name.white(),
            rows.to_formatted_string(&Locale::en)
        );
    }

    println!();
    println!("{}", "Tables Overview:".cyan());
    println!("{}", "─".repeat(50).dimmed());

    for (name, rows, cols) in &table_info {
        let bar_width = if total_rows > 0 {
            ((*rows as f64 / total_rows as f64) * 30.0).round() as usize
        } else {
            0
        };
        let bar = "█".repeat(bar_width);

        println!(
            "  {:20} {:>10} rows {:>3} cols {}",
            name.white(),
            rows.to_formatted_string(&Locale::en).green(),
            cols,
            bar.blue()
        );
    }

    println!();
    Ok(())
}

/// Truncates a string to a maximum length
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

/// Infers the purpose of a table based on its name and columns
fn infer_table_purpose(columns: &[ColumnInfo], table_name: &str) -> String {
    let name_lower = table_name.to_lowercase();
    let col_names: Vec<String> = columns.iter().map(|c| c.name.to_lowercase()).collect();

    // Check for common table types
    if name_lower.contains("user")
        || name_lower.contains("account")
        || name_lower.contains("member")
    {
        return format!(
            "The '{}' table stores user/account information.",
            table_name
        );
    }

    if name_lower.contains("order") || name_lower.contains("purchase") {
        return format!("The '{}' table tracks order/purchase records.", table_name);
    }

    if name_lower.contains("product")
        || name_lower.contains("item")
        || name_lower.contains("inventory")
    {
        return format!(
            "The '{}' table contains product/inventory data.",
            table_name
        );
    }

    if name_lower.contains("log") || name_lower.contains("audit") || name_lower.contains("history")
    {
        return format!(
            "The '{}' table maintains a log or history of events.",
            table_name
        );
    }

    if name_lower.contains("config") || name_lower.contains("setting") {
        return format!(
            "The '{}' table stores configuration or settings.",
            table_name
        );
    }

    if name_lower.contains("session") || name_lower.contains("token") {
        return format!(
            "The '{}' table manages sessions or authentication tokens.",
            table_name
        );
    }

    // Check for join tables (many-to-many)
    if name_lower.contains("_to_")
        || (columns.len() <= 3 && col_names.iter().filter(|c| c.ends_with("_id")).count() >= 2)
    {
        return format!(
            "The '{}' table appears to be a junction/mapping table for many-to-many relationships.",
            table_name
        );
    }

    // Default
    format!("The '{}' table stores related data records.", table_name)
}
