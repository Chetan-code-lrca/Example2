use anyhow::{Context, Result, bail};
use praxilume_core::{AgentPolicy, Actor, Capability, CommandEnvelope, Project};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub format: String,
    pub format_version: u32,
    pub project_id: String,
    pub project_file: String,
}

impl ProjectManifest {
    pub fn from_project(project: &Project) -> Self {
        Self {
            format: "praxilume-project".to_owned(),
            format_version: project.schema_version,
            project_id: project.id.as_str().to_owned(),
            project_file: "project.json".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectStore {
    root: PathBuf,
}

impl ProjectStore {
    pub fn create(root: impl Into<PathBuf>, project: &Project) -> Result<Self> {
        let store = Self { root: root.into() };
        store.ensure_layout()?;
        store.save(project)?;
        Ok(store)
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let store = Self { root: root.into() };
        if !store.root.join("manifest.json").is_file() {
            bail!("not a Praxilume project: {}", store.root.display());
        }
        Ok(store)
    }

    pub fn root(&self) -> &Path { &self.root }

    fn ensure_layout(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("assets"))?;
        fs::create_dir_all(self.root.join("history"))?;
        Ok(())
    }

    pub fn save(&self, project: &Project) -> Result<()> {
        self.ensure_layout()?;
        let manifest = ProjectManifest::from_project(project);
        write_json_atomic(&self.root.join("manifest.json"), &manifest)?;
        write_json_atomic(&self.root.join("project.json"), project)?;
        Ok(())
    }

    pub fn load(&self) -> Result<Project> {
        read_json(&self.root.join("project.json"))
    }

    pub fn asset_path(&self, sha256: &str, extension: &str) -> PathBuf {
        self.root.join("assets").join(format!("{sha256}.{extension}"))
    }

    pub fn import_asset(&self, source: &Path, extension: &str) -> Result<(String, PathBuf)> {
        validate_extension(extension)?;
        let bytes = fs::read(source)
            .with_context(|| format!("failed to read asset {}", source.display()))?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256 = format!("{:x}", hasher.finalize());
        let target = self.asset_path(&sha256, extension);
        if !target.exists() {
            let tmp = target.with_extension("tmp");
            fs::write(&tmp, &bytes)?;
            fs::rename(tmp, &target)?;
        }
        Ok((sha256, target))
    }

    pub fn authorize_command(policy: &AgentPolicy, envelope: &CommandEnvelope) -> Result<()> {
        match &envelope.actor {
            Actor::Agent { .. } | Actor::Automation { .. } => {
                policy.authorize(&Capability::ExecuteCreativeCommand)
                    .map_err(anyhow::Error::msg)
            }
            Actor::User | Actor::System => Ok(()),
        }
    }
}

fn validate_extension(extension: &str) -> Result<()> {
    if extension.is_empty()
        || !extension.chars().all(|c| c.is_ascii_alphanumeric())
        || extension.len() > 12
    {
        bail!("asset extension must be 1-12 ASCII alphanumeric characters");
    }
    Ok(())
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxilume_core::Project;

    #[test]
    fn store_round_trips_a_project() {
        let root = std::env::temp_dir().join(format!("praxilume-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let project = Project::new("sample", "Sample");
        let store = ProjectStore::create(&root, &project).expect("create");
        let loaded = store.load().expect("load");
        assert_eq!(loaded.name, "Sample");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unsafe_asset_extensions_are_rejected() {
        assert!(validate_extension("../tmp").is_err());
        assert!(validate_extension("png").is_ok());
    }
}
