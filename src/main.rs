use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

use runequest::auth::{AuthMode, JwtManager, UserRole, UserStore};
use runequest::storage::shop_store::ShopStore;
use runequest::llm::client::XaiClient;
use runequest::web::presence::PresenceRegistry;

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

        #[arg(long, default_value = "2998", env = "RUNEQUEST_API_PORT")]
        api_port: u16,

        #[arg(long, default_value = "0.0.0.0", env = "RUNEQUEST_BIND_ADDR")]
        bind_address: String,

        #[arg(long, env = "RUNEQUEST_DATA_DIR")]
        data_dir: Option<PathBuf>,

        #[arg(long, env = "RUNEQUEST_REQUIRE_AUTH")]
        require_auth: bool,

        #[arg(long, default_value = "2997", env = "RUNEQUEST_WIKI_PORT")]
        wiki_port: u16,

        #[arg(long, default_value = "0.0.0.0", env = "RUNEQUEST_WIKI_BIND_ADDR")]
        wiki_bind_address: String,

        #[arg(long, default_value = "wiki", env = "RUNEQUEST_WIKI_DIR")]
        wiki_dir: PathBuf,

        /// Path to TLS certificate file (PEM format)
        #[arg(long, env = "RUNEQUEST_TLS_CERT")]
        tls_cert: Option<PathBuf>,

        /// Path to TLS private key file (PEM format)
        #[arg(long, env = "RUNEQUEST_TLS_KEY")]
        tls_key: Option<PathBuf>,
    },

    /// Run battle simulations
    Simulate {
        /// Run full class x enemy sweep
        #[arg(long)]
        sweep: bool,

        /// Run consumable impact analysis
        #[arg(long)]
        consumables: bool,

        /// Run armor slot impact analysis
        #[arg(long)]
        armor_slots: bool,

        /// Run combined gear+consumables analysis
        #[arg(long)]
        combined: bool,

        /// Print stat tables
        #[arg(long)]
        stats: bool,

        /// Run party balance report
        #[arg(long)]
        party_report: bool,

        /// Single class report
        #[arg(long)]
        class: Option<String>,

        /// Number of trials per matchup
        #[arg(long, default_value = "1000")]
        trials: u32,
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
            api_port,
            bind_address,
            data_dir,
            require_auth,
            wiki_port,
            wiki_bind_address,
            wiki_dir,
            tls_cert,
            tls_key,
        } => {
            let base_path = resolve_data_dir(data_dir.as_deref())?;

            // TLS configuration
            let tls_config = match (&tls_cert, &tls_key) {
                (Some(cert), Some(key)) => {
                    let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
                    eprintln!("TLS enabled with cert: {}", cert.display());
                    Some(config)
                }
                (None, None) => {
                    eprintln!("TLS disabled (no --tls-cert/--tls-key provided)");
                    None
                }
                _ => anyhow::bail!("Both --tls-cert and --tls-key must be provided together"),
            };

            // Shared resources created once
            let api_key = std::env::var("XAI_API_KEY").map_err(|_| {
                anyhow::anyhow!(
                    "XAI_API_KEY not set. Create a .env file with XAI_API_KEY=your_key"
                )
            })?;
            let model =
                std::env::var("XAI_MODEL").unwrap_or_else(|_| "grok-4-1-fast-reasoning".to_string());
            let xai_client = Arc::new(XaiClient::new(&api_key, &model));

            let user_store = Arc::new(UserStore::new(&base_path));
            let jwt_manager = Arc::new(JwtManager::new(&base_path)?);

            let shop_store = ShopStore::new(&base_path);
            let presence = PresenceRegistry::new();

            let auth_mode = if require_auth || user_store.has_users() {
                AuthMode::Enabled
            } else {
                AuthMode::Disabled
            };

            let shop_store_web = shop_store.clone();
            let shop_store_api = shop_store.clone();
            let presence_web = presence.clone();
            let presence_api = presence.clone();

            let bind_addr_web = bind_address.clone();
            let bind_addr_api = bind_address.clone();
            let data_dir_web = base_path.clone();
            let data_dir_api = base_path.clone();
            let xai_api = xai_client.clone();
            let user_store_api = user_store.clone();
            let jwt_api = jwt_manager.clone();
            let model_api = model.clone();
            let auth_mode_api = auth_mode.clone();

            let tls_web = tls_config.clone();
            let web_handle = tokio::spawn(async move {
                runequest::web::run_server(port, &bind_addr_web, data_dir_web, require_auth, shop_store_web, presence_web, tls_web).await
            });

            let tls_api = tls_config.clone();
            let api_handle = tokio::spawn(async move {
                runequest::web::api_server::run_api_server(
                    api_port,
                    &bind_addr_api,
                    data_dir_api,
                    xai_api,
                    model_api,
                    auth_mode_api,
                    user_store_api,
                    jwt_api,
                    shop_store_api,
                    presence_api,
                    tls_api,
                )
                .await
            });

            let tls_wiki = tls_config.clone();
            let wiki_handle = tokio::spawn(async move {
                runequest::web::wiki_server::run_wiki_server(
                    wiki_port,
                    &wiki_bind_address,
                    wiki_dir,
                    tls_wiki,
                )
                .await
            });

            tokio::select! {
                result = web_handle => {
                    match result {
                        Ok(Ok(())) => eprintln!("Web server exited"),
                        Ok(Err(e)) => eprintln!("Web server error: {}", e),
                        Err(e) => eprintln!("Web server task error: {}", e),
                    }
                }
                result = api_handle => {
                    match result {
                        Ok(Ok(())) => eprintln!("API server exited"),
                        Ok(Err(e)) => eprintln!("API server error: {}", e),
                        Err(e) => eprintln!("API server task error: {}", e),
                    }
                }
                result = wiki_handle => {
                    match result {
                        Ok(Ok(())) => eprintln!("Wiki server exited"),
                        Ok(Err(e)) => eprintln!("Wiki server error: {}", e),
                        Err(e) => eprintln!("Wiki server task error: {}", e),
                    }
                }
            }
        }

        Commands::Simulate {
            sweep, consumables, armor_slots, combined, stats, party_report,
            class, trials,
        } => {
            use runequest::engine::simulator;

            if stats {
                print!("{}", simulator::stat_table_report());
            }
            if sweep {
                print!("{}", simulator::sweep_report(trials));
            }
            if consumables {
                print!("{}", simulator::consumable_report(trials));
            }
            if armor_slots {
                print!("{}", simulator::armor_slot_report(trials));
            }
            if combined {
                print!("{}", simulator::combined_report(trials));
            }
            if party_report {
                print!("{}", simulator::party_report(trials));
            }
            if let Some(ref name) = class {
                if let Some(c) = simulator::SimClass::from_str(name) {
                    print!("{}", simulator::class_report(c, trials));
                } else {
                    eprintln!("Unknown class: {}. Options: warrior, berserker, paladin, rogue, ranger, monk, mage, warlock, cleric, bard", name);
                }
            }
            if !sweep && !consumables && !armor_slots && !combined && !stats && !party_report && class.is_none() {
                // Default: run everything
                print!("{}", simulator::sweep_report(trials));
                print!("{}", simulator::consumable_report(trials));
                print!("{}", simulator::armor_slot_report(trials));
            }
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
