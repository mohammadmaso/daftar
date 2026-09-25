//! Developer CLI over `daftar_core`. Subcommands are added as their milestone lands;
//! none is exposed before it works.
//!
//! Credentials come from the environment, never from arguments (they would land in shell history):
//! `DAFTAR_GIT_TOKEN` (HTTPS) or `DAFTAR_SSH_KEY_FILE` (path to an OpenSSH private key), and
//! `DAFTAR_API_KEY_<PROVIDER_ID>` for AI providers (id upper-cased, `-` → `_`).

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use daftar_core::agent::Cancel;
use daftar_core::providers::{self, ProviderConfig, Role};
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
    Init {
        path: PathBuf,
        #[arg(long)]
        device: String,
    },
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
    Photo {
        path: PathBuf,
        image: PathBuf,
        #[arg(long)]
        vault: Option<String>,
    },
    /// List today's (or DATE's, YYYY-MM-DD) captures.
    Today {
        path: PathBuf,
        #[arg(long)]
        date: Option<String>,
    },
    /// Local sync status.
    Status { path: PathBuf },
    /// Commit, fetch, integrate and push.
    Sync { path: PathBuf },
    /// Run queued AI jobs (transcribe, describe, ingest) until the queue is empty or blocked.
    Jobs { path: PathBuf },
    /// Manage AI providers in the shared config (keys come from the environment).
    Provider {
        path: PathBuf,
        #[command(subcommand)]
        action: ProviderAction,
    },
    /// Point a model role at a provider and model, or clear it.
    Role {
        path: PathBuf,
        /// router, ingest, chat, voice, vision, reflect, lint, stt, tts, embedding
        role: String,
        provider: Option<String>,
        model: Option<String>,
        #[arg(long)]
        clear: bool,
    },
    /// Make one real minimal call for a role and report its latency.
    Test { path: PathBuf, role: String },
    /// Search the wiki (Persian-aware).
    Search {
        path: PathBuf,
        query: String,
        #[arg(long)]
        vault: Vec<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Show a page's properties and backlinks.
    Page { path: PathBuf, page: String },
    /// Pages around PAGE within DEPTH links (1–2).
    Graph {
        path: PathBuf,
        page: String,
        #[arg(long, default_value_t = 1)]
        depth: usize,
    },
    /// Rebuild the search and link cache from the files.
    Reindex { path: PathBuf },
    /// Replace PAGE with the contents of FILE as a human edit (one `edit:` commit).
    Edit {
        path: PathBuf,
        page: String,
        file: PathBuf,
    },
    /// AI operations, newest first.
    Activity {
        path: PathBuf,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// The page changes of one operation.
    Diff { path: PathBuf, op: String },
    /// Undo an operation (revert, or a queued compensating op). Run `jobs` to complete a queued one.
    Undo { path: PathBuf, op: String },
    /// Undo an ingest and file the capture again into VAULT.
    Move {
        path: PathBuf,
        op: String,
        vault: String,
    },
    /// Undo an ingest and file the capture again with a correction.
    Rerun {
        path: PathBuf,
        op: String,
        note: String,
    },
    /// File an excluded capture again.
    Include { path: PathBuf, raw_id: String },
    /// List Review cards, or resolve one: confirm [--text T], reject, dismiss.
    Review {
        path: PathBuf,
        id: Option<String>,
        action: Option<String>,
        #[arg(long)]
        text: Option<String>,
    },
    /// Generate an ed25519 key pair; prints the public key, writes the private key to FILE.
    Keygen {
        file: PathBuf,
        #[arg(long, default_value = "daftar")]
        comment: String,
    },
}

#[derive(Subcommand)]
enum ProviderAction {
    /// List providers and model roles.
    List,
    /// Add a provider; prints its id.
    Add {
        #[arg(long)]
        name: String,
        /// openai_compatible, anthropic or gemini
        #[arg(long, default_value = "openai_compatible")]
        kind: String,
        #[arg(long, default_value = "")]
        base_url: String,
    },
    /// Remove a provider and the roles that used it.
    Remove { id: String },
}

fn parse_enum<T: serde::de::DeserializeOwned>(s: &str, what: &str) -> anyhow::Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .with_context(|| format!("unknown {what}: {s}"))
}

