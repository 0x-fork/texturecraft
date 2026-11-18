use serde::{Deserialize, Serialize};

use super::DdsCompression;

/// What DDS files are written with
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DdsExport {
    pub compression: DdsCompression,
    /// Also write the full mip chain down to 1×1 (each level a box filter of the one above)
    pub mipmaps: bool,
}
