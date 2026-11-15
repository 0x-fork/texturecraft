//! Edit > Project Configuration...: the project's settings in sections. Today it has one, the
//! 3D mesh: pick another mesh file for the project, or read the current one again.

use egui::{Key, Modifiers};
use serde_json::{Value, json};
use texturecraft_engine::core::NormalFormat;

use crate::TextureCraftApp;
use crate::file_dialogs::{Bound, FileDialog};
use crate::i18n::{tr, trf};
use crate::state::ProjectConfig;
use crate::theme::caption;
use crate::widgets::{combo, primary_button};

/// Room for the Select... button beside the path.
const SELECT_BUTTON_WIDTH: f32 = 90.0;

/// Opens the window on the current project, with nothing chosen yet.
pub(crate) fn open(app: &mut TextureCraftApp) {
    app.ui.windows.project_config = Some(ProjectConfig { mesh: None, reimport: false, preserve: true, normal_format: None, confirm: false, project: app.session.project_id });
}

/// `text` with its beginning replaced by an ellipsis until `fits` accepts it (the end of a path
/// is the part that tells files apart).
fn elide_start(text: &str, fits: impl Fn(&str) -> bool) -> String {
    if fits(text) {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let shown = |from: usize| format!("…{}", chars[from..].iter().collect::<String>());
    // Dropping more characters only ever makes it shorter, so the first start that fits is found by bisection.
    let (mut low, mut high) = (1, chars.len());
    while low < high {
        let mid = low + (high - low) / 2;
        if fits(&shown(mid)) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    shown(low)
}

/// The keys of the open window, read in `menus::shortcuts` before any panel draws: Escape closes
/// it (and goes no further, so it doesn't cancel a stroke), Enter confirms it unless a text field
/// or button had the keyboard (`nothing_focused`), whose Enter belongs to it.
pub(crate) fn keys_in(app: &mut TextureCraftApp, input: &mut egui::InputState, nothing_focused: bool) {
    let escape = input.consume_key(Modifiers::NONE, Key::Escape);
    let enter = input.consume_key(Modifiers::NONE, Key::Enter);
    if escape {
        app.ui.windows.project_config = None;
    } else if enter
        && nothing_focused
        && let Some(config) = app.ui.windows.project_config.as_mut()
    {
        config.confirm = true;
    }
}

/// The command OK runs for the mesh: the project moves to `chosen`, or `reimport` (the project's
/// own file, when Reimport mesh is ticked) is read again. `preserve` is the Import settings box.
fn mesh_command(chosen: Option<&str>, reimport: Option<&str>, preserve: bool) -> Option<(&'static str, Value)> {
    match (chosen, reimport) {
        (Some(path), _) => Some(("file.changeMesh", json!({"path": path, "preserveStrokes": preserve}))),
        (None, Some(path)) => Some(("file.reimportMesh", json!({"path": path, "preserveStrokes": preserve}))),
        (None, None) => None,
    }
}

fn format_name(format: NormalFormat) -> &'static str {
    match format {
        NormalFormat::OpenGl => "OpenGL",
        NormalFormat::DirectX => "DirectX",
    }
}

pub(crate) fn show(app: &mut TextureCraftApp, ctx: &egui::Context) {
    let Some(mut config) = app.ui.windows.project_config.clone() else { return };
    // Another project was opened meanwhile: the choices were for the old one.
    if config.project != app.session.project_id {
        app.ui.windows.project_config = None;
        return;
    }
    let project = &app.session.doc.project;
    let current = project.mesh_path.clone();
    // What the project's mesh came from: its file, or at least the name it was imported under.
    let named = if current.is_empty() { project.mesh_file.clone() } else { current.clone() };
    let readable = app.session.remembered_mesh_path().is_some();
    let normal_format = project.normal_format;
    // Each texture set's UV tiles, as `1001, 1002` (None: the set has none).
    let tiles: Vec<(String, Option<String>)> = project
        .texture_sets
        .iter()
        .map(|set| (set.name.clone(), (!set.uv_tiles.is_empty()).then(|| set.uv_tiles.iter().map(u32::to_string).collect::<Vec<_>>().join(", "))))
        .collect();
    // A file other than the project's own is the project's new mesh.
    let changed = config.mesh.as_ref().filter(|chosen| **chosen != current).cloned();

    let mut open = true;
    let (mut ok, mut cancel, mut select) = (std::mem::take(&mut config.confirm), false, false);
    crate::windows::dialog("project_config", tr("Project configuration"), &mut open).default_width(460.0).show(ctx, |ui| {
        // One section per part of the project; more follow the same pattern.
        ui.label(caption(tr("3D MESH")));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(tr("3D mesh"));
            let shown = changed.clone().unwrap_or_else(|| named.clone());
            let font = egui::TextStyle::Body.resolve(ui.style());
            let max = (ui.available_width() - SELECT_BUTTON_WIDTH).max(60.0);
            let measure = |s: &str| ui.painter().layout_no_wrap(s.to_string(), font.clone(), egui::Color32::WHITE).rect.width() <= max;
            if shown.is_empty() {
                ui.weak(tr("Built-in mesh"));
            } else {
                ui.label(elide_start(&shown, measure)).on_hover_text(&shown);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                select = ui.button(tr("Select...")).clicked();
            });
        });
        if changed.is_none() && !current.is_empty() && !readable {
            ui.colored_label(ui.visuals().error_fg_color, tr("File not found"));
        }
        ui.add_space(4.0);
        // Another file always reads the mesh again, so its box shows ticked and can't be cleared.
        // Without a file on disk there is nothing to read again.
        let mut reimport = config.reimport || changed.is_some();
        let free = changed.is_none() && readable;
        let tick = ui.add_enabled(free, egui::Checkbox::new(&mut reimport, tr("Reimport mesh")));
        if free {
            config.reimport = reimport;
            tick.on_hover_text(tr("Read the mesh file again, keeping the layers and painted strokes"));
        } else if changed.is_none() {
            tick.on_disabled_hover_text(tr("There is no mesh file to read again. Select one."));
        }
        ui.add_space(8.0);
        ui.label(caption(tr("Import settings")));
        // Only a mesh that is going to be read has settings to apply.
        let importing = changed.is_some() || (config.reimport && readable);
        ui.add_enabled(importing, egui::Checkbox::new(&mut config.preserve, tr("Preserve stroke positions on mesh")))
            .on_hover_text(tr("Painted strokes follow the surface when the new mesh's UVs differ"));

        ui.add_space(12.0);
        ui.label(caption(tr("PROJECT SETTINGS")));
        ui.add_space(4.0);
        let shown_format = config.normal_format.unwrap_or(normal_format);
        combo(ui, tr("Normal map format"), format_name(shown_format), |ui| {
            for format in [NormalFormat::OpenGl, NormalFormat::DirectX] {
                if ui.selectable_label(format == shown_format, format_name(format)).clicked() {
                    config.normal_format = (format != normal_format).then_some(format);
                }
            }
        })
        .on_hover_text(tr(
            "For the whole project: how normal map images set as mesh maps are read (set them again after a change), and the default for export",
        ));
        ui.add_space(4.0);
        // Read only: the tiles are set when the project is created.
        ui.label(tr("UV Tile settings (UDIMs)"));
        if tiles.iter().all(|(_, t)| t.is_none()) {
            ui.label(egui::RichText::new(tr("Off")).weak());
        }
        for (name, t) in tiles.iter().filter(|(_, t)| t.is_some()) {
            let list = t.as_deref().unwrap_or_default();
            ui.label(egui::RichText::new(if tiles.len() > 1 { trf("{set}: {tiles}", &[("set", name), ("tiles", list)]) } else { list.to_string() }).weak());
        }
        ui.label(egui::RichText::new(tr("You can't change UV Tile settings once a project has been created.")).small().weak());

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ok |= ui.add(primary_button(ui, tr("OK"))).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
    });

    if select {
        let dialog = FileDialog::open().filter(tr("3D meshes"), &texturecraft_engine::formats::mesh_import::MESH_EXTENSIONS);
        app.ask_file(dialog, Bound::Project, |app, paths| {
            let Some(path) = paths.first().map(|p| p.display().to_string()) else { return };
            let current = app.session.doc.project.mesh_path.clone();
            if let Some(config) = app.ui.windows.project_config.as_mut() {
                // Picking the project's own file again drops an earlier choice.
                config.mesh = (path != current).then_some(path);
            }
        });
    }
    // OK closes the window once everything worked; a failure leaves it open, choices kept, with its toast.
    // The mesh goes first: a project that couldn't change its mesh stays as it was.
    let done = ok && {
        let reimport = config.reimport.then(|| app.session.remembered_mesh_path().map(str::to_string)).flatten();
        let mesh_done = match mesh_command(changed.as_deref(), reimport.as_deref(), config.preserve) {
            Some((command, params)) => app.run(command, params).is_some(),
            None => true,
        };
        mesh_done
            && match config.normal_format {
                Some(format) => app.run("textureSet.settings", json!({"normalFormat": if format == NormalFormat::DirectX { "directX" } else { "openGl" }})).is_some(),
                None => true,
            }
    };
    if done || cancel || !open {
        app.ui.windows.project_config = None;
    } else if let Some(stored) = app.ui.windows.project_config.as_mut() {
        // Keep the choices made now; the file answer arrives later through the stored value.
        (stored.reimport, stored.preserve, stored.normal_format, stored.confirm) = (config.reimport, config.preserve, config.normal_format, false);
    }
}

#[cfg(test)]
mod tests {
    use super::{elide_start, mesh_command};

    #[test]
    fn a_path_that_fits_is_unchanged() {
        assert_eq!(elide_start("/a/b.obj", |s| s.chars().count() <= 20), "/a/b.obj");
    }

    #[test]
    fn a_long_path_loses_its_beginning() {
        let shown = elide_start("/home/me/projects/models/hero.obj", |s| s.chars().count() <= 12);
        assert_eq!(shown, "…ls/hero.obj", "the longest tail that fits, not a shorter one");
    }

    #[test]
    fn the_import_setting_goes_to_the_command() {
        for preserve in [true, false] {
            let (command, params) = mesh_command(Some("/a/b.obj"), None, preserve).unwrap();
            assert_eq!((command, params["preserveStrokes"].as_bool()), ("file.changeMesh", Some(preserve)));
            let (command, params) = mesh_command(None, Some("/a/c.obj"), preserve).unwrap();
            assert_eq!((command, params["preserveStrokes"].as_bool(), params["path"].as_str()), ("file.reimportMesh", Some(preserve), Some("/a/c.obj")));
        }
        assert!(mesh_command(None, None, true).is_none(), "nothing to read");
    }
}
