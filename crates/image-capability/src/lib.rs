use anyhow::{Result, bail};
use praxilume_core::{Actor, COMMAND_PROTOCOL, CommandEnvelope};
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
    ImageCommandDescriptor {
        id: "image.adjustment_layer.hue_saturation",
        label: "Hue / saturation adjustment layer",
        description: "Create a non-destructive hue and saturation adjustment layer.",
        parameters: "{hue: -180..180, saturation: -100..100}",
        photocraft_command: "layer.newAdjustmentLayer.hueSaturation",
    },
    ImageCommandDescriptor {
        id: "image.adjustment_layer.levels",
        label: "Levels adjustment layer",
        description: "Create a non-destructive levels adjustment layer.",
        parameters: "{inBlack: 0..254, inWhite: 1..255, gamma: positive finite number}",
        photocraft_command: "layer.newAdjustmentLayer.levels",
    },
    ImageCommandDescriptor {
        id: "image.adjustment_layer.curves",
        label: "Curves adjustment layer",
        description: "Create a non-destructive curves adjustment layer with ascending input points.",
        parameters: "{points: [[x,y], ...], at least 2 points with strictly ascending x}",
        photocraft_command: "layer.newAdjustmentLayer.curves",
    },
    ImageCommandDescriptor {
        id: "image.smart_filter.prepare",
        label: "Prepare Smart Filters",
        description: "Convert the active layer or selected layers for non-destructive filters.",
        parameters: "{layer?: unsigned integer}",
        photocraft_command: "filter.convertForSmartFilters",
    },
    ImageCommandDescriptor {
        id: "image.smart_filter.gaussian_blur",
        label: "Add Gaussian Blur",
        description: "Add a Gaussian Blur filter; prepare a Smart Object first for non-destructive behavior.",
        parameters: "{radius: 0.1..1000}",
        photocraft_command: "filter.blur.gaussianBlur",
    },
    ImageCommandDescriptor {
        id: "image.smart_filter.set_params",
        label: "Edit Smart Filter",
        description: "Update parameters of an existing Smart Filter by stack index.",
        parameters: "{layer?: unsigned integer, index: unsigned integer, params: object}",
        photocraft_command: "layer.smartFilter.setParams",
    },
    ImageCommandDescriptor {
        id: "image.smart_filter.set_visible",
        label: "Set Smart Filter visibility",
        description: "Show or hide an existing Smart Filter by stack index.",
        parameters: "{layer?: unsigned integer, index: unsigned integer, visible: boolean}",
        photocraft_command: "layer.smartFilter.setVisible",
    },
    ImageCommandDescriptor {
        id: "image.smart_filter.move",
        label: "Move Smart Filter",
        description: "Move an existing Smart Filter to another stack index.",
        parameters: "{layer?: unsigned integer, index: unsigned integer, to: unsigned integer}",
        photocraft_command: "layer.smartFilter.move",
    },
    ImageCommandDescriptor {
        id: "image.smart_filter.delete",
        label: "Delete Smart Filter",
        description: "Delete an existing Smart Filter by stack index.",
        parameters: "{layer?: unsigned integer, index: unsigned integer}",
        photocraft_command: "layer.smartFilter.delete",
    },
    ImageCommandDescriptor {
        id: "image.smart_filter.toggle",
        label: "Toggle Smart Filters",
        description: "Enable or disable the Smart Filter stack on a Smart Object layer.",
        parameters: "{layer?: unsigned integer, enabled: boolean}",
        photocraft_command: "layer.smartFilter.disableSmartFilters",
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

fn validate_blur_radius(radius: f64) -> Result<()> {
    if !radius.is_finite() || !(0.1..=1000.0).contains(&radius) {
        bail!("Gaussian Blur radius must be finite and between 0.1 and 1000");
    }
    Ok(())
}

/// Typed inputs for the first, existing-document image capability pack.
#[derive(Debug, Clone, PartialEq)]
pub enum ImageOperation {
    Invert,
    Crop {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Resize {
        width: i32,
        height: i32,
        resample: ResampleMethod,
    },
    BrightnessContrast {
        brightness: i32,
        contrast: i32,
    },
    CreateLayer {
        name: Option<String>,
        position: LayerPosition,
    },
    MoveLayer {
        layer: u64,
        target: u64,
        position: LayerPosition,
    },
    CreateHueSaturationAdjustmentLayer {
        hue: i32,
        saturation: i32,
    },
    CreateLevelsAdjustmentLayer {
        in_black: u8,
        in_white: u8,
        gamma: f64,
    },
    CreateCurvesAdjustmentLayer {
        points: Vec<(u8, u8)>,
    },
    PrepareSmartFilters {
        layer: Option<u64>,
    },
    AddGaussianBlur {
        radius: f64,
    },
    SetSmartFilterParams {
        layer: Option<u64>,
        index: u32,
        params: Value,
    },
    SetSmartFilterVisibility {
        layer: Option<u64>,
        index: u32,
        visible: bool,
    },
    MoveSmartFilter {
        layer: Option<u64>,
        index: u32,
        to: u32,
    },
    DeleteSmartFilter {
        layer: Option<u64>,
        index: u32,
    },
    ToggleSmartFilters {
        layer: Option<u64>,
        enabled: bool,
    },
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
            Self::CreateHueSaturationAdjustmentLayer { .. } => {
                "image.adjustment_layer.hue_saturation"
            }
            Self::CreateLevelsAdjustmentLayer { .. } => "image.adjustment_layer.levels",
            Self::CreateCurvesAdjustmentLayer { .. } => "image.adjustment_layer.curves",
            Self::PrepareSmartFilters { .. } => "image.smart_filter.prepare",
            Self::AddGaussianBlur { .. } => "image.smart_filter.gaussian_blur",
            Self::SetSmartFilterParams { .. } => "image.smart_filter.set_params",
            Self::SetSmartFilterVisibility { .. } => "image.smart_filter.set_visible",
            Self::MoveSmartFilter { .. } => "image.smart_filter.move",
            Self::DeleteSmartFilter { .. } => "image.smart_filter.delete",
            Self::ToggleSmartFilters { .. } => "image.smart_filter.toggle",
        }
    }

    pub fn params(&self) -> Result<Value> {
        match self {
            Self::Invert => Ok(json!({})),
            Self::Crop {
                x,
                y,
                width,
                height,
            } => {
                if *width <= 0 || *height <= 0 {
                    bail!("image.crop requires positive width and height");
                }
                Ok(json!({"x": x, "y": y, "width": width, "height": height}))
            }
            Self::Resize {
                width,
                height,
                resample,
            } => {
                if *width <= 0 || *height <= 0 {
                    bail!("image.resize requires positive width and height");
                }
                Ok(json!({
                    "width": width,
                    "height": height,
                    "resample": resample.as_str(),
                }))
            }
            Self::BrightnessContrast {
                brightness,
                contrast,
            } => Ok(json!({"brightness": brightness, "contrast": contrast})),
            Self::CreateLayer { name, position } => match name {
                Some(value) if value.trim().is_empty() => {
                    bail!("image.layer.create name must not be empty");
                }
                Some(value) => Ok(json!({"name": value, "position": position.as_str()})),
                None => Ok(json!({"position": position.as_str()})),
            },
            Self::MoveLayer {
                layer,
                target,
                position,
            } => {
                if layer == target {
                    bail!("image.layer.move requires distinct layer and target IDs");
                }
                Ok(json!({"layer": layer, "target": target, "position": position.as_str()}))
            }
            Self::CreateHueSaturationAdjustmentLayer { hue, saturation } => {
                if !(-180..=180).contains(hue) {
                    bail!("hue must be between -180 and 180 degrees");
                }
                if !(-100..=100).contains(saturation) {
                    bail!("saturation must be between -100 and 100");
                }
                Ok(json!({"hue": hue, "saturation": saturation}))
            }
            Self::CreateLevelsAdjustmentLayer {
                in_black,
                in_white,
                gamma,
            } => {
                if in_black >= in_white {
                    bail!("levels in_black must be less than in_white");
                }
                if !gamma.is_finite() || *gamma <= 0.0 {
                    bail!("levels gamma must be a finite positive number");
                }
                Ok(json!({"inBlack": in_black, "inWhite": in_white, "gamma": gamma}))
            }
            Self::CreateCurvesAdjustmentLayer { points } => {
                if points.len() < 2 {
                    bail!("curves requires at least two points");
                }
                if points.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
                    bail!("curves points must have strictly ascending input coordinates");
                }
                Ok(json!({"points": points}))
            }
            Self::PrepareSmartFilters { layer } => {
                let mut params = json!({});
                if let Some(layer) = layer {
                    params["layer"] = json!(layer);
                }
                Ok(params)
            }
            Self::AddGaussianBlur { radius } => {
                validate_blur_radius(*radius)?;
                Ok(json!({"radius": radius}))
            }
            Self::SetSmartFilterParams {
                layer,
                index,
                params,
            } => {
                if !params.is_object() {
                    bail!("Smart Filter params must be a JSON object");
                }
                let mut result = json!({"index": index, "params": params});
                if let Some(layer) = layer {
                    result["layer"] = json!(layer);
                }
                Ok(result)
            }
            Self::SetSmartFilterVisibility {
                layer,
                index,
                visible,
            } => {
                let mut result = json!({"index": index, "visible": visible});
                if let Some(layer) = layer {
                    result["layer"] = json!(layer);
                }
                Ok(result)
            }
            Self::MoveSmartFilter { layer, index, to } => {
                let mut result = json!({"index": index, "to": to});
                if let Some(layer) = layer {
                    result["layer"] = json!(layer);
                }
                Ok(result)
            }
            Self::DeleteSmartFilter { layer, index } => {
                let mut result = json!({"index": index});
                if let Some(layer) = layer {
                    result["layer"] = json!(layer);
                }
                Ok(result)
            }
            Self::ToggleSmartFilters { layer, enabled } => {
                let mut result = json!({"enabled": enabled});
                if let Some(layer) = layer {
                    result["layer"] = json!(layer);
                }
                Ok(result)
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
        let valid = ImageOperation::Crop {
            x: -10,
            y: 4,
            width: 80,
            height: 60,
        };
        assert_eq!(
            valid.params().expect("valid crop"),
            json!({"x": -10, "y": 4, "width": 80, "height": 60})
        );
        let invalid = ImageOperation::Crop {
            x: 0,
            y: 0,
            width: 0,
            height: 10,
        };
        assert!(invalid.params().is_err());
    }

    #[test]
    fn resize_requires_positive_dimensions() {
        let valid = ImageOperation::Resize {
            width: 1280,
            height: 720,
            resample: ResampleMethod::Lanczos,
        };
        assert_eq!(
            valid.params().expect("valid resize"),
            json!({"width": 1280, "height": 720, "resample": "lanczos"})
        );
        assert!(
            ImageOperation::Resize {
                width: -1,
                height: 720,
                resample: ResampleMethod::Bicubic,
            }
            .params()
            .is_err()
        );
    }

    #[test]
    fn layer_move_rejects_a_self_target() {
        assert!(
            ImageOperation::MoveLayer {
                layer: 9,
                target: 9,
                position: LayerPosition::Above,
            }
            .params()
            .is_err()
        );
    }

    #[test]
    fn adjustment_layers_validate_inputs_and_serialize_upstream_params() {
        assert_eq!(
            ImageOperation::CreateHueSaturationAdjustmentLayer {
                hue: -30,
                saturation: 50,
            }
            .params()
            .unwrap(),
            json!({"hue": -30, "saturation": 50})
        );
        assert!(
            ImageOperation::CreateHueSaturationAdjustmentLayer {
                hue: 181,
                saturation: 0,
            }
            .params()
            .is_err()
        );
        assert_eq!(
            ImageOperation::CreateLevelsAdjustmentLayer {
                in_black: 8,
                in_white: 245,
                gamma: 1.05,
            }
            .params()
            .unwrap(),
            json!({"inBlack": 8, "inWhite": 245, "gamma": 1.05})
        );
        assert!(
            ImageOperation::CreateLevelsAdjustmentLayer {
                in_black: 200,
                in_white: 100,
                gamma: 1.0,
            }
            .params()
            .is_err()
        );
        assert!(
            ImageOperation::CreateLevelsAdjustmentLayer {
                in_black: 0,
                in_white: 255,
                gamma: f64::NAN,
            }
            .params()
            .is_err()
        );
    }

    #[test]
    fn curves_points_must_have_ascending_input_coordinates() {
        assert_eq!(
            ImageOperation::CreateCurvesAdjustmentLayer {
                points: vec![(0, 0), (128, 140), (255, 255)],
            }
            .params()
            .unwrap(),
            json!({"points": [[0, 0], [128, 140], [255, 255]]})
        );
        assert!(
            ImageOperation::CreateCurvesAdjustmentLayer {
                points: vec![(0, 0)]
            }
            .params()
            .is_err()
        );
        assert!(
            ImageOperation::CreateCurvesAdjustmentLayer {
                points: vec![(128, 140), (128, 200)]
            }
            .params()
            .is_err()
        );
    }

    #[test]
    fn smart_filter_operations_validate_and_keep_optional_layer_scoping() {
        assert_eq!(
            ImageOperation::PrepareSmartFilters { layer: Some(7) }
                .params()
                .unwrap(),
            json!({"layer": 7})
        );
        assert_eq!(
            ImageOperation::AddGaussianBlur { radius: 3.0 }
                .params()
                .unwrap(),
            json!({"radius": 3.0})
        );
        assert!(
            ImageOperation::AddGaussianBlur {
                radius: f64::INFINITY,
            }
            .params()
            .is_err()
        );
        assert_eq!(
            ImageOperation::SetSmartFilterParams {
                layer: Some(4),
                index: 1,
                params: json!({"radius": 6}),
            }
            .params()
            .unwrap(),
            json!({"layer": 4, "index": 1, "params": {"radius": 6}})
        );
        assert!(
            ImageOperation::SetSmartFilterParams {
                layer: None,
                index: 0,
                params: json!([]),
            }
            .params()
            .is_err()
        );
        assert_eq!(
            ImageOperation::ToggleSmartFilters {
                layer: None,
                enabled: false,
            }
            .params()
            .unwrap(),
            json!({"enabled": false})
        );
        assert_eq!(
            ImageOperation::MoveSmartFilter {
                layer: Some(3),
                index: 0,
                to: 2,
            }
            .params()
            .unwrap(),
            json!({"layer": 3, "index": 0, "to": 2})
        );
        assert_eq!(
            ImageOperation::DeleteSmartFilter {
                layer: None,
                index: 2,
            }
            .params()
            .unwrap(),
            json!({"index": 2})
        );
    }

    #[test]
    fn new_canonical_commands_map_to_verified_photocraft_ids() {
        assert_eq!(
            upstream_command_id("image.adjustment_layer.curves"),
            Some("layer.newAdjustmentLayer.curves")
        );
        assert_eq!(
            upstream_command_id("image.smart_filter.gaussian_blur"),
            Some("filter.blur.gaussianBlur")
        );
        assert_eq!(
            upstream_command_id("image.smart_filter.set_params"),
            Some("layer.smartFilter.setParams")
        );
        assert_eq!(IMAGE_COMMAND_CATALOG.len(), 16);
    }

    #[test]
    fn catalog_maps_canonical_ids_to_verified_photocraft_ids() {
        assert_eq!(
            upstream_command_id("image.invert"),
            Some("image.adjustments.invert")
        );
        assert_eq!(
            upstream_command_id("image.layer.move"),
            Some("layer.moveTo")
        );
        assert_eq!(upstream_command_id("image.not_supported"), None);
        assert_eq!(IMAGE_CAPABILITY_PROTOCOL, "praxilume.image/v1");
    }
}
