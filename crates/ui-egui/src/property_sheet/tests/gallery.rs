//! What the gallery's controls last set

use super::Filtering;

pub(super) struct Gallery {
    pub(super) filtering: Filtering,
    pub(super) position: [f32; 3],
    pub(super) position_merges: Vec<bool>,
    pub(super) linked: bool,
}

impl Default for Gallery {
    fn default() -> Self {
    }
}
