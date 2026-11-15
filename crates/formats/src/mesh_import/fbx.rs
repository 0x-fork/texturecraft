//! FBX scene import through the shared bounded reader.
use super::{Corner, MAX_MESH_FILE_BYTES, MAX_TEXTURE_BYTES, MAX_ZIP_ENTRIES, PolygonSink, RawBaseColor, RawCamera, RawMesh};
use crate::{FormatError, Result, malformed};
use texturecraft_geom::glam::{Mat3, Mat4, Vec2, Vec3, Vec4};
const KIND: &str = "FBX";
type Files<'a> = dyn FnMut(&[&str]) -> Option<Vec<u8>> + 'a;
fn limits() -> craft_mesh::Limits {
    craft_mesh::Limits {
        max_vertices: super::MAX_VERTICES,
        max_triangles: texturecraft_geom::MAX_TRIANGLES,
        max_face_corners: super::MAX_VERTICES,
        max_meshes: super::MAX_VERTICES,
        max_parts: super::MAX_VERTICES,
        max_materials: super::MAX_VERTICES,
        max_nodes: super::MAX_VERTICES,
        max_data_bytes: 1 << 31,
        max_extras: 100_000,
        ..Default::default()
    }
}

fn package_limits() -> craft_mesh::fbx::package::PackageLimits {
    craft_mesh::fbx::package::PackageLimits {
        max_entries: MAX_ZIP_ENTRIES as u64,
        max_model_bytes: MAX_MESH_FILE_BYTES,
        max_texture_bytes: MAX_TEXTURE_BYTES,
        max_total_bytes: MAX_MESH_FILE_BYTES,
    }
}

fn convert(e: craft_mesh::Error) -> FormatError {
    match e {
        craft_mesh::Error::TooLarge(s) => FormatError::TooLarge(s),
        craft_mesh::Error::Unsupported(s) => FormatError::Unsupported(s),
        e => FormatError::Invalid(e.to_string()),
    }
}

pub(crate) fn read(bytes: &[u8]) -> Result<RawMesh> {
    crate::guard(KIND, || {
        if !craft_mesh::fbx::package::is_package(bytes) {
            return read_inner(bytes, &mut |_| None);
        }
        let mut package = craft_mesh::fbx::package::Package::open(bytes, package_limits()).map_err(convert)?;
        let fbx = package.model().map_err(convert)?;
        read_inner(&fbx, &mut |paths| package.texture(paths))
    })
}

pub(crate) fn package(file_name: &str, bytes: &[u8], found: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> Result<(Option<Vec<u8>>, Vec<String>)> {
    if craft_mesh::fbx::package::is_package(bytes) {
        return Ok((None, Vec::new()));
    }
    crate::guard(KIND, || {
        let limits = limits();
        let doc = craft_mesh::fbx::read_document(bytes, &limits).map_err(convert)?;
        if doc.version == 0 {
            return Err(malformed(KIND, "not an FBX file (no FBXHeaderExtension / FBXVersion)"));
        }
        doc.scene_with_options(&limits, &mut craft_mesh::no_external, &craft_mesh::fbx::SceneOptions { apply_inactive_rotation: true, strict_geometry: true })
            .map_err(convert)?;
        craft_mesh::fbx::package::write(&doc, file_name, bytes, &limits, package_limits(), found).map_err(convert)
    })
}

fn read_inner(bytes: &[u8], files: &mut Files) -> Result<RawMesh> {
    let limits = limits();
    let doc = craft_mesh::fbx::read_document(bytes, &limits).map_err(convert)?;
    if doc.version == 0 {
        return Err(malformed(KIND, "not an FBX file (no FBXHeaderExtension / FBXVersion)"));
    }
    let colors = doc.diffuse_materials(&limits, files).map_err(convert)?;
    let scene = doc
        .scene_with_options(&limits, &mut craft_mesh::no_external, &craft_mesh::fbx::SceneOptions { apply_inactive_rotation: true, strict_geometry: true })
        .map_err(convert)?;
    let to_mat = |m: craft_mesh::Mat4| Mat4::from_cols_array(&m.map(|v| v as f32));
    let global = to_mat(doc.axes_to_y_up()) * Mat4::from_scale(Vec3::splat((doc.unit_scale().unwrap_or(1.0) / 100.0) as f32));
    let mut sink = PolygonSink::default();
    let mut cameras = Vec::new();
    let mut stack: Vec<_> = scene.roots.iter().rev().map(|i| (*i, global, String::new(), 0usize)).collect();
    let mut visited = 0usize;
    let mut points = 0usize;
    while let Some((id, parent, inherited, depth)) = stack.pop() {
        visited += 1;
        if visited > limits.max_nodes || depth > limits.max_depth {
            return Err(FormatError::TooLarge("FBX hierarchy exceeds its traversal limit".into()));
        }
        let node = scene.nodes.get(id).ok_or_else(|| malformed(KIND, "node index out of range"))?;
        let world = parent * to_mat(node.transform);
        let part = if node.name.ends_with(" geometry") { inherited } else { node.name.clone() };
        for child in node.children.iter().rev() {
            stack.push((*child, world, part.clone(), depth + 1));
        }
        if let Some(index) = node.camera {
            if cameras.len() < super::MAX_CAMERAS {
                if let Some(camera) = scene.cameras.get(index) {
                    let (fov, ortho) = match camera.projection {
                        craft_mesh::Projection::Perspective { yfov, .. } => (yfov.to_degrees() as f32, None),
                        craft_mesh::Projection::Orthographic { ymag, .. } => (40.0, Some(ymag as f32)),
                    };
                    cameras.push(RawCamera::placed(camera.name.clone(), world, Vec3::NEG_Z, fov, ortho));
                }
            }
        }
        let Some(index) = node.mesh else { continue };
        let mesh = scene.meshes.get(index).ok_or_else(|| malformed(KIND, "mesh index out of range"))?;
        points = points.checked_add(mesh.positions.len()).ok_or_else(|| FormatError::TooLarge("FBX instance work overflow".into()))?;
        if points > 200_000_000 {
            return Err(FormatError::TooLarge("FBX instances exceed the 200000000 control-point work limit".into()));
        }
        let normal_transform = Mat3::from_mat4(world).inverse().transpose();
        let flip = world.determinant() < 0.0;
        sink.part = part;
        for group in &mesh.parts {
            let material = group.material.and_then(|m| scene.materials.get(m)).map_or("Default", |m| m.name.as_str());
            for face in group.faces() {
                let mut corners = Vec::with_capacity(face.len());
                for c in face {
                    let p = mesh.positions.get(c.position as usize).ok_or_else(|| malformed(KIND, "position index out of range"))?;
                    let p = world.transform_point3(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32));
                    let n = mesh.normals.get(c.normal as usize).map(|n| normal_transform * Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32));
                    let uv = mesh.uv_sets.first().and_then(|set| set.get(c.uv as usize)).map(|v| Vec2::new(v[0], v[1]));
                    let color = mesh.colors.get(c.position as usize).copied().map(Vec4::from_array);
                    corners.push(Corner { p, n, uv, color });
                }
                if flip {
                    corners.reverse();
                }
                sink.polygon(material, &corners)?;
            }
        }
    }
    if visited < scene.nodes.len() {
        return Err(malformed(KIND, "cycle in model hierarchy"));
    }
    for warning in scene.warnings {
        sink.warn(warning);
    }
    let mut raw = sink.finish();
    raw.cameras = cameras;
    raw.base_colors = colors.into_iter().map(|c| RawBaseColor { material: c.name, factor: Vec4::from_array(c.factor), image: c.image }).collect();
    Ok(raw)
}
