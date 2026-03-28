use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use runequest::auth::{UserRole, UserStore};

#[derive(Parser)]
#[command(name = "runequest")]
#[command(about = "D&D Adventure Chat - Grok-powered storytelling with OSRS style")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server
    Serve {
        #[arg(long, short, default_value = "2999", env = "RUNEQUEST_PORT")]
        port: u16,

        #[arg(long, default_value = "0.0.0.0", env = "RUNEQUEST_BIND_ADDR")]
        bind_address: String,

        #[arg(long, env = "RUNEQUEST_DATA_DIR")]
        data_dir: Option<PathBuf>,

        #[arg(long, env = "RUNEQUEST_REQUIRE_AUTH")]
        require_auth: bool,
    },

    /// Manage users
    User {
        #[arg(long, env = "RUNEQUEST_DATA_DIR")]
        data_dir: Option<PathBuf>,

        #[command(subcommand)]
        action: UserAction,
    },
}

#[derive(Subcommand)]
enum UserAction {
    /// Create a new user
    Create {
        username: String,
        #[arg(long)]
        admin: bool,
    },
    /// List all users
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            port,
            bind_address,
            data_dir,
            require_auth,
        } => {
            let base_path = resolve_data_dir(data_dir.as_deref())?;
            runequest::web::run_server(port, &bind_address, base_path, require_auth).await?;
        }

        Commands::User { data_dir, action } => {
            let base_path = resolve_data_dir(data_dir.as_deref())?;
            cmd_user(&base_path, action)?;
        }
    }

    Ok(())
}

fn resolve_data_dir(data_dir: Option<&std::path::Path>) -> Result<PathBuf> {
    match data_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create data directory: {}", dir.display()))?;
            Ok(dir.to_path_buf())
        }
        None => {
            let base = dirs::data_local_dir()
                .context("Could not determine local data directory")?
                .join("runequest");
            std::fs::create_dir_all(&base)?;
            Ok(base)
        }
    }
}

fn cmd_user(base_path: &std::path::Path, action: UserAction) -> Result<()> {
    let store = UserStore::new(base_path);

    match action {
        UserAction::Create { username, admin } => {
            let role = if admin {
                UserRole::Admin
            } else {
                UserRole::User
            };

            let password = rpassword::prompt_password_stderr("Password: ")
                .context("Failed to read password")?;
            let confirm = rpassword::prompt_password_stderr("Confirm password: ")
                .context("Failed to read password confirmation")?;

            if password != confirm {
                anyhow::bail!("Passwords do not match");
            }

            if password.len() < 8 {
                anyhow::bail!("Password must be at least 8 characters");
            }

            store
                .create_user(&username, &password, role)
                .with_context(|| format!("Failed to create user '{}'", username))?;
            println!("Created user '{}'", username);
        }

        UserAction::List => {
            let users = store.list_users().context("Failed to list users")?;
            if users.is_empty() {
                println!("No users found.");
                return Ok(());
            }

            println!("{:<20} {:<8} {}", "USERNAME", "ROLE", "CREATED");
            println!("{}", "-".repeat(55));
            for user in &users {
                println!(
                    "{:<20} {:<8} {}",
                    user.username,
                    user.role,
                    user.created_at.format("%Y-%m-%d %H:%M")
                );
            }
        }
    }

    Ok(())
}
