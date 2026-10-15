use anyhow::{Result, bail};
use praxilume_core::{Actor, COMMAND_PROTOCOL, CommandEnvelope};
use serde::Serialize;
use serde_json::{Value, json};

pub const VECTOR_CAPABILITY_PROTOCOL: &str = "praxilume.vector/v1";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct VectorCommandDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub parameters: &'static str,
    pub vectorcraft_command: &'static str,
}

/// Canonical vector operations supported by the initial VectorCraft adapter.
pub const VECTOR_COMMAND_CATALOG: &[VectorCommandDescriptor] = &[
    VectorCommandDescriptor {
        id: "vector.rectangle",
        label: "Create rectangle",
        description: "Create a rectangle on the active VectorCraft document.",
        parameters: "{x: finite number, y: finite number, width: positive finite number, height: positive finite number, radius?: non-negative finite number, rotation?: finite degrees}",
        vectorcraft_command: "shape.rectangle",
    },
    VectorCommandDescriptor {
        id: "vector.ellipse",
        label: "Create ellipse",
        description: "Create an ellipse on the active VectorCraft document.",
        parameters: "{x: finite number, y: finite number, width: positive finite number, height: positive finite number, rotation?: finite degrees}",
        vectorcraft_command: "shape.ellipse",
    },
    VectorCommandDescriptor {
        id: "vector.path.create",
        label: "Create SVG path",
        description: "Create a path from SVG path data on the active VectorCraft document.",
        parameters: "{d: non-empty SVG path data}",
        vectorcraft_command: "path.create",
    },
    VectorCommandDescriptor {
        id: "vector.paint.fill",
        label: "Set fill",
        description: "Set the fill paint for the current selection or active fill proxy.",
        parameters: "{color: #RRGGBB or none}",
        vectorcraft_command: "paint.setFill",
    },
    VectorCommandDescriptor {
        id: "vector.paint.stroke",
        label: "Set stroke",
        description: "Set the stroke paint for the current selection or active stroke proxy.",
        parameters: "{color: #RRGGBB or none}",
        vectorcraft_command: "paint.setStroke",
    },
];

pub fn upstream_command_id(canonical_id: &str) -> Option<&'static str> {
    VECTOR_COMMAND_CATALOG
        .iter()
        .find(|descriptor| descriptor.id == canonical_id)
        .map(|descriptor| descriptor.vectorcraft_command)
}

#[derive(Debug, Clone, PartialEq)]
pub enum VectorOperation {
    Rectangle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: Option<f64>,
        rotation: Option<f64>,
    },
    Ellipse {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rotation: Option<f64>,
    },
    SvgPath {
        d: String,
    },
    SetFillColor {
        color: String,
    },
    SetStrokeColor {
        color: String,
    },
}

