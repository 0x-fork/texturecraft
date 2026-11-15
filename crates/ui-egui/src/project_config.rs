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
    app.ui.windows.project_config = Some(ProjectConfig { mesh: None, reimport: false, project: app.session.project_id });
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

pub(crate) fn show(app: &mut TextureCraftApp, ctx: &egui::Context) {
    let Some(mut config) = app.ui.windows.project_config.clone() else { return };
    // Another project was opened meanwhile: the choices were for the old one.
    if config.project != app.session.project_id {
        app.ui.windows.project_config = None;
        return;
    }
    let current = app.session.doc.project.mesh_path.clone();
    // A file other than the project's own is the project's new mesh.
    let changed = config.mesh.as_ref().filter(|chosen| **chosen != current).cloned();

    let mut open = true;
    let (mut ok, mut cancel, mut select) = (false, false, false);
    let font = egui::TextStyle::Body.resolve(&ctx.style());
    crate::windows::dialog("project_config", tr("Project configuration"), &mut open).default_width(460.0).show(ctx, |ui| {
        // One section per part of the project; more follow the same pattern.
        ui.label(caption(tr("3D MESH")));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(tr("3D mesh"));
            let shown = changed.clone().unwrap_or_else(|| current.clone());
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
        ui.add_space(4.0);
        // Another file always reads the mesh again, so its box is ticked and stays ticked.
        let mut reimport = config.reimport || changed.is_some();
        ui.add_enabled(changed.is_none(), egui::Checkbox::new(&mut reimport, tr("Reimport mesh")))
            .on_hover_text(tr("Read the mesh file again, keeping the layers and painted strokes"));
        config.reimport = reimport;
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ok = ui.add(primary_button(ui, tr("OK"))).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
    });
    // Esc cancels; Enter confirms unless a button has the keyboard (it presses that one itself).
    let nothing_focused = ctx.memory(|m| m.focused()).is_none();
    ctx.input_mut(|i| {
        cancel |= i.consume_key(Modifiers::NONE, Key::Escape);
        ok |= nothing_focused && i.consume_key(Modifiers::NONE, Key::Enter);
    });

    if select {
        let dialog = FileDialog::open().filter(tr("3D meshes"), &texturecraft_engine::formats::mesh_import::MESH_EXTENSIONS);
        app.ask_file(dialog, Bound::Project, |app, paths| {
            let Some(path) = paths.first().map(|p| p.display().to_string()) else { return };
            let current = app.session.doc.project.mesh_path.clone();
            if let Some(config) = app.ui.windows.project_config.as_mut() {
                // Picking the project's own file again drops an earlier choice.
                config.mesh = (path != current).then_some(path);
                config.reimport |= config.mesh.is_some();
            }
        });
    }
    if ok || cancel || !open {
        app.ui.windows.project_config = None;
    } else {
        // Keep a tick made now; the file answer arrives later through the stored value.
        if let Some(stored) = app.ui.windows.project_config.as_mut() {
            stored.reimport = config.reimport;
        }
    }
    if ok {
        if let Some(path) = changed {
            app.run("file.changeMesh", json!({"path": path, "preserveStrokes": true}));
        } else if config.reimport {
            crate::menus::invoke(app, ctx, "file.reimportMesh");
        }
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
        assert_eq!(shown, "…/hero.obj", "the longest tail that fits, not a shorter one");
    }
}
