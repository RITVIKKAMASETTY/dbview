//! ASCII visualization for data analysis

use anyhow::{Context, Result};
use colored::*;
use rusqlite::Connection;

use crate::db;

/// Displays an ASCII histogram for a numeric column
pub fn show_histogram(conn: &Connection, table: &str, column: &str, bins: usize) -> Result<()> {
    // Fetch numeric values
    let values = db::fetch_column_values(conn, table, column)
        .with_context(|| format!("Failed to fetch values from {}.{}", table, column))?;

    if values.is_empty() {
        println!("{}", "No numeric data found for histogram".yellow());
        return Ok(());
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < f64::EPSILON {
        println!(
            "{}",
            "All values are identical - cannot create histogram".yellow()
        );
        return Ok(());
    }

    // Create bins
    let bin_width = (max - min) / bins as f64;
    let mut bin_counts: Vec<usize> = vec![0; bins];

    for &value in &values {
        let bin_idx = ((value - min) / bin_width).floor() as usize;
        let bin_idx = bin_idx.min(bins - 1); // Handle edge case for max value
        bin_counts[bin_idx] += 1;
    }

    let max_count = *bin_counts.iter().max().unwrap_or(&1);
    let bar_max_width = 50;

    // Print header
    println!();
    println!(
        "{}",
        format!("Histogram: {}.{}", table, column).cyan().bold()
    );
    println!(
        "{}",
        format!(
            "Range: {:.2} to {:.2} | {} values | {} bins",
            min,
            max,
            values.len(),
            bins
        )
        .dimmed()
    );
    println!("{}", "─".repeat(70).dimmed());

    // Print bars
    for (i, &count) in bin_counts.iter().enumerate() {
        let bin_start = min + (i as f64 * bin_width);
        let bin_end = bin_start + bin_width;

        // Calculate bar width
        let bar_width = if max_count > 0 {
            (count as f64 / max_count as f64 * bar_max_width as f64).round() as usize
        } else {
            0
        };

        // Format range label
        let range_label = format!("{:>8.2} - {:<8.2}", bin_start, bin_end);

        // Create bar
        let bar = "█".repeat(bar_width);
        let bar_colored = if count > 0 { bar.green() } else { bar.dimmed() };

        println!("{} │{} {}", range_label.dimmed(), bar_colored, count);
    }

    println!("{}", "─".repeat(70).dimmed());
    println!();

    Ok(())
}

/// Creates a simple bar chart for categorical data
pub fn show_bar_chart(conn: &Connection, table: &str, column: &str, limit: usize) -> Result<()> {
    let escaped_table = db::escape_identifier(table);
    let escaped_col = db::escape_identifier(column);

    let sql = format!(
        "SELECT \"{}\", COUNT(*) as cnt FROM \"{}\" WHERE \"{}\" IS NOT NULL GROUP BY \"{}\" ORDER BY cnt DESC LIMIT {}",
        escaped_col, escaped_table, escaped_col, escaped_col, limit
    );

    let mut stmt = conn.prepare(&sql)?;
    let data: Vec<(String, i64)> = stmt
        .query_map([], |row| {
            let value: String = row
                .get::<_, rusqlite::types::Value>(0)
                .map(|v| match v {
                    rusqlite::types::Value::Null => "NULL".to_string(),
                    rusqlite::types::Value::Integer(i) => i.to_string(),
                    rusqlite::types::Value::Real(f) => format!("{:.2}", f),
                    rusqlite::types::Value::Text(s) => s,
                    rusqlite::types::Value::Blob(_) => "<BLOB>".to_string(),
                })
                .unwrap_or_else(|_| "NULL".to_string());
            let count: i64 = row.get(1)?;
            Ok((value, count))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if data.is_empty() {
        println!("{}", "No data found for bar chart".yellow());
        return Ok(());
    }

    let max_count = data.iter().map(|(_, c)| *c).max().unwrap_or(1);
    let max_label_len = data
        .iter()
        .map(|(l, _)| l.len())
        .max()
        .unwrap_or(10)
        .min(20);
    let bar_max_width = 40;

    println!();
    println!(
        "{}",
        format!("Distribution: {}.{}", table, column).cyan().bold()
    );
    println!("{}", "─".repeat(70).dimmed());

    for (label, count) in data {
        let truncated_label = if label.len() > max_label_len {
            format!("{}…", &label[..max_label_len - 1])
        } else {
            label
        };

        let bar_width = (count as f64 / max_count as f64 * bar_max_width as f64).round() as usize;
        let bar = "▓".repeat(bar_width);

        println!(
            "{:>width$} │{} {}",
            truncated_label.white(),
            bar.blue(),
            count,
            width = max_label_len
        );
    }

    println!("{}", "─".repeat(70).dimmed());
    println!();

    Ok(())
}

/// Creates an ASCII sparkline for a sequence of values
pub fn sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    if range.abs() < f64::EPSILON {
        return "▄".repeat(values.len());
    }

    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    values
        .iter()
        .map(|&v| {
            let normalized = (v - min) / range;
            let idx = (normalized * 7.0).round() as usize;
            chars[idx.min(7)]
        })
        .collect()
}