/// Provider API keys from `DAFTAR_API_KEY_<ID>`.
fn keys_from_env(s: &Session) -> anyhow::Result<HashMap<String, String>> {
    let cfg = s.library().config()?;
    Ok(cfg
        .ai
        .providers
        .iter()
        .filter_map(|p| {
            let var = format!("DAFTAR_API_KEY_{}", p.id.to_uppercase().replace('-', "_"));
            std::env::var(var).ok().map(|k| (p.id.clone(), k))
        })
        .collect())
}

fn runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

fn auth_from_env() -> anyhow::Result<GitAuth> {
    if let Ok(token) = std::env::var("DAFTAR_GIT_TOKEN") {
        return Ok(GitAuth::Token {
            username: std::env::var("DAFTAR_GIT_USER").ok(),
            token,
        });
    }
    if let Ok(file) = std::env::var("DAFTAR_SSH_KEY_FILE") {
        let private_key =
            std::fs::read_to_string(&file).with_context(|| format!("reading {file}"))?;
        return Ok(GitAuth::SshKey {
            private_key,
            passphrase: std::env::var("DAFTAR_SSH_PASSPHRASE").ok(),
        });
    }
    Ok(GitAuth::None)
}

fn print<T: serde::Serialize>(
    json: bool,
    value: &T,
    text: impl FnOnce(&T) -> String,
) -> anyhow::Result<()> {
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
            print(json, &info, |i| {
                format!(
                    "{} core {} (repo schema v{}, {})",
                    i.app_name, i.version, i.repo_schema_version, i.target
                )
            })?;
        }
        Command::Init { path, device } => {
            let lib = sync::init_local(&path, "main")?;
            lib.set_device(&device, std::env::consts::OS, &now())?;
            println!("initialised {}", path.display());
        }
        Command::Clone {
            url,
            path,
            device,
            branch,
        } => {
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
                    daftar_core::layout::Date {
                        year: parsed.year() as i32,
                        month: parsed.month() as u8,
                        day: parsed.day() as u8,
                    }
                }
                None => daftar_core::time::date_of(&now()),
            };
            let items = s.day(d)?;
            print(json, &items, |items| {
                items
                    .iter()
                    .map(|c| {
                        format!(
                            "{}  {:<6} {:<8?}  {}",
                            &c.captured_at[11..16],
                            c.kind.as_str(),
                            c.stage,
                            c.text.lines().next().unwrap_or("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
        }
        Command::Status { path } => {
            let s = Session::open(path)?;
            let st = s.status()?;
            print(json, &st, |st| {
                format!(
                    "branch {} · {} uncommitted · {} unpushed · remote: {}",
                    st.branch,
                    st.uncommitted,
                    st.unpushed,
                    if st.has_remote { "yes" } else { "no" }
                )
            })?;
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
                    o.message
                        .as_deref()
                        .map(|m| format!(" — {m}"))
                        .unwrap_or_default()
                )
            })?;
        }
        Command::Jobs { path } => {
            let s = Session::open(path)?;
            let rt = s.runtime(keys_from_env(&s)?)?;
            let reports = runtime()?.block_on(s.run_jobs(&rt, true, &Cancel::default()))?;
            print(json, &reports, |rs| {
                if rs.is_empty() {
                    return "nothing to do".into();
                }
                rs.iter()
                    .map(|r| {
                        format!(
                            "{:<10} {:<8?} {}",
                            r.kind.as_str(),
                            r.state,
                            r.message.as_deref().unwrap_or("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
        }
        Command::Provider { path, action } => {
            let s = Session::open(path)?;
            match action {
                ProviderAction::List => {
                    let ai = s.library().config()?.ai;
                    print(json, &ai, |ai| {
                        let mut out: Vec<String> = ai
                            .providers
                            .iter()
                            .map(|p| format!("{}  {} ({:?}) {}", p.id, p.name, p.kind, p.base_url))
                            .collect();
                        for r in Role::ALL {
                            if let Some(rc) = ai.role(r) {
                                out.push(format!(
                                    "{:<10} → {} {}{}",
                                    r.as_str(),
                                    rc.provider,
                                    rc.model,
                                    if rc.role == r {
                                        String::new()
                                    } else {
                                        format!(" (uses {})", rc.role.as_str())
                                    }
                                ));
                            }
                        }
                        out.join("\n")
                    })?;
                }
                ProviderAction::Add {
                    name,
                    kind,
                    base_url,
                } => {
                    let cfg = ProviderConfig {
                        id: String::new(),
                        name,
                        kind: parse_enum(&kind, "provider kind")?,
                        base_url,
                        extra_headers: Default::default(),
                        timeout_s: 60,
                    };
                    let id = s.library().update_config(|c| c.ai.upsert_provider(cfg))?;
                    print(json, &id, |id| id.clone())?;
                }
                ProviderAction::Remove { id } => {
                    s.library().update_config(|c| c.ai.remove_provider(&id))?;
                }
            }
        }
        Command::Role {
            path,
            role,
            provider,
            model,
            clear,
        } => {
            let s = Session::open(path)?;
            let role: Role = parse_enum(&role, "role")?;
            if clear {
                s.library().update_config(|c| c.ai.clear_role(role))?;
            } else {
                let (Some(p), Some(m)) = (provider, model) else {
                    bail!("give PROVIDER and MODEL, or --clear");
                };
                let warning = s.library().update_config(|c| {
                    if c.ai.provider(&p).is_none() {
                        bail!("no provider {p}");
                    }
                    c.ai.set_role(role, &p, &m);
                    Ok(providers::capability_warning(role, &m))
                })??;
                if let Some(w) = warning {
                    eprintln!("warning: {w}");
                }
            }
        }
        Command::Test { path, role } => {
            let s = Session::open(path)?;
            let role: Role = parse_enum(&role, "role")?;
            let rt = s.runtime(keys_from_env(&s)?)?;
            let (p, rc) = rt.for_role(role)?;
            let r = runtime()?.block_on(providers::probe(&p, &rc))?;
            print(json, &r, |r| {
                format!("ok · {} ms · {}", r.latency_ms, r.detail)
            })?;
        }
        Command::Search {
            path,
            query,
            vault,
            limit,
        } => {
            let s = Session::open(path)?;
            let hits = s.search(&query, &vault, limit)?;
            print(json, &hits, |hs| {
                hs.iter()
                    .map(|h| {
                        format!(
                            "{}  {} · {}\n    {}",
                            h.path, h.title_en, h.title_fa, h.snippet
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
        }
        Command::Page { path, page } => {
            let s = Session::open(path)?;
            let v = s.page(&page)?;
            print(json, &v, |v| {
                let mut out = format!(
                    "{} · {}\n{} · {} · updated {} · {} sources\n",
                    v.meta.title.en,
                    v.meta.title.fa,
                    v.meta.kind,
                    v.meta.vault,
                    v.meta.updated,
                    v.meta.sources.len()
                );
                out.push_str("backlinks:\n");
                for b in &v.backlinks {
                    out.push_str(&format!("  {}\n", b.path));
                }
                out
            })?;
        }
        Command::Graph { path, page, depth } => {
            let s = Session::open(path)?;
            let g = s.local_graph(&page, depth)?;
            print(json, &g, |g| {
                g.nodes
                    .iter()
                    .map(|n| format!("{}{}", "  ".repeat(n.depth), n.page.path))
                    .chain(g.edges.iter().map(|(a, b)| format!("{a} → {b}")))
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
        }
        Command::Reindex { path } => {
            let s = Session::open(path)?;
            let n = s.rebuild_index()?;
            print(json, &n, |n| format!("indexed {n} pages"))?;
        }
        Command::Edit { path, page, file } => {
            let s = Session::open(path)?;
            let base = s.page(&page).map(|v| v.hash).unwrap_or_default();
            let text = std::fs::read_to_string(&file)?;
            let out = s.save_page(&page, &base, &text)?;
            print(json, &out, |o| {
                if o.committed {
                    "committed".into()
                } else {
                    "unchanged".into()
                }
            })?;
        }
        Command::Activity { path, limit } => {
            let s = Session::open(path)?;
            let ops = s.activity(limit, None)?;
            print(json, &ops, |ops| {
                ops.iter()
                    .map(|o| {
                        format!(
                            "{}  {:<10} {}{}",
                            o.op_id,
                            o.op_type.as_str(),
                            o.summary,
                            if o.reverted { "  [undone]" } else { "" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
        }
        Command::Diff { path, op } => {
            let s = Session::open(path)?;
            let files = s.op_diff(&op)?;
            print(json, &files, |fs| {
                let mut out = String::new();
                for f in fs {
                    out.push_str(&format!("--- {} ({:?})\n", f.path, f.change));
                    for l in &f.lines {
                        use daftar_core::audit::LineKind::*;
                        let p = match l.kind {
                            Added => "+",
                            Removed => "-",
                            Context => " ",
                            Gap => "…",
                        };
                        out.push_str(&format!("{p}{}\n", l.text));
                    }
                }
                out
            })?;
        }
        Command::Undo { path, op } => {
            let s = Session::open(path)?;
            let st = s.undo(&op, None)?;
            print(json, &st, |st| format!("{st:?}"))?;
        }
        Command::Move { path, op, vault } => {
            let s = Session::open(path)?;
            let opts = daftar_core::ops::IngestOptions {
                forced_vault: Some(vault),
                ..Default::default()
            };
            let st = s.undo(&op, Some(opts))?;
            print(json, &st, |st| {
                format!("{st:?}; run `jobs` to file it again")
            })?;
        }
        Command::Rerun { path, op, note } => {
            let s = Session::open(path)?;
            let opts = daftar_core::ops::IngestOptions {
                note: Some(note),
                ..Default::default()
            };
            let st = s.undo(&op, Some(opts))?;
            print(json, &st, |st| {
                format!("{st:?}; run `jobs` to file it again")
            })?;
        }
        Command::Include { path, raw_id } => {
            let s = Session::open(path)?;
            s.include(&raw_id)?;
        }
        Command::Review {
            path,
            id,
            action,
            text,
        } => {
            use daftar_core::review_ops::Resolution;
            let s = Session::open(path)?;
            match (id, action.as_deref()) {
                (None, _) => {
                    let cards = s.review_cards()?;
                    print(json, &cards, |cs| {
                        cs.iter()
                            .map(|c| {
                                format!(
                                    "{}  {:?}  {}",
                                    c.item.id,
                                    c.item.kind,
                                    c.claim
                                        .as_ref()
                                        .map(|x| x.text.clone())
                                        .unwrap_or_else(|| c.item.payload.to_string())
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })?;
                }
                (Some(id), Some(a)) => {
                    let r = match a {
                        "confirm" => Resolution::ConfirmClaim { text },
                        "reject" => Resolution::RejectClaim,
                        "dismiss" => Resolution::Dismiss,
                        other => bail!("unknown action {other}: confirm, reject or dismiss"),
                    };
                    let op = s.resolve_review(&id, r)?;
                    print(json, &op, |o| o.clone())?;
                }
                (Some(_), None) => bail!("give an action: confirm, reject or dismiss"),
            }
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
