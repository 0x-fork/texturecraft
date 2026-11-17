//! The app starts with no project and no popup; File > Close goes back to that empty state.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;
use texturecraft_engine::Session;
use texturecraft_ui_egui::{TextureCraftApp, menus};

fn harness(session: Session) -> Harness<'static, TextureCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 960.0)).build_eframe(|cc| TextureCraftApp::new(cc, session));
    h.run_steps(4);
    h
}

fn empty() -> Harness<'static, TextureCraftApp> {
    harness(Session::closed().unwrap())
}

fn open_project() -> Harness<'static, TextureCraftApp> {
    harness(texturecraft_engine::sample_session(256, 64).unwrap())
}

/// Texts painted this frame as (text, left edge)
fn painted(h: &Harness<'static, TextureCraftApp>) -> Vec<(String, f32)> {
    h.output()
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some((text.galley.text().to_string(), text.pos.x)),
            _ => None,
        })
        .collect()
}

fn has_camera_picker(h: &Harness<'static, TextureCraftApp>) -> bool {
    painted(h).iter().any(|(text, _)| text == "Default camera")
}

fn invoke(h: &mut Harness<'static, TextureCraftApp>, id: &str) {
    let ctx = h.ctx.clone();
    menus::invoke(h.state_mut(), &ctx, id);
    h.run_steps(3);
}

#[test]
fn starts_with_no_project_and_no_popup() {
    let h = empty();
    let ui = &h.state().ui;
    assert!(!h.state().session.project_open);
    assert!(!ui.windows.new_project && !ui.windows.about && !ui.windows.settings && !ui.windows.export, "no window opens by itself");
    assert!(h.query_by_label("Continue with the sample project").is_none() && h.query_by_label("Recent projects").is_none(), "no start window");
    assert!(h.query_by_label("No properties found").is_some(), "Properties says it has nothing");
}

#[test]
fn project_items_are_disabled_without_a_project() {
    let h = empty();
    let app = h.state();
    for id in [
        "file.save",
        "file.saveAs",
        "file.saveCopy",
        "file.close",
        "ui.export",
        "file.exportMesh",
        "ui.projectConfig",
        "edit.undo",
        "edit.redo",
        "file.reimportMesh",
        "ui.mode.bake",
        "ui.mode.render",
        "ui.uvChecker",
        "viewport.frame",
        "viewport.displayMaterial",
        "tool.paint",
        "tool.eraser",
        "viewport.quickMask",
        "layer.add.paint",
        "layer.add.fill",
        "layer.remove",
        "bake.run",
        "resource.import",
        "material.load",
    ] {
        assert!(!menus::enabled(app, id), "{id} needs a project");
    }
    for id in ["ui.newProject", "file.open", "project.sample", "ui.settings", "ui.about", "ui.quit", "ui.mode.paint", "ui.layout.split"] {
        assert!(menus::enabled(app, id), "{id} works without a project");
    }
    let open = open_project();
    for id in ["file.save", "file.close", "ui.export", "layer.add.paint", "ui.mode.bake"] {
        assert!(menus::enabled(open.state(), id), "{id} is on with a project");
    }
}

#[test]
fn the_empty_views_keep_the_view_picker_and_drop_the_camera_picker() {
    let empty = empty();
    assert!(!has_camera_picker(&empty), "no camera picker");
    // The picker shows "Material" right of the tool shelf and the Assets panel: one per view in the split layout
    let pickers = painted(&empty).iter().filter(|(text, x)| text == "Material" && *x > 400.0).count();
    assert_eq!(pickers, 2, "each of the two views keeps its view picker");
    let open = open_project();
    assert!(has_camera_picker(&open), "with a project the camera picker is there");
}

#[test]
fn the_status_bar_has_the_version_but_no_frame_time_or_layer_memory() {
    for h in [empty(), open_project()] {
        assert!(h.query_by_label_contains("Version:").is_some());
        assert!(h.query_by_label_contains("Layer memory").is_none());
        assert!(h.query_by_label_contains(" ms").is_none(), "no frame time");
    }
}

#[test]
fn close_unloads_the_project_back_to_the_empty_state() {
    let mut h = open_project();
    assert!(h.state().session.project_open && h.state().session.set_count() == 2);
    invoke(&mut h, "file.close");
    assert!(!h.state().session.project_open, "Close leaves no project open");
    assert!(h.state().session.path.is_none() && !h.state().session.modified);
    assert!(h.query_by_label("No properties found").is_some());
    assert!(!has_camera_picker(&h));
    assert!(!menus::enabled(h.state(), "file.save"));
}

#[test]
fn creating_or_opening_a_project_from_the_empty_state_works() {
    let mut h = empty();
    assert!(h.state_mut().run("project.new", json!({"mesh": "cube", "resolution": 128})).is_some());
    h.run_steps(3);
    assert!(h.state().session.project_open);
    assert!(menus::enabled(h.state(), "file.save") && has_camera_picker(&h));

    invoke(&mut h, "file.close");
    invoke(&mut h, "ui.newProject");
    assert!(h.state().ui.windows.new_project, "New project opens its dialog from the empty state");
    let mut h = empty();
    invoke(&mut h, "project.sample");
    assert!(h.state().session.project_open && h.state().session.set_count() == 2, "the sample is still one menu click away");
}

#[test]
fn using_every_panel_and_menu_item_in_the_empty_state_does_not_panic() {
    let mut h = empty();
    for item in menus::CATALOG
        .iter()
        .filter(|item| item.label != "-" && !matches!(item.id, "ui.quit" | "file.open" | "project.sample" | "ui.layout.load" | "ui.help.checkForUpdates"))
    {
        invoke(&mut h, item.id);
    }
    for mode in ["ui.mode.bake", "ui.mode.render", "ui.mode.paint"] {
        invoke(&mut h, mode);
    }
    for id in ["ui.layout.2d", "ui.layout.3d", "ui.layout.split"] {
        invoke(&mut h, id);
    }
    assert!(!h.state().session.project_open, "none of these opened a project");
    assert!(!h.state().session.can_undo(), "and none changed it");
}