impl VectorOperation {
    pub fn command_id(&self) -> &'static str {
        match self {
            Self::Rectangle { .. } => "vector.rectangle",
            Self::Ellipse { .. } => "vector.ellipse",
            Self::SvgPath { .. } => "vector.path.create",
            Self::SetFillColor { .. } => "vector.paint.fill",
            Self::SetStrokeColor { .. } => "vector.paint.stroke",
        }
    }

    pub fn params(&self) -> Result<Value> {
        match self {
            Self::Rectangle {
                x,
                y,
                width,
                height,
                radius,
                rotation,
            } => {
                validate_position_and_size(*x, *y, *width, *height, "vector.rectangle")?;
                let mut params = json!({"x": x, "y": y, "width": width, "height": height});
                if let Some(radius) = radius {
                    if !radius.is_finite() || *radius < 0.0 {
                        bail!("rectangle radius must be finite and non-negative");
                    }
                    params["radius"] = json!(radius);
                }
                if let Some(rotation) = rotation {
                    if !rotation.is_finite() {
                        bail!("rectangle rotation must be finite");
                    }
                    params["rotation"] = json!(rotation);
                }
                Ok(params)
            }
            Self::Ellipse {
                x,
                y,
                width,
                height,
                rotation,
            } => {
                validate_position_and_size(*x, *y, *width, *height, "vector.ellipse")?;
                let mut params = json!({"x": x, "y": y, "width": width, "height": height});
                if let Some(rotation) = rotation {
                    if !rotation.is_finite() {
                        bail!("ellipse rotation must be finite");
                    }
                    params["rotation"] = json!(rotation);
                }
                Ok(params)
            }
            Self::SvgPath { d } => {
                if d.trim().is_empty() || d.contains('\0') {
                    bail!("SVG path data must be non-empty and contain no NUL characters");
                }
                Ok(json!({"d": d}))
            }
            Self::SetFillColor { color } | Self::SetStrokeColor { color } => {
                validate_solid_color(color)?;
                Ok(json!({"color": color}))
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

fn validate_position_and_size(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    command: &str,
) -> Result<()> {
    if !x.is_finite() || !y.is_finite() {
        bail!("{command} requires finite x and y coordinates");
    }
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        bail!("{command} requires positive finite width and height");
    }
    Ok(())
}

fn validate_solid_color(color: &str) -> Result<()> {
    if color == "none" {
        return Ok(());
    }
    let bytes = color.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        bail!("color must be 'none' or a six-digit #RRGGBB value");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxilume_core::Id;

    #[test]
    fn rectangle_builds_a_versioned_command_envelope() {
        let operation = VectorOperation::Rectangle {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
            radius: Some(4.0),
            rotation: Some(15.0),
        };
        let envelope = operation
            .into_envelope("vector-1", Actor::User, "create a rounded rectangle")
            .expect("valid rectangle");
        assert_eq!(envelope.protocol, COMMAND_PROTOCOL);
        assert_eq!(envelope.command, "vector.rectangle");
        assert_eq!(envelope.command_id, Id::from("vector-1"));
        assert_eq!(
            envelope.params,
            json!({"x":10.0,"y":20.0,"width":100.0,"height":50.0,"radius":4.0,"rotation":15.0})
        );
    }

    #[test]
    fn shapes_reject_invalid_dimensions_and_rotation() {
        assert!(
            VectorOperation::Ellipse {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 10.0,
                rotation: None,
            }
            .params()
            .is_err()
        );
        assert!(
            VectorOperation::Rectangle {
                x: f64::NAN,
                y: 0.0,
                width: 20.0,
                height: 20.0,
                radius: None,
                rotation: None,
            }
            .params()
            .is_err()
        );
        assert!(
            VectorOperation::Ellipse {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
                rotation: Some(f64::INFINITY),
            }
            .params()
            .is_err()
        );
    }

    #[test]
    fn svg_path_and_paint_inputs_are_validated() {
        assert_eq!(
            VectorOperation::SvgPath {
                d: "M10 10 L40 10 Z".to_owned()
            }
            .params()
            .unwrap(),
            json!({"d": "M10 10 L40 10 Z"})
        );
        assert!(
            VectorOperation::SvgPath { d: "  ".to_owned() }
                .params()
                .is_err()
        );
        assert_eq!(
            VectorOperation::SetFillColor {
                color: "#cc66AA".to_owned()
            }
            .params()
            .unwrap(),
            json!({"color": "#cc66AA"})
        );
        assert!(
            VectorOperation::SetStrokeColor {
                color: "#xyzxyz".to_owned()
            }
            .params()
            .is_err()
        );
        assert!(
            VectorOperation::SetFillColor {
                color: "red".to_owned()
            }
            .params()
            .is_err()
        );
        assert!(
            VectorOperation::SetFillColor { color: "none".to_owned() }.params().is_ok()
        );
    }

    #[test]
    fn catalog_maps_only_declared_canonical_commands() {
        assert_eq!(
            upstream_command_id("vector.rectangle"),
            Some("shape.rectangle")
        );
        assert_eq!(
            upstream_command_id("vector.path.create"),
            Some("path.create")
        );
        assert_eq!(
            upstream_command_id("vector.paint.stroke"),
            Some("paint.setStroke")
        );
        assert_eq!(upstream_command_id("vector.unknown"), None);
        assert_eq!(VECTOR_CAPABILITY_PROTOCOL, "praxilume.vector/v1");
        assert_eq!(VECTOR_COMMAND_CATALOG.len(), 5);
    }
}
