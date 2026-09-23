//! Developer CLI over `daftar_core`. Subcommands are added as their milestone lands;
//! none is exposed before it works.
//!
//! Credentials come from the environment, never from arguments (they would land in shell history):
//! `DAFTAR_GIT_TOKEN` (HTTPS) or `DAFTAR_SSH_KEY_FILE` (path to an OpenSSH private key).

use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use daftar_core::session::Session;
use daftar_core::sync::{self, GitAuth};

#[derive(Parser)]
#[command(name = daftar_core::APP_ID, version = daftar_core::VERSION, about = "Daftar core CLI")]
struct Cli {
    /// Emit machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print core build information.
    Info,
    /// Create a new local library.
    Init { path: PathBuf, #[arg(long)] device: String },
    /// Clone (or initialise an empty) remote into PATH.
    Clone {
        url: String,
        path: PathBuf,
        #[arg(long)]
        device: String,
        #[arg(long, default_value = "main")]
        branch: String,
    },
    /// Set the remote of an existing library.
    Remote { path: PathBuf, url: String },
    /// Capture a text note.
    Capture {
        path: PathBuf,
        text: String,
        #[arg(long)]
        vault: Option<String>,
    },
    /// Capture a photo (any common image format).
    Photo { path: PathBuf, image: PathBuf, #[arg(long)] vault: Option<String> },
    /// List today's (or DATE's, YYYY-MM-DD) captures.
    Today { path: PathBuf, #[arg(long)] date: Option<String> },
    /// Local sync status.
    Status { path: PathBuf },
    /// Commit, fetch, integrate and push.
    Sync { path: PathBuf },
    /// Generate an ed25519 key pair; prints the public key, writes the private key to FILE.
    Keygen { file: PathBuf, #[arg(long, default_value = "daftar")] comment: String },
}

fn auth_from_env() -> anyhow::Result<GitAuth> {
    if let Ok(token) = std::env::var("DAFTAR_GIT_TOKEN") {
        return Ok(GitAuth::Token { username: std::env::var("DAFTAR_GIT_USER").ok(), token });
    }
    if let Ok(file) = std::env::var("DAFTAR_SSH_KEY_FILE") {
        let private_key = std::fs::read_to_string(&file).with_context(|| format!("reading {file}"))?;
        return Ok(GitAuth::SshKey { private_key, passphrase: std::env::var("DAFTAR_SSH_PASSPHRASE").ok() });
    }
    Ok(GitAuth::None)
}

fn print<T: serde::Serialize>(json: bool, value: &T, text: impl FnOnce(&T) -> String) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", text(value));
    }
    Ok(())
}

fn now() -> jiff::Zoned {
    jiff::Zoned::now()
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let json = cli.json;
    match cli.command {
        Command::Info => {
            let info = daftar_core::core_info();
            print(json, &info, |i| format!("{} core {} (repo schema v{}, {})", i.app_name, i.version, i.repo_schema_version, i.target))?;
        }
        Command::Init { path, device } => {
            let lib = sync::init_local(&path, "main")?;
            lib.set_device(&device, std::env::consts::OS, &now())?;
            println!("initialised {}", path.display());
        }
        Command::Clone { url, path, device, branch } => {
            let lib = sync::clone(&url, &path, &auth_from_env()?, &branch)?;
            lib.set_device(&device, std::env::consts::OS, &now())?;
            println!("cloned into {}", path.display());
        }
        Command::Remote { path, url } => {
            let s = Session::open(path)?;
            sync::set_remote(s.library(), &url)?;
        }
        Command::Capture { path, text, vault } => {
            let s = Session::open(path)?;
            let item = s.capture_text(&text, vault, &now())?;
            print(json, &item.path, |p| p.clone())?;
        }
        Command::Photo { path, image, vault } => {
            let s = Session::open(path)?;
            let bytes = std::fs::read(&image)?;
            let item = s.capture_photo(&bytes, None, vault, &now())?;
            print(json, &item.path, |p| p.clone())?;
        }
        Command::Today { path, date } => {
            let s = Session::open(path)?;
            let d = match date {
                Some(d) => {
                    let parsed: jiff::civil::Date = d.parse().context("date must be YYYY-MM-DD")?;
                    daftar_core::layout::Date { year: parsed.year() as i32, month: parsed.month() as u8, day: parsed.day() as u8 }
                }
                None => daftar_core::time::date_of(&now()),
            };
            let items = s.day(d)?;
            print(json, &items, |items| {
                items
                    .iter()
                    .map(|c| format!("{}  {:<6} {:<8?}  {}", &c.captured_at[11..16], c.kind.as_str(), c.stage, c.text.lines().next().unwrap_or("")))
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
        }
        Command::Status { path } => {
            let s = Session::open(path)?;
            let st = s.status()?;
            print(json, &st, |st| format!("branch {} · {} uncommitted · {} unpushed · remote: {}", st.branch, st.uncommitted, st.unpushed, if st.has_remote { "yes" } else { "no" }))?;
        }
        Command::Sync { path } => {
            let s = Session::open(path)?;
            let o = s.sync(&auth_from_env()?, &now())?;
            print(json, &o, |o| {
                format!(
                    "{:?}: committed {}, pulled {}, pushed {}, conflicts {}, replays {}{}",
                    o.state,
                    o.committed,
                    o.pulled,
                    o.pushed,
                    o.conflicts.len(),
                    o.replays.len(),
                    o.message.as_deref().map(|m| format!(" — {m}")).unwrap_or_default()
                )
            })?;
        }
        Command::Keygen { file, comment } => {
            if file.exists() {
                bail!("{} already exists", file.display());
            }
            let k = daftar_core::keys::generate_ed25519(&comment)?;
            std::fs::write(&file, &k.private_openssh)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
            }
            println!("{}", k.public_openssh);
        }
    }
    Ok(())
}
