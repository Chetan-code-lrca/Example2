use anyhow::{Result, bail};
use praxilume_core::{Actor, CommandEnvelope, COMMAND_PROTOCOL};
use serde::Serialize;
use serde_json::{Value, json};

pub const IMAGE_CAPABILITY_PROTOCOL: &str = "praxilume.image/v1";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct ImageCommandDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub parameters: &'static str,
    pub photocraft_command: &'static str,
}

/// Canonical command IDs for edits to an already-open image document.
pub const IMAGE_COMMAND_CATALOG: &[ImageCommandDescriptor] = &[
    ImageCommandDescriptor {
        id: "image.invert",
        label: "Invert",
        description: "Invert the active layer or selected target.",
        parameters: "{}",
        photocraft_command: "image.adjustments.invert",
    },
    ImageCommandDescriptor {
        id: "image.crop",
        label: "Crop",
        description: "Crop or extend the document to an explicit rectangle.",
        parameters: "{x: integer, y: integer, width: positive integer, height: positive integer}",
        photocraft_command: "image.crop",
    },
    ImageCommandDescriptor {
        id: "image.resize",
        label: "Resize image",
        description: "Resize the document to the requested pixel dimensions.",
        parameters: "{width: positive integer, height: positive integer, resample?: automatic|bicubic|nearest|bilinear|lanczos|preserveDetails}",
        photocraft_command: "image.imageSize",
    },
    ImageCommandDescriptor {
        id: "image.brightness_contrast",
        label: "Brightness / contrast",
        description: "Apply a brightness and contrast adjustment.",
        parameters: "{brightness: integer, contrast: integer}",
        photocraft_command: "image.adjustments.brightnessContrast",
    },
    ImageCommandDescriptor {
        id: "image.layer.create",
        label: "Create pixel layer",
        description: "Create a pixel layer above or below the active layer.",
        parameters: "{name?: string, position: above|below}",
        photocraft_command: "layer.new.layer",
    },
    ImageCommandDescriptor {
        id: "image.layer.move",
        label: "Move layer",
        description: "Move one layer above or below a target layer.",
        parameters: "{layer: unsigned integer, target: unsigned integer, position: above|below}",
        photocraft_command: "layer.moveTo",
    },
];

/// Translate a canonical Praxilume command ID to a verified PhotoCraft engine ID.
pub fn upstream_command_id(canonical_id: &str) -> Option<&'static str> {
    IMAGE_COMMAND_CATALOG
        .iter()
        .find(|descriptor| descriptor.id == canonical_id)
        .map(|descriptor| descriptor.photocraft_command)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerPosition {
    Above,
    Below,
}

impl LayerPosition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Above => "above",
            Self::Below => "below",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleMethod {
    Automatic,
    Bicubic,
    Nearest,
    Bilinear,
    Lanczos,
    PreserveDetails,
}

impl ResampleMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Bicubic => "bicubic",
            Self::Nearest => "nearest",
            Self::Bilinear => "bilinear",
            Self::Lanczos => "lanczos",
            Self::PreserveDetails => "preserveDetails",
        }
    }
}

/// Typed inputs for the first, existing-document image capability pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageOperation {
    Invert,
    Crop { x: i32, y: i32, width: i32, height: i32 },
    Resize { width: i32, height: i32, resample: ResampleMethod },
    BrightnessContrast { brightness: i32, contrast: i32 },
    CreateLayer { name: Option<String>, position: LayerPosition },
    MoveLayer { layer: u64, target: u64, position: LayerPosition },
}

impl ImageOperation {
    pub fn command_id(&self) -> &'static str {
        match self {
            Self::Invert => "image.invert",
            Self::Crop { .. } => "image.crop",
            Self::Resize { .. } => "image.resize",
            Self::BrightnessContrast { .. } => "image.brightness_contrast",
            Self::CreateLayer { .. } => "image.layer.create",
            Self::MoveLayer { .. } => "image.layer.move",
        }
    }

    pub fn params(&self) -> Result<Value> {
        match self {
            Self::Invert => Ok(json!({})),
            Self::Crop { x, y, width, height } => {
                if *width <= 0 || *height <= 0 {
                    bail!("image.crop requires positive width and height");
                }
                Ok(json!({"x": x, "y": y, "width": width, "height": height}))
            }
            Self::Resize { width, height, resample } => {
                if *width <= 0 || *height <= 0 {
                    bail!("image.resize requires positive width and height");
                }
                Ok(json!({
                    "width": width,
                    "height": height,
                    "resample": resample.as_str(),
                }))
            }
            Self::BrightnessContrast { brightness, contrast } => {
                Ok(json!({"brightness": brightness, "contrast": contrast}))
            }
            Self::CreateLayer { name, position } => match name {
                Some(value) if value.trim().is_empty() => {
                    bail!("image.layer.create name must not be empty");
                }
                Some(value) => Ok(json!({"name": value, "position": position.as_str()})),
                None => Ok(json!({"position": position.as_str()})),
            },
            Self::MoveLayer { layer, target, position } => {
                if layer == target {
                    bail!("image.layer.move requires distinct layer and target IDs");
                }
                Ok(json!({"layer": layer, "target": target, "position": position.as_str()}))
            }
        }
    }

    pub fn into_envelope(
        &self,
        command_id: impl Into<String>,
        actor: Actor,
        intent: impl Into<String>,
    ) -> Result<CommandEnvelope> {
        Ok(CommandEnvelope::new(
            command_id.into(),
            actor,
            intent,
            self.command_id(),
            self.params()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxilume_core::Id;

    #[test]
    fn operation_creates_stable_praxilume_envelope() {
        let envelope = ImageOperation::Invert
            .into_envelope("cmd-image-1", Actor::User, "invert an image")
            .expect("valid operation");
        assert_eq!(envelope.protocol, COMMAND_PROTOCOL);
        assert_eq!(envelope.command, "image.invert");
        assert_eq!(envelope.command_id, Id::from("cmd-image-1"));
        assert_eq!(envelope.params, json!({}));
    }

    #[test]
    fn crop_rejects_non_positive_dimensions_and_keeps_negative_origin() {
        let valid = ImageOperation::Crop { x: -10, y: 4, width: 80, height: 60 };
        assert_eq!(
            valid.params().expect("valid crop"),
            json!({"x": -10, "y": 4, "width": 80, "height": 60})
        );
        let invalid = ImageOperation::Crop { x: 0, y: 0, width: 0, height: 10 };
        assert!(invalid.params().is_err());
    }

    #[test]
    fn resize_requires_positive_dimensions() {
        let valid = ImageOperation::Resize {
            width: 1280, height: 720, resample: ResampleMethod::Lanczos,
        };
        assert_eq!(
            valid.params().expect("valid resize"),
            json!({"width": 1280, "height": 720, "resample": "lanczos"})
        );
        assert!(ImageOperation::Resize {
            width: -1, height: 720, resample: ResampleMethod::Bicubic,
        }.params().is_err());
    }

    #[test]
    fn layer_move_rejects_a_self_target() {
        assert!(ImageOperation::MoveLayer {
            layer: 9, target: 9, position: LayerPosition::Above,
        }.params().is_err());
    }

    #[test]
    fn catalog_maps_canonical_ids_to_verified_photocraft_ids() {
        assert_eq!(upstream_command_id("image.invert"), Some("image.adjustments.invert"));
        assert_eq!(upstream_command_id("image.layer.move"), Some("layer.moveTo"));
        assert_eq!(upstream_command_id("image.not_supported"), None);
        assert_eq!(IMAGE_CAPABILITY_PROTOCOL, "praxilume.image/v1");
    }
}
