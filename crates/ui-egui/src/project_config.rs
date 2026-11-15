//! Edit > Project Configuration...: the project's settings in sections. Today it has one, the
//! 3D mesh: pick another mesh file for the project, or read the current one again.

use egui::{Key, Modifiers};
use serde_json::json;

use crate::TextureCraftApp;
use crate::file_dialogs::{Bound, FileDialog};
use crate::i18n::tr;
use crate::state::ProjectConfig;
use crate::theme::caption;
use crate::widgets::primary_button;

/// Room for the Select... button beside the path.
const SELECT_BUTTON_WIDTH: f32 = 90.0;

/// Opens the window on the current project, with nothing chosen yet.
pub(crate) fn open(app: &mut TextureCraftApp) {
    app.ui.windows.project_config = Some(ProjectConfig { mesh: None, reimport: false, confirm: false, project: app.session.project_id });
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
    // OK closes the window once the change worked; a failure leaves it open, choice kept, with its toast.
    let done = ok
        && if let Some(path) = changed {
            app.run("file.changeMesh", json!({"path": path, "preserveStrokes": true})).is_some()
        } else if config.reimport
            && let Some(path) = app.session.remembered_mesh_path().map(str::to_string)
        {
            app.run("file.reimportMesh", json!({"path": path, "preserveStrokes": true})).is_some()
        } else {
            true
        };
    if done || cancel || !open {
        app.ui.windows.project_config = None;
    } else if let Some(stored) = app.ui.windows.project_config.as_mut() {
        // Keep the tick made now; the file answer arrives later through the stored value.
        stored.reimport = config.reimport;
    }
}

#[cfg(test)]
mod tests {
    use super::elide_start;

    #[test]
    fn a_path_that_fits_is_unchanged() {
        assert_eq!(elide_start("/a/b.obj", |s| s.chars().count() <= 20), "/a/b.obj");
    }

    #[test]
    fn a_long_path_loses_its_beginning() {
        let shown = elide_start("/home/me/projects/models/hero.obj", |s| s.chars().count() <= 12);
        assert_eq!(shown, "…ls/hero.obj", "the longest tail that fits, not a shorter one");
    }
}
