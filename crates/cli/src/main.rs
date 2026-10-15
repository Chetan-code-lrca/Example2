use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use praxilume_artcraft_bridge::{ControlEndpoint, FilmCraftEndpoint, VectorCraftEndpoint};
use praxilume_core::{Actor, CommandEnvelope, Id, Operation, Project};
use praxilume_image_capability::IMAGE_COMMAND_CATALOG;
use praxilume_runtime::ProjectStore;
use praxilume_vector_capability::VECTOR_COMMAND_CATALOG;
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
    ImageCommands,
    VectorCommands,
    #[command(name = "vectorcraft")]
    VectorCraft {
        port: u16,
        #[arg(long)]
        command: String,
        #[arg(long, default_value = "vector design bridge call")]
        intent: String,
        #[arg(long, default_value = "{}")]
        params: String,
    },
    #[command(
        name = "vectorcraft-open",
        about = "Open a file beneath VectorCraft's configured automation read root"
    )]
    VectorCraftOpen {
        port: u16,
        path: String,
    },
    #[command(
        name = "vectorcraft-export",
        about = "Export beneath VectorCraft's configured automation write root"
    )]
    VectorCraftExport {
        port: u16,
        path: String,
    },
    #[command(
        name = "vectorcraft-commands",
        about = "List commands exposed by a running VectorCraft instance"
    )]
    VectorCraftCommands {
        port: u16,
    },
    #[command(
        name = "vectorcraft-inspect",
        about = "Inspect the active VectorCraft document as structured JSON"
    )]
    VectorCraftInspect {
        port: u16,
    },
    #[command(
        name = "filmcraft",
        about = "Run a command through FilmCraft's localhost control socket"
    )]
    FilmCraft {
        port: u16,
        #[arg(long)]
        command: String,
        #[arg(long, default_value = "{}")]
        params: String,
    },
    #[command(
        name = "filmcraft-commands",
        about = "List commands exposed by a running FilmCraft instance"
    )]
    FilmCraftCommands {
        port: u16,
    },
    #[command(
        name = "filmcraft-sequence",
        about = "Inspect the active FilmCraft sequence as JSON"
    )]
    FilmCraftSequence {
        port: u16,
    },
    Artcraft {
        port: u16,
        #[arg(long, help = "Control token; defaults to PHOTOCRAFT_CONTROL_TOKEN")]
        token: Option<String>,
        #[arg(long)]
        command: String,
        #[arg(long, default_value = "creative bridge call")]
        intent: String,
        #[arg(long, default_value = "{}")]
        params: String,
    },
    #[command(
        name = "photocraft-open",
        about = "Open an image beneath PhotoCraft's configured automation read root"
    )]
    PhotoCraftOpen {
        port: u16,
        path: String,
        #[arg(long, help = "Control token; defaults to PHOTOCRAFT_CONTROL_TOKEN")]
        token: Option<String>,
    },
    #[command(
        name = "photocraft-save",
        about = "Save/export an open document beneath PhotoCraft's configured automation write root"
    )]
    PhotoCraftSave {
        port: u16,
        path: String,
        #[arg(long, help = "Control token; defaults to PHOTOCRAFT_CONTROL_TOKEN")]
        token: Option<String>,
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
        Command::ImageCommands => list_image_commands(),
        Command::VectorCommands => list_vector_commands(),
        Command::VectorCraft {
            port,
            command,
            intent,
            params,
        } => vectorcraft(port, command, intent, params),
        Command::VectorCraftOpen { port, path } => {
            let response = VectorCraftEndpoint::localhost(port).open_document(&path)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Command::VectorCraftExport { port, path } => {
            let response = VectorCraftEndpoint::localhost(port).export_document(&path)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Command::VectorCraftCommands { port } => {
            let response = VectorCraftEndpoint::localhost(port).list_commands()?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Command::VectorCraftInspect { port } => {
            let response = VectorCraftEndpoint::localhost(port).inspect_document()?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Command::FilmCraft {
            port,
            command,
            params,
        } => filmcraft(port, command, params),
        Command::FilmCraftCommands { port } => {
            let response = FilmCraftEndpoint::localhost(port).list_commands()?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Command::FilmCraftSequence { port } => {
            let response = FilmCraftEndpoint::localhost(port).inspect_sequence()?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Command::Artcraft {
            port,
            token,
            command,
            intent,
            params,
        } => {
            let token = resolve_control_token(token)?;
            bridge(port, &token, command, intent, params)
        }
        Command::PhotoCraftOpen { port, path, token } => {
            let token = resolve_control_token(token)?;
            let response = ControlEndpoint::localhost(port).open_document(&path, Some(&token))?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Command::PhotoCraftSave { port, path, token } => {
            let token = resolve_control_token(token)?;
            let response = ControlEndpoint::localhost(port).save_document(&path, Some(&token))?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
    }
}

fn vectorcraft(port: u16, command: String, intent: String, params: String) -> Result<()> {
    let params: serde_json::Value =
        serde_json::from_str(&params).context("params must be valid JSON")?;
    let envelope = CommandEnvelope::new("vector-1", Actor::User, intent, command, params);
    let response = VectorCraftEndpoint::localhost(port).send(&envelope)?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn list_vector_commands() -> Result<()> {
    println!("{}", serde_json::to_string_pretty(VECTOR_COMMAND_CATALOG)?);
    Ok(())
}

fn filmcraft(port: u16, command: String, params: String) -> Result<()> {
    let params: serde_json::Value =
        serde_json::from_str(&params).context("params must be valid JSON")?;
    let response = FilmCraftEndpoint::localhost(port).execute(&command, params)?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn resolve_control_token(token: Option<String>) -> Result<String> {
    token
        .or_else(|| std::env::var("PHOTOCRAFT_CONTROL_TOKEN").ok())
        .context("provide --token or set PHOTOCRAFT_CONTROL_TOKEN")
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

fn list_image_commands() -> Result<()> {
    println!("{}", serde_json::to_string_pretty(IMAGE_COMMAND_CATALOG)?);
    Ok(())
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn parses_vectorcraft_command_and_vector_command_catalog_subcommand() {
        let cli = Cli::try_parse_from([
            "praxilume",
            "vectorcraft",
            "7979",
            "--command",
            "vector.rectangle",
            "--params",
            r#"{"x":1,"y":2,"width":10,"height":20}"#,
        ])
        .expect("valid VectorCraft command");
        match cli.command {
            Command::VectorCraft {
                port,
                command,
                params,
                ..
            } => {
                assert_eq!(port, 7979);
                assert_eq!(command, "vector.rectangle");
                assert!(params.contains("width"));
            }
            _ => panic!("expected vectorcraft subcommand"),
        }
        assert!(matches!(
            Cli::try_parse_from(["praxilume", "vector-commands"])
                .expect("valid vector command catalog")
                .command,
            Command::VectorCommands
        ));
    }

    #[test]
    fn parses_root_scoped_vectorcraft_open_and_export() {
        assert!(matches!(
            Cli::try_parse_from(["praxilume", "vectorcraft-open", "7979", "assets/logo.svg"])
                .expect("valid open command")
                .command,
            Command::VectorCraftOpen { port: 7979, .. }
        ));
        assert!(matches!(
            Cli::try_parse_from([
                "praxilume",
                "vectorcraft-export",
                "7979",
                "exports/logo.svg"
            ])
            .expect("valid export command")
            .command,
            Command::VectorCraftExport { port: 7979, .. }
        ));
    }

    #[test]
    fn parses_vectorcraft_live_discovery_commands() {
        assert!(matches!(
            Cli::try_parse_from(["praxilume", "vectorcraft-commands", "7979"])
                .expect("valid command discovery CLI")
                .command,
            Command::VectorCraftCommands { port: 7979 }
        ));
        assert!(matches!(
            Cli::try_parse_from(["praxilume", "vectorcraft-inspect", "7979"])
                .expect("valid document inspection CLI")
                .command,
            Command::VectorCraftInspect { port: 7979 }
        ));
    }

    #[test]
    fn parses_filmcraft_bridge_commands() {
        match Cli::try_parse_from([
            "praxilume",
            "filmcraft",
            "9876",
            "--command",
            "sequence.inspect",
            "--params",
            "{}",
        ])
        .expect("valid FilmCraft command")
        .command
        {
            Command::FilmCraft { port, command, params } => {
                assert_eq!(port, 9876);
                assert_eq!(command, "sequence.inspect");
                assert_eq!(params, "{}");
            }
            _ => panic!("expected filmcraft subcommand"),
        }
        assert!(matches!(
            Cli::try_parse_from(["praxilume", "filmcraft-commands", "9876"])
                .expect("valid command discovery")
                .command,
            Command::FilmCraftCommands { port: 9876 }
        ));
        assert!(matches!(
            Cli::try_parse_from(["praxilume", "filmcraft-sequence", "9876"])
                .expect("valid sequence inspection")
                .command,
            Command::FilmCraftSequence { port: 9876 }
        ));
    }

    #[test]
    fn parses_root_scoped_photocraft_open_command() {
        let cli = Cli::try_parse_from(["praxilume", "photocraft-open", "7878", "assets/input.png"])
            .expect("valid open CLI");
        match cli.command {
            Command::PhotoCraftOpen { port, path, token } => {
                assert_eq!(port, 7878);
                assert_eq!(path, "assets/input.png");
                assert!(token.is_none());
            }
            _ => panic!("expected photocraft-open subcommand"),
        }
    }

    #[test]
    fn parses_root_scoped_photocraft_save_command() {
        let cli =
            Cli::try_parse_from(["praxilume", "photocraft-save", "7878", "exports/edited.png"])
                .expect("valid save CLI");
        match cli.command {
            Command::PhotoCraftSave { port, path, token } => {
                assert_eq!(port, 7878);
                assert_eq!(path, "exports/edited.png");
                assert!(token.is_none());
            }
            _ => panic!("expected photocraft-save subcommand"),
        }
    }
}
