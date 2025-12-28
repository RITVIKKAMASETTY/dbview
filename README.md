# dbview

A human-readable CLI interface for understanding SQLite database files.

## Overview

**dbview** is a Rust-based command-line tool designed to bridge the gap between opaque database storage and human understanding. Instead of exposing raw SQL queries or requiring database expertise, the tool provides intuitive CLI commands that allow users to **explore, summarize, explain, and visualize** the contents of a database in a human-friendly manner.

## Installation

```bash
# Build from source
cargo build --release

# The binary will be at: target/release/dbview
```

## Usage

```bash
dbview <database.db> <command> [options]
```

### Commands

| Command | Description |
|---------|-------------|
| `tables` | List all tables with row counts |
| `summary` | Overview of the entire database |
| `schema <table>` | Show table structure and columns |
| `view <table>` | Display records in formatted table |
| `stats <table>` | Column statistics (min, max, avg, etc.) |
| `describe <table>` | Human-readable table explanation |
| `histogram <table> <column>` | ASCII histogram for numeric data |

### Examples

```bash
# List all tables
dbview mydata.db tables

# Get database overview
dbview mydata.db summary

# View table schema
dbview mydata.db schema users

# View records with pagination
dbview mydata.db view products --limit 10 --offset 20

# Get column statistics
dbview mydata.db stats orders

# Human-readable description
dbview mydata.db describe users

# Visualize price distribution
dbview mydata.db histogram products price --bins 10
```

## Features

- **📊 Database Inspection** - List tables, columns, data types, and row counts
- **📄 Human-Readable Views** - Display records in clean, formatted tables
- **🔍 Semantic Explanation** - Automatically describe what a table represents
- **📈 Statistics** - Compute meaningful statistics (min, max, avg, counts)
- **📉 CLI Visualization** - ASCII histograms for distributions
- **🔒 Safe Abstraction** - Read-only access, never modifies database

## Tech Stack

- **Language:** Rust
- **CLI:** clap v4
- **Database:** rusqlite
- **Tables:** comfy-table, tabled
- **Colors:** colored
- **Error Handling:** anyhow

## How to Build a Rust Project (vs Node.js)

| Node.js | Rust | Purpose |
|---------|------|---------|
| `npm init` | `cargo init` | Create project |
| `package.json` | `Cargo.toml` | Dependencies file |
| `npm install pkg` | Edit `Cargo.toml` | Add dependencies |
| `index.js` | `src/main.rs` | Entry point |
| `require('./mod')` | `mod modname;` | Import modules |
| `node index.js` | `cargo run` | Run app |
| - | `cargo build --release` | Build binary |

**Key difference:** Rust compiles to a standalone binary - no runtime needed!

## License

MIT
