use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 1;
pub const COMMAND_PROTOCOL: &str = "praxilume.command/v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(String);

impl Id {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Id {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub id: Id,
    pub name: String,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
    pub objects: Vec<ProjectObject>,
    pub assets: BTreeMap<Id, AssetRecord>,
    pub history: Vec<OperationRecord>,
}

impl Project {
    pub fn new(id: impl Into<Id>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            schema_version: SCHEMA_VERSION,
            id: id.into(),
            name: name.into(),
            revision: 0,
            created_at: now,
            updated_at: now,
            metadata: BTreeMap::new(),
            objects: Vec::new(),
            assets: BTreeMap::new(),
            history: Vec::new(),
        }
    }

    pub fn apply(&mut self, operation: &Operation) -> Result<OperationRecord, CoreError> {
        match operation {
            Operation::CreateObject(object) => {
                if self.objects.iter().any(|item| item.id == object.id) {
                    return Err(CoreError::DuplicateId(object.id.as_str().to_owned()));
                }
                self.objects.push(object.clone());
            }
            Operation::SetMetadata { key, value } => {
                self.metadata.insert(key.clone(), value.clone());
            }
            Operation::RegisterAsset(asset) => {
                if self.assets.contains_key(&asset.id) {
                    return Err(CoreError::DuplicateId(asset.id.as_str().to_owned()));
                }
                self.assets.insert(asset.id.clone(), asset.clone());
            }
        }

        self.revision += 1;
        self.updated_at = Utc::now();
        let record = OperationRecord {
            revision: self.revision,
            operation: operation.clone(),
            applied_at: self.updated_at,
        };
        self.history.push(record.clone());
        Ok(record)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectObject {
    pub id: Id,
    pub kind: ObjectKind,
    pub name: String,
    pub properties: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Image,
    Vector,
    Text,
    Video,
    Audio,
    Shape,
    Group,
    Component,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: Id,
    pub logical_name: String,
    pub media_type: String,
    pub sha256: String,
    pub source: AssetSource,
    pub license: LicenseInfo,
    pub tags: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AssetSource {
    LocalFile {
        path: String,
    },
    Generated {
        provider: String,
        request_id: Option<String>,
    },
    Remote {
        uri: String,
    },
    Upstream {
        project: String,
        uri: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub spdx: Option<String>,
    pub attribution: Option<String>,
    pub restrictions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    CreateObject(ProjectObject),
    SetMetadata { key: String, value: String },
    RegisterAsset(AssetRecord),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub revision: u64,
    pub operation: Operation,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub protocol: String,
    pub command_id: Id,
    pub issued_at: DateTime<Utc>,
    pub actor: Actor,
    pub intent: String,
    pub command: String,
    pub params: serde_json::Value,
    pub dry_run: bool,
}

impl CommandEnvelope {
    pub fn new(
        command_id: impl Into<Id>,
        actor: Actor,
        intent: impl Into<String>,
        command: impl Into<String>,
        params: serde_json::Value,
    ) -> Self {
        Self {
            protocol: COMMAND_PROTOCOL.to_owned(),
            command_id: command_id.into(),
            issued_at: Utc::now(),
            actor,
            intent: intent.into(),
            command: command.into(),
            params,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Actor {
    User,
    Agent { agent_id: Id },
    Automation { workflow_id: Id },
    System,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentPolicy {
    pub allowed_capabilities: BTreeSet<Capability>,
    pub allowed_roots: Vec<String>,
    pub require_confirmation_for: BTreeSet<Capability>,
    pub allow_network: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadProject,
    WriteProject,
    ReadAsset,
    WriteAsset,
    ExecuteCreativeCommand,
    SpawnUpstreamTool,
    Network,
    Export,
}

impl AgentPolicy {
    pub fn authorize(&self, capability: &Capability) -> Result<(), CoreError> {
        if self.allowed_capabilities.contains(capability) {
            Ok(())
        } else {
            Err(CoreError::CapabilityDenied(format!("{capability:?}")))
        }
    }
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("duplicate id: {0}")]
    DuplicateId(String),
    #[error("capability denied: {0}")]
    CapabilityDenied(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_revision_increments() {
        let mut project = Project::new("demo", "Demo Project");
        let object = ProjectObject {
            id: Id::from("node-1"),
            kind: ObjectKind::Shape,
            name: "Rectangle".to_owned(),
            properties: BTreeMap::new(),
        };
        let record = project
            .apply(&Operation::CreateObject(object))
            .expect("apply");
        assert_eq!(project.revision, 1);
        assert_eq!(record.revision, 1);
    }

    #[test]
    fn duplicate_objects_are_rejected() {
        let mut project = Project::new("demo", "Demo");
        let object = ProjectObject {
            id: Id::from("node-1"),
            kind: ObjectKind::Shape,
            name: "Rectangle".to_owned(),
            properties: BTreeMap::new(),
        };
        project
            .apply(&Operation::CreateObject(object.clone()))
            .expect("first");
        assert!(matches!(
            project.apply(&Operation::CreateObject(object)),
            Err(CoreError::DuplicateId(_))
        ));
    }

    #[test]
    fn policies_default_to_deny() {
        let policy = AgentPolicy::default();
        assert!(policy.authorize(&Capability::WriteProject).is_err());
    }

    #[test]
    fn command_envelope_uses_stable_protocol() {
        let envelope = CommandEnvelope::new(
            "cmd-1",
            Actor::User,
            "test",
            "project.rename",
            serde_json::json!({"name":"X"}),
        );
        assert_eq!(envelope.protocol, COMMAND_PROTOCOL);
    }
}
