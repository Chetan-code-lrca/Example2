use anyhow::{Context, Result, bail};
use praxilume_core::{Actor, CommandEnvelope, Id};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const TIMELINE_CAPABILITY_PROTOCOL: &str = "praxilume.timeline/v1";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct TimelineCommandDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub parameters: &'static str,
    pub upstream_command: &'static str,
    pub mutating: bool,
}

pub const TIMELINE_COMMAND_CATALOG: &[TimelineCommandDescriptor] = &[
    TimelineCommandDescriptor {
        id: "timeline.add_edit",
        label: "Add edit",
        description: "Split clips at an explicit time or the current playhead.",
        parameters: "{time_ticks?: non-negative integer}",
        upstream_command: "sequence.addEdit",
        mutating: true,
    },
    TimelineCommandDescriptor {
        id: "timeline.add_edit_all_tracks",
        label: "Add edit on all tracks",
        description: "Split targeted clips on all tracks at a time or the current playhead.",
        parameters: "{time_ticks?: non-negative integer}",
        upstream_command: "sequence.addEditAllTracks",
        mutating: true,
    },
    TimelineCommandDescriptor {
        id: "timeline.razor",
        label: "Razor",
        description: "Split a clip or selected/targeted clips at a timeline time.",
        parameters: "{time_ticks?: non-negative integer, track?: track name or positive ID, clip?: positive ID}",
        upstream_command: "timeline.razor",
        mutating: true,
    },
    TimelineCommandDescriptor {
        id: "timeline.move_clips",
        label: "Move clips",
        description: "Move named clips to destination tracks and start times.",
        parameters: "{moves: [{clip: positive ID, track: name or positive ID, time_ticks: non-negative integer}], insert?: bool, linked?: bool, copy?: bool}",
        upstream_command: "timeline.move",
        mutating: true,
    },
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TrackTarget {
    Name(String),
    Id(u64),
}

impl TrackTarget {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Name(name) if name.trim().is_empty() => bail!("track name must not be empty"),
            Self::Id(0) => bail!("track ID must be positive"),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipMove {
    pub clip: u64,
    pub track: TrackTarget,
    pub time_ticks: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimelineOperation {
    AddEdit {
        time_ticks: Option<i64>,
    },
    AddEditAllTracks {
        time_ticks: Option<i64>,
    },
    Razor {
        time_ticks: Option<i64>,
        track: Option<TrackTarget>,
        clip: Option<u64>,
    },
    MoveClips {
        moves: Vec<ClipMove>,
        insert: Option<bool>,
        linked: Option<bool>,
        copy: Option<bool>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimeParams {
    #[serde(default)]
    time_ticks: Option<i64>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RazorParams {
    #[serde(default)]
    time_ticks: Option<i64>,
    #[serde(default)]
    track: Option<TrackTarget>,
    #[serde(default)]
    clip: Option<u64>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveParams {
    moves: Vec<ClipMove>,
    #[serde(default)]
    insert: Option<bool>,
    #[serde(default)]
    linked: Option<bool>,
    #[serde(default)]
    copy: Option<bool>,
}

impl TimelineOperation {
    /// Parse and validate a canonical command. Times are integer FilmCraft ticks
    /// (254,016,000,000 ticks per second).
    pub fn from_command(id: &str, params: Value) -> Result<Self> {
        if !params.is_object() {
            bail!("timeline command params must be a JSON object");
        }
        let op = match id {
            "timeline.add_edit" => Self::AddEdit {
                time_ticks: parse::<TimeParams>(id, params)?.time_ticks,
            },
            "timeline.add_edit_all_tracks" => Self::AddEditAllTracks {
                time_ticks: parse::<TimeParams>(id, params)?.time_ticks,
            },
            "timeline.razor" => {
                let p = parse::<RazorParams>(id, params)?;
                Self::Razor {
                    time_ticks: p.time_ticks,
                    track: p.track,
                    clip: p.clip,
                }
            }
            "timeline.move_clips" => {
                let p = parse::<MoveParams>(id, params)?;
                Self::MoveClips {
                    moves: p.moves,
                    insert: p.insert,
                    linked: p.linked,
                    copy: p.copy,
                }
            }
            _ => bail!("unknown canonical timeline command '{id}'"),
        };
        op.validate()?;
        Ok(op)
    }

    pub fn command_id(&self) -> &'static str {
        match self {
            Self::AddEdit { .. } => "timeline.add_edit",
            Self::AddEditAllTracks { .. } => "timeline.add_edit_all_tracks",
            Self::Razor { .. } => "timeline.razor",
            Self::MoveClips { .. } => "timeline.move_clips",
        }
    }

    pub fn upstream_command(&self) -> &'static str {
        match self {
            Self::AddEdit { .. } => "sequence.addEdit",
            Self::AddEditAllTracks { .. } => "sequence.addEditAllTracks",
            Self::Razor { .. } => "timeline.razor",
            Self::MoveClips { .. } => "timeline.move",
        }
    }

    pub fn canonical_params(&self) -> Value {
        match self {
            Self::AddEdit { time_ticks } | Self::AddEditAllTracks { time_ticks } => {
                optional_time("time_ticks", *time_ticks)
            }
            Self::Razor {
                time_ticks,
                track,
                clip,
            } => {
                let mut p = optional_time("time_ticks", *time_ticks);
                if let Some(v) = track {
                    p["track"] = json!(v);
                }
                if let Some(v) = clip {
                    p["clip"] = json!(v);
                }
                p
            }
            Self::MoveClips {
                moves,
                insert,
                linked,
                copy,
            } => {
                let mut p = json!({"moves": moves.iter().map(|m| json!({"clip":m.clip,"track":m.track,"time_ticks":m.time_ticks})).collect::<Vec<_>>()});
                if let Some(v) = insert {
                    p["insert"] = json!(v);
                }
                if let Some(v) = linked {
                    p["linked"] = json!(v);
                }
                if let Some(v) = copy {
                    p["copy"] = json!(v);
                }
                p
            }
        }
    }

    pub fn upstream_params(&self) -> Value {
        match self {
            Self::AddEdit { time_ticks } | Self::AddEditAllTracks { time_ticks } => {
                optional_time("time", *time_ticks)
            }
            Self::Razor {
                time_ticks,
                track,
                clip,
            } => {
                let mut p = optional_time("time", *time_ticks);
                if let Some(v) = track {
                    p["track"] = json!(v);
                }
                if let Some(v) = clip {
                    p["clip"] = json!(v);
                }
                p
            }
            Self::MoveClips {
                moves,
                insert,
                linked,
                copy,
            } => {
                let mut p = json!({"moves": moves.iter().map(|m| json!({"clip":m.clip,"track":m.track,"time":m.time_ticks})).collect::<Vec<_>>()});
                if let Some(v) = insert {
                    p["insert"] = json!(v);
                }
                if let Some(v) = linked {
                    p["linked"] = json!(v);
                }
                if let Some(v) = copy {
                    p["copy"] = json!(v);
                }
                p
            }
        }
    }

    pub fn into_envelope(
        &self,
        id: impl Into<String>,
        actor: Actor,
        intent: impl Into<String>,
    ) -> CommandEnvelope {
        CommandEnvelope::new(
            Id::new(id.into()),
            actor,
            intent,
            self.command_id(),
            self.canonical_params(),
        )
    }

    fn validate(&self) -> Result<()> {
        let validate_time = |t: Option<i64>| -> Result<()> {
            if t.is_some_and(|v| v < 0) {
                bail!("timeline time must not be negative");
            }
            Ok(())
        };
        match self {
            Self::AddEdit { time_ticks } | Self::AddEditAllTracks { time_ticks } => {
                validate_time(*time_ticks)
            }
            Self::Razor {
                time_ticks,
                track,
                clip,
            } => {
                validate_time(*time_ticks)?;
                if let Some(v) = track {
                    v.validate()?;
                }
                if *clip == Some(0) {
                    bail!("clip ID must be positive");
                }
                Ok(())
            }
            Self::MoveClips { moves, .. } => {
                if moves.is_empty() {
                    bail!("timeline.move_clips requires at least one move");
                }
                let mut seen = BTreeSet::new();
                for m in moves {
                    if m.clip == 0 {
                        bail!("clip ID must be positive");
                    }
                    if !seen.insert(m.clip) {
                        bail!("duplicate clip ID {} in timeline.move_clips", m.clip);
                    }
                    if m.time_ticks < 0 {
                        bail!("timeline move time must not be negative");
                    }
                    m.track.validate()?;
                }
                Ok(())
            }
        }
    }
}

fn optional_time(key: &str, time: Option<i64>) -> Value {
    let mut value = json!({});
    if let Some(t) = time {
        value[key] = json!(t);
    }
    value
}

fn parse<T: DeserializeOwned>(id: &str, params: Value) -> Result<T> {
    serde_json::from_value(params)
        .with_context(|| format!("invalid parameters for timeline command '{id}'"))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineSnapshot {
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub settings: Value,
    #[serde(rename = "duration")]
    pub duration_ticks: i64,
    #[serde(rename = "durationFrames")]
    pub duration_frames: i64,
    #[serde(default)]
    pub playhead: Option<i64>,
    #[serde(rename = "in", default)]
    pub in_ticks: Option<i64>,
    #[serde(rename = "out", default)]
    pub out_ticks: Option<i64>,
    #[serde(default)]
    pub markers: Vec<TimelineMarker>,
    #[serde(default)]
    pub video: Vec<TimelineTrack>,
    #[serde(default)]
    pub audio: Vec<TimelineTrack>,
    #[serde(default)]
    pub selection: Vec<u64>,
    #[serde(rename = "transitionSelection", default)]
    pub transition_selection: Vec<u64>,
    #[serde(rename = "gapSelection", default)]
    pub gap_selection: Option<Value>,
}

impl TimelineSnapshot {
    pub fn from_control_response(response: &Value) -> Result<Self> {
        let payload = response
            .get("result")
            .filter(|v| v.is_object())
            .unwrap_or(response);
        if !payload.is_object() {
            bail!("FilmCraft sequence inspection result must be a JSON object");
        }
        serde_json::from_value(payload.clone())
            .context("FilmCraft sequence.inspect result did not match the typed timeline model")
    }
    pub fn clips(&self) -> impl Iterator<Item = &TimelineClip> {
        self.video
            .iter()
            .chain(self.audio.iter())
            .flat_map(|t| t.items.iter())
    }
    pub fn clip_count(&self) -> usize {
        self.clips().count()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineMarker {
    pub id: u64,
    #[serde(rename = "start")]
    pub start_ticks: i64,
    #[serde(rename = "duration")]
    pub duration_ticks: i64,
    pub kind: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineTrack {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub locked: Option<bool>,
    #[serde(rename = "syncLock", default)]
    pub sync_lock: Option<bool>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub muted: Option<bool>,
    #[serde(default)]
    pub solo: Option<bool>,
    #[serde(default)]
    pub items: Vec<TimelineClip>,
    #[serde(default)]
    pub transitions: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineClip {
    #[serde(rename = "clip")]
    pub id: u64,
    #[serde(rename = "item")]
    pub item_id: u64,
    pub name: String,
    #[serde(rename = "start")]
    pub start_ticks: i64,
    #[serde(rename = "duration")]
    pub duration_ticks: i64,
    #[serde(rename = "end")]
    pub end_ticks: i64,
    #[serde(rename = "startFrame", default)]
    pub start_frame: Option<i64>,
    #[serde(rename = "durationFrames", default)]
    pub duration_frames: Option<i64>,
    #[serde(rename = "sourceIn", default)]
    pub source_in_ticks: Option<i64>,
    #[serde(default)]
    pub speed: Option<f64>,
    #[serde(default)]
    pub reverse: Option<bool>,
    #[serde(rename = "endFrame", default)]
    pub end_frame: Option<i64>,
    #[serde(rename = "sourceOut", default)]
    pub source_out_ticks: Option<i64>,
    #[serde(rename = "gainDb", default)]
    pub gain_db: Option<f64>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub link: Option<Value>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub effects: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_add_edit_maps_tick_parameter_to_filmcraft() {
        let op = TimelineOperation::from_command(
            "timeline.add_edit",
            json!({"time_ticks": 254_016_000_000_i64}),
        )
        .unwrap();
        assert_eq!(op.upstream_command(), "sequence.addEdit");
        assert_eq!(op.upstream_params(), json!({"time": 254_016_000_000_i64}));
    }

    #[test]
    fn playhead_default_omits_time() {
        let op = TimelineOperation::from_command("timeline.add_edit", json!({})).unwrap();
        assert_eq!(op.upstream_params(), json!({}));
    }

    #[test]
    fn razor_maps_targets() {
        let op = TimelineOperation::from_command(
            "timeline.razor",
            json!({"time_ticks": 48, "track": "V1", "clip": 12}),
        )
        .unwrap();
        assert_eq!(
            op.upstream_params(),
            json!({"time": 48, "track": "V1", "clip": 12})
        );
    }

    #[test]
    fn move_clips_maps_tick_times_and_flags() {
        let op = TimelineOperation::from_command("timeline.move_clips", json!({"moves":[{"clip":12,"track":"V1","time_ticks":100}],"insert":true,"linked":false,"copy":true})).unwrap();
        assert_eq!(
            op.upstream_params(),
            json!({"moves":[{"clip":12,"track":"V1","time":100}],"insert":true,"linked":false,"copy":true})
        );
    }

    #[test]
    fn invalid_and_unknown_parameters_are_rejected() {
        assert!(
            TimelineOperation::from_command("timeline.add_edit", json!({"time_ticks":-1})).is_err()
        );
        assert!(TimelineOperation::from_command("timeline.razor", json!({"clip":0})).is_err());
        assert!(
            TimelineOperation::from_command("timeline.move_clips", json!({"moves":[]})).is_err()
        );
        assert!(
            TimelineOperation::from_command("timeline.add_edit", json!({"time_tick":10})).is_err()
        );
    }

    #[test]
    fn operation_builds_a_versioned_envelope() {
        let op = TimelineOperation::from_command(
            "timeline.add_edit_all_tracks",
            json!({"time_ticks":77}),
        )
        .unwrap();
        let env = op.into_envelope("timeline-test-1", Actor::User, "split at a beat");
        assert_eq!(env.protocol, "praxilume.command/v1");
        assert_eq!(env.command, "timeline.add_edit_all_tracks");
        assert_eq!(env.params, json!({"time_ticks":77}));
    }

    #[test]
    fn sequence_response_becomes_typed_snapshot() {
        let response = json!({"ok":true,"result":{
            "id":9,"name":"Demo sequence","settings":{"fps":24},"duration":200,"durationFrames":2,"playhead":20,
            "in":null,"out":null,"markers":[{"id":7,"start":100,"duration":0,"kind":"Point","name":"Beat","comment":"cut here","color":"Yellow"}],
            "video":[{"id":3,"name":"V1","locked":false,"items":[{"clip":12,"item":8,"name":"shot.mov","start":0,"duration":100,"end":100,"sourceIn":0,"sourceOut":100}]}],
            "audio":[],"selection":[12],"transitionSelection":[],"gapSelection":null
        }});
        let snapshot = TimelineSnapshot::from_control_response(&response).unwrap();
        assert_eq!(snapshot.clip_count(), 1);
        assert_eq!(snapshot.clips().next().unwrap().id, 12);
        assert_eq!(snapshot.duration_ticks, 200);
        assert_eq!(snapshot.markers[0].comment.as_deref(), Some("cut here"));
    }
}
