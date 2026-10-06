use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use praxilume_artcraft_bridge::ControlEndpoint;
use praxilume_core::{Actor, CommandEnvelope, Id, Operation, Project};
use praxilume_runtime::ProjectStore;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "praxilume",
    version,
    about = "Structured creative engine + agent control plane"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        path: PathBuf,
        #[arg(long, default_value = "Untitled")]
        name: String,
    },
    Inspect {
        path: PathBuf,
    },
    Rename {
        path: PathBuf,
        #[arg(long)]
        name: String,
    },
    Import {
        path: PathBuf,
        source: PathBuf,
        #[arg(long, default_value = "bin")]
        extension: String,
    },
    Command {
        path: PathBuf,
        #[arg(long)]
        command: String,
        #[arg(long, default_value = "user")]
        actor: String,
        #[arg(long, default_value = "{}")]
        params: String,
        #[arg(long)]
        dry_run: bool,
    },
    Artcraft {
        port: u16,
        #[arg(long)]
        token: String,
        #[arg(long)]
        command: String,
        #[arg(long, default_value = "creative bridge call")]
        intent: String,
        #[arg(long, default_value = "{}")]
        params: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Init { path, name } => init(path, name),
        Command::Inspect { path } => inspect(path),
        Command::Rename { path, name } => rename(path, name),
        Command::Import {
            path,
            source,
            extension,
        } => import_asset(path, source, extension),
        Command::Command {
            path,
            command,
            actor,
            params,
            dry_run,
        } => run_command(path, command, actor, params, dry_run),
        Command::Artcraft {
            port,
            token,
            command,
            intent,
            params,
        } => bridge(port, &token, command, intent, params),
    }
}

fn init(path: PathBuf, name: String) -> Result<()> {
    let id = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    ProjectStore::create(&path, &Project::new(id, name))?;
    println!("created {}", path.display());
    Ok(())
}

fn inspect(path: PathBuf) -> Result<()> {
    let project = ProjectStore::open(path)?.load()?;
    println!(
        "{} v{} revision {}",
        project.name, project.schema_version, project.revision
    );
    println!(
        "objects: {}  assets: {}  history: {}",
        project.objects.len(),
        project.assets.len(),
        project.history.len()
    );
    Ok(())
}

fn rename(path: PathBuf, name: String) -> Result<()> {
    run_command(
        path,
        "project.rename".to_owned(),
        "user".to_owned(),
        serde_json::json!({ "name": name }).to_string(),
        false,
    )
}

fn run_command(
    path: PathBuf,
    command: String,
    actor: String,
    params: String,
    dry_run: bool,
) -> Result<()> {
    let store = ProjectStore::open(&path)?;
    let mut project = store.load()?;
    let actor = parse_actor(&actor)?;
    let params: serde_json::Value =
        serde_json::from_str(&params).context("params must be valid JSON")?;
    let mut envelope = CommandEnvelope::new(
        format!("cmd-{}", project.revision + 1),
        actor,
        "creative project command",
        command,
        params,
    );
    envelope.dry_run = dry_run;

    if dry_run {
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    match envelope.command.as_str() {
        "project.rename" => {
            let name = envelope
                .params
                .get("name")
                .and_then(|value| value.as_str())
                .context("project.rename requires {\"name\": string}")?;
            project.apply(&Operation::SetMetadata {
                key: "display_name".into(),
                value: name.into(),
            })?;
            project.name = name.into();
            store.save(&project)?;
        }
        command => {
            anyhow::bail!(
                "command '{}' is not implemented in core; route it to a capability adapter",
                command
            )
        }
    }

    println!("applied {}", envelope.command);
    Ok(())
}

fn parse_actor(value: &str) -> Result<Actor> {
    match value {
        "user" => Ok(Actor::User),
        "system" => Ok(Actor::System),
        value if value.starts_with("agent:") && value.len() > 6 => Ok(Actor::Agent {
            agent_id: Id::new(&value[6..]),
        }),
        value if value.starts_with("automation:") && value.len() > 11 => Ok(Actor::Automation {
            workflow_id: Id::new(&value[11..]),
        }),
        _ => anyhow::bail!("unknown actor '{value}'"),
    }
}

fn import_asset(path: PathBuf, source: PathBuf, extension: String) -> Result<()> {
    let store = ProjectStore::open(&path)?;
    let mut project = store.load()?;
    let (sha, target) = store.import_asset(&source, &extension)?;
    let logical_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("asset")
        .to_owned();
    let asset = praxilume_core::AssetRecord {
        id: Id::new(format!("asset-{}", &sha[..16])),
        logical_name,
        media_type: guess_media_type(&extension),
        sha256: sha.clone(),
        source: praxilume_core::AssetSource::LocalFile {
            path: source.display().to_string(),
        },
        license: praxilume_core::LicenseInfo {
            spdx: None,
            attribution: None,
            restrictions: Vec::new(),
        },
        tags: Default::default(),
    };
    project.apply(&Operation::RegisterAsset(asset))?;
    store.save(&project)?;
    println!("sha256={sha}\nstored={}", target.display());
    Ok(())
}

fn guess_media_type(extension: &str) -> String {
    match extension.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
    .to_owned()
}

fn bridge(port: u16, token: &str, command: String, intent: String, params: String) -> Result<()> {
    let params: serde_json::Value =
        serde_json::from_str(&params).context("params must be valid JSON")?;
    let envelope = CommandEnvelope::new("bridge-1", Actor::User, intent, command, params);
    let response = ControlEndpoint::localhost(port).send(&envelope, Some(token))?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
