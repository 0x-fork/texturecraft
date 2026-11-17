use std::collections::BTreeSet;
use std::path::PathBuf;

/// What an imported resource is for
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ResourceUsage {
    /// Imported as an image resource, nothing more
    #[default]
    Undefined,
    /// An image resource that also becomes the brush alpha
    Alpha,
    /// Color lookup tables (not available yet)
    ColorLut,
    /// An image resource that also lights the scene
    Environment,
    /// Post effect resources (not available yet)
    PostFx,
    /// An image resource for fills and stencils
    Texture,
}

impl ResourceUsage {
    /// The usages in the dropdown's order
    pub const ALL: [ResourceUsage; 6] = [Self::Undefined, Self::Alpha, Self::ColorLut, Self::Environment, Self::PostFx, Self::Texture];

    /// The name shown in the dropdowns (the same in every language, as in the reference)
    pub fn name(self) -> &'static str {
        match self {
            Self::Undefined => "undefined",
            Self::Alpha => "alpha",
            Self::ColorLut => "colorlut",
            Self::Environment => "environment",
            Self::PostFx => "postfx",
            Self::Texture => "texture",
        }
    }

    /// Whether the app can do something with resources of this usage
    pub fn available(self) -> bool {
        !matches!(self, Self::ColorLut | Self::PostFx)
    }
}

/// Where the resources go
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImportTarget {
    /// The open project's resources
    #[default]
    Session,
    /// The user's asset library (not available for images yet)
    Library,
}

/// One file waiting in the window
#[derive(Clone, Debug, PartialEq)]
pub struct ImportRow {
    pub id: u64,
    pub path: PathBuf,
    pub prefix: String,
    pub usage: ResourceUsage,
}

/// File > Import resources: the files added so far, imported by the Import button
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ImportResources {
    pub rows: Vec<ImportRow>,
    /// Ids of the selected rows
    pub selected: BTreeSet<u64>,
    /// The row a Shift click extends from
    pub anchor: Option<u64>,
    /// The usage the list is narrowed to (None: all)
    pub filter: Option<ResourceUsage>,
    pub target: ImportTarget,
    /// Whether Enter requested the import in the next frame
    pub confirm: bool,
    /// The project that was open when this window opened
    pub project: u64,
    /// This opening's identity, used to reject late file picker answers
    pub(crate) opening_id: u64,
    pub(crate) next_row: u64,
}
