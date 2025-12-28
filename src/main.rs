//! dbview - A human-readable CLI interface for understanding SQLite database files
//!
//! This tool provides intuitive commands to explore, summarize, and visualize
//! the contents of SQLite databases without requiring SQL expertise.

mod db;
mod schema;
mod stats;
mod viz;
mod display;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

/// A human-readable CLI interface for understanding SQLite database files
#[derive(Parser)]
#[command(name = "dbview")]
#[command(version = "0.1.0")]
#[command(about = "Explore and understand SQLite databases without SQL knowledge")]
#[command(long_about = None)]
struct Cli {
    /// Path to the SQLite database file
    #[arg(value_name = "DATABASE")]
    database: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all tables in the database with row counts
    Tables,

    /// Show the schema/structure of a table
    Schema {
        /// Name of the table to inspect
        table: String,
    },

    /// View records from a table in a formatted display
    View {
        /// Name of the table to view
        table: String,

        /// Maximum number of rows to display
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Number of rows to skip
        #[arg(short, long, default_value = "0")]
        offset: usize,
    },

    /// Show statistics for table columns
    Stats {
        /// Name of the table to analyze
        table: String,
    },

    /// Get a human-readable description of what a table contains
    Describe {
        /// Name of the table to describe
        table: String,
    },

    /// Display an ASCII histogram for a numeric column
    Histogram {
        /// Name of the table
        table: String,

        /// Name of the column to visualize
        column: String,

        /// Number of bins for the histogram
        #[arg(short, long, default_value = "10")]
        bins: usize,
    },

    /// Show a summary overview of the entire database
    Summary,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if database file exists
    if !cli.database.exists() {
        eprintln!(
            "{} Database file not found: {}",
            "Error:".red().bold(),
            cli.database.display()
        );
        std::process::exit(1);
    }

    // Connect to the database
    let conn = db::connect(&cli.database)?;

    // Execute the requested command
    match cli.command {
        Commands::Tables => {
            display::show_tables(&conn)?;
        }
        Commands::Schema { table } => {
            display::show_schema(&conn, &table)?;
        }
        Commands::View { table, limit, offset } => {
            display::show_records(&conn, &table, limit, offset)?;
        }
        Commands::Stats { table } => {
            display::show_stats(&conn, &table)?;
        }
        Commands::Describe { table } => {
            display::describe_table(&conn, &table)?;
        }
        Commands::Histogram { table, column, bins } => {
            viz::show_histogram(&conn, &table, &column, bins)?;
        }
        Commands::Summary => {
            display::show_summary(&conn)?;
        }
    }

    Ok(())
}
