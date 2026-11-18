use serde::{Deserialize, Serialize};

/// How a stencil sits on screen, without the image (which is a project resource)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StencilLook {
    pub center: [f32; 2],
    pub scale: f32,
    pub rotation: f32,
    pub tile: bool,
    pub opacity: f32,
}

impl Default for StencilLook {
    fn default() -> Self {
        StencilLook { center: [0.0, 0.0], scale: 0.8, rotation: 0.0, tile: false, opacity: 0.5 }
    }
}
