use serde::{Deserialize, Serialize};

/// How DDS files are compressed. BC4 (one channel) and BC5 (two channels) are picked per map by
/// [`DdsCompression::Auto`]; BC7 is not written
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DdsCompression {
    /// Per map: normal maps BC5 (X and Y; the renderer rebuilds Z), maps with alpha BC3,
    /// grayscale maps BC4, the other colour maps BC1
    #[default]
    Auto,
    /// Uncompressed and lossless: 8 or 16 bits per channel, as the bit depth says
    None,
    /// BC1 (DXT1), 4 bits per pixel; alpha is only on or off
    Bc1,
    /// BC3 (DXT5), 8 bits per pixel with smooth alpha
    Bc3,
}

impl DdsCompression {
    pub const ALL: [DdsCompression; 4] = [DdsCompression::Auto, DdsCompression::None, DdsCompression::Bc1, DdsCompression::Bc3];

    /// The name commands use: `auto`, `none`, `bc1` or `bc3`
    pub fn name(self) -> &'static str {
        match self {
            DdsCompression::Auto => "auto",
            DdsCompression::None => "none",
            DdsCompression::Bc1 => "bc1",
            DdsCompression::Bc3 => "bc3",
        }
    }

    /// [`Self::name`] back to a compression, also taking `dxt1` and `dxt5`, in any case
    pub fn from_name(name: &str) -> Option<DdsCompression> {
        match name.to_ascii_lowercase().as_str() {
            "auto" => Some(DdsCompression::Auto),
            "none" => Some(DdsCompression::None),
            "bc1" | "dxt1" => Some(DdsCompression::Bc1),
            "bc3" | "dxt5" => Some(DdsCompression::Bc3),
            _ => None,
        }
    }

    /// The compression a map of `n` components (1 L, 2 LA, 3 RGB, 4 RGBA) is written with;
    /// `normal` for a normal map
    pub(super) fn resolve(self, normal: bool, n: usize) -> craft_dds::DdsCompression {
        use craft_dds::DdsCompression as C;
        match self {
            DdsCompression::Auto if normal && n == 3 => C::Bc5,
            DdsCompression::Auto if matches!(n, 2 | 4) => C::Bc3,
            DdsCompression::Auto if n == 1 => C::Bc4,
            DdsCompression::Auto => C::Bc1,
            DdsCompression::None => C::None,
            DdsCompression::Bc1 => C::Bc1,
            DdsCompression::Bc3 => C::Bc3,
        }
    }
}
