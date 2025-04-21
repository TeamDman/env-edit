use env_edit::env_reader::get_machine_env_var;
use env_edit::env_reader::list_machine_env_var;
use env_edit::env_writer::set_machine_env_var;
use env_edit::init::init;
use env_edit::win_elevation::ensure_elevated;
use tracing::info;

use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(
    name = "env-edit",
    version,
    about = "Edits machine-level environment variables"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists all machine environment variables
    List,
    /// Shows a single machine environment variable by name
    Show {
        #[arg(long)]
        key: String,
    },
    /// Sets a machine environment variable
    Set {
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: String,
    },
}

fn main() -> eyre::Result<()> {
    init()?;

    // Parse CLI
    let cli = Cli::parse();

    // We only need elevation if we plan to modify registry
    match &cli.command {
        Commands::List | Commands::Show { .. } => {
            // read-only, so we can run without admin rights
        }
        Commands::Set { .. } => {
            // ensure elevated
            ensure_elevated()?;
        }
    };

    match cli.command {
        Commands::List => cmd_list()?,
        Commands::Show { key } => cmd_show(&key)?,
        Commands::Set { key, value } => cmd_set(&key, &value)?,
    }

    info!("Done!");
    wait_for_enter();
    Ok(())
}

fn cmd_list() -> eyre::Result<()> {
    let environment_variables = list_machine_env_var()?;
    let dump = serde_json::to_string_pretty(&environment_variables)?;
    println!("{dump}");
    Ok(())
}

fn cmd_show(key_name: &str) -> eyre::Result<()> {
    match get_machine_env_var(key_name)? {
        Some(value) => {
            println!("{} = {}", key_name, value);
        }
        None => {
            println!("{} is not set.", key_name);
        }
    }
    Ok(())
}

fn cmd_set(key_name: &str, value: &str) -> eyre::Result<()> {
    // Because we've already done ensure_elevated(), we are definitely admin by now
    set_machine_env_var(key_name, value)?;
    info!("Set {key_name} to {value}");
    Ok(())
}

/// Waits for the user to press Enter.
pub fn wait_for_enter() {
    eprint!("Press Enter to exit...");
    std::io::Write::flush(&mut std::io::stdout()).unwrap(); // Ensure the prompt is displayed immediately
    let _ = std::io::stdin().read_line(&mut String::new()); // Wait for user input
}
