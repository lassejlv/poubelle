use anyhow::{Context, Result};
use clap::Parser;
use poubelle_sdk::{PoubelleClient, Row, Value};
use prettytable::{Cell, Row as TableRow, Table};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

#[derive(Parser, Debug)]
#[command(name = "poubelle-cli")]
#[command(about = "A psql-like CLI for Poubelle DB", long_about = None)]
struct Args {
    /// Connection string (e.g., poubelle://admin:admin@localhost:5432)
    #[arg(
        short = 'c',
        long,
        default_value = "poubelle://admin:admin@127.0.0.1:5432"
    )]
    connection: String,

    /// Execute a single command and exit
    #[arg(short = 'e', long)]
    command: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut client =
        PoubelleClient::new(&args.connection).context("Failed to parse connection string")?;

    println!("Connecting to Poubelle DB...");
    client.connect().await.context("Failed to connect")?;
    println!("Connected to Poubelle DB");
    println!("Type 'help' for help, '\\q' to quit\n");

    // If command is provided, execute it and exit
    if let Some(cmd) = args.command {
        execute_command(&mut client, &cmd).await?;
        client.close().await?;
        return Ok(());
    }

    // Interactive REPL
    let mut rl = DefaultEditor::new()?;
    let history_file = dirs::home_dir()
        .map(|mut p| {
            p.push(".poubelle_history");
            p
        })
        .unwrap_or_default();

    if history_file.exists() {
        let _ = rl.load_history(&history_file);
    }

    let mut multi_line_buffer = String::new();

    loop {
        let prompt = if multi_line_buffer.is_empty() {
            "poubelle> "
        } else {
            "       -> "
        };

        match rl.readline(prompt) {
            Ok(line) => {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(line);

                // Handle meta-commands
                if line.starts_with('\\') {
                    multi_line_buffer.clear();
                    if handle_meta_command(&mut client, line).await? {
                        break;
                    }
                    continue;
                }

                // Handle multi-line SQL
                multi_line_buffer.push_str(line);
                multi_line_buffer.push(' ');

                // Check if statement is complete (ends with semicolon)
                if line.ends_with(';') {
                    let sql = multi_line_buffer.trim_end_matches(';').trim();
                    if !sql.is_empty() {
                        execute_command(&mut client, sql).await?;
                    }
                    multi_line_buffer.clear();
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                multi_line_buffer.clear();
            }
            Err(ReadlineError::Eof) => {
                println!("\\q");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    if !history_file.as_os_str().is_empty() {
        let _ = rl.save_history(&history_file);
    }

    client.close().await?;
    println!("Goodbye!");

    Ok(())
}

async fn execute_command(client: &mut PoubelleClient, sql: &str) -> Result<()> {
    let sql = sql.trim();

    if sql.is_empty() {
        return Ok(());
    }

    // Handle special commands
    if sql.eq_ignore_ascii_case("help") {
        print_help();
        return Ok(());
    }

    // Execute query
    let start = std::time::Instant::now();

    match client.execute(sql).await {
        Ok(rows) => {
            let duration = start.elapsed();

            if rows.is_empty() {
                println!("OK ({}ms)", duration.as_millis());
            } else {
                print_table(&rows);
                println!("\n{} row(s) ({}ms)", rows.len(), duration.as_millis());
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}

async fn handle_meta_command(client: &mut PoubelleClient, cmd: &str) -> Result<bool> {
    match cmd {
        "\\q" | "\\quit" => {
            return Ok(true);
        }
        "\\?" | "\\h" | "\\help" => {
            print_help();
        }
        "\\l" | "\\list" => {
            println!("Listing tables...");
            match client.execute("SELECT * FROM __tables__").await {
                Ok(rows) => {
                    if rows.is_empty() {
                        println!("No tables found");
                    } else {
                        print_table(&rows);
                    }
                }
                Err(_) => {
                    println!("Unable to list tables (meta-table not available)");
                }
            }
        }
        "\\dt" => {
            println!("Listing tables...");
            match client.execute("SELECT * FROM __tables__").await {
                Ok(rows) => {
                    if rows.is_empty() {
                        println!("No tables found");
                    } else {
                        print_table(&rows);
                    }
                }
                Err(_) => {
                    println!("Unable to list tables (meta-table not available)");
                }
            }
        }
        "\\c" => {
            println!("You are connected to Poubelle DB");
        }
        _ => {
            println!("Unknown command: {}", cmd);
            println!("Type '\\?' for help");
        }
    }
    Ok(false)
}

fn print_help() {
    println!("Poubelle CLI Help");
    println!();
    println!("SQL Commands:");
    println!("  CREATE TABLE <name> (<columns>)  Create a new table");
    println!("  INSERT INTO <table> VALUES (...)  Insert data");
    println!("  SELECT * FROM <table>             Query data");
    println!("  End SQL statements with semicolon (;)");
    println!();
    println!("Meta-commands:");
    println!("  \\q, \\quit       Quit the CLI");
    println!("  \\?, \\h, \\help   Show this help");
    println!("  \\l, \\list       List all tables");
    println!("  \\dt             List all tables");
    println!("  \\c              Show connection info");
    println!();
    println!("Shortcuts:");
    println!("  Ctrl+C          Cancel current input");
    println!("  Ctrl+D          Exit (same as \\q)");
    println!("  Up/Down arrows  Navigate command history");
}

fn print_table(rows: &[Row]) {
    if rows.is_empty() {
        return;
    }

    // Collect all unique column names
    let mut columns: Vec<String> = Vec::new();
    for row in rows {
        for key in row.keys() {
            if !columns.contains(key) {
                columns.push(key.clone());
            }
        }
    }
    columns.sort();

    // Create table
    let mut table = Table::new();

    // Add header
    let header: Vec<Cell> = columns.iter().map(|col| Cell::new(col)).collect();
    table.add_row(TableRow::new(header));

    // Add rows
    for row in rows {
        let cells: Vec<Cell> = columns
            .iter()
            .map(|col| {
                let value = row.get(col).unwrap_or(&Value::Null);
                Cell::new(&format_value(value))
            })
            .collect();
        table.add_row(TableRow::new(cells));
    }

    table.printstd();
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        Value::Text(s) => s.clone(),
        Value::Null => "NULL".to_string(),
    }
}
