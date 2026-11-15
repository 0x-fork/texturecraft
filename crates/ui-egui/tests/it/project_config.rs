//! Edit > Project Configuration...: the 3D mesh section. File pickers are answered by the
//! override, never shown.

use std::path::PathBuf;

use egui::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;
use texturecraft_engine::Session;
use texturecraft_ui_egui::TextureCraftApp;
use texturecraft_ui_egui::file_dialogs::{Answer, Request};

const QUAD: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf 1/1 2/2 3/3 4/4\n";
const TRIANGLE: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nf 1/1 2/2 3/3\n";
const TITLE: &str = "Project configuration";

/// A new folder for one test.
fn temp_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("texturecraft-project-config-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An app whose project is the quad in `dir/quad.obj` with a fill layer on it.
fn app_on_a_quad(dir: &std::path::Path) -> Harness<'static, TextureCraftApp> {
    let file = dir.join("quad.obj");
    std::fs::write(&file, QUAD).unwrap();
    let mut session = Session::from_mesh_bytes("quad.obj", QUAD.as_bytes(), 128, "quad").unwrap();
    texturecraft_engine::execute(&mut session, "layer.add.fill", &json!({"color": [1.0, 0.0, 0.0]})).unwrap();
    session.doc.project.mesh_path = file.display().to_string();
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 960.0)).build_eframe(|cc| TextureCraftApp::new(cc, session));
    h.state_mut().ui.prefs.language = "en".into();
    h.run_steps(4);
    h
}

fn open_window(h: &mut Harness<'_, TextureCraftApp>) {
    let ctx = h.ctx.clone();
    texturecraft_ui_egui::menus::invoke(h.state_mut(), &ctx, "ui.projectConfig");
    h.run_steps(3);
}

fn window_is_open(h: &Harness<'_, TextureCraftApp>) -> bool {
    h.query_by_label(TITLE).is_some()
}

fn click(h: &mut Harness<'_, TextureCraftApp>, role: Role, label: &str) {
    h.get_by_role_and_label(role, label).click();
    h.run_steps(3);
}

/// Select... answered with `file`.
fn select(h: &mut Harness<'_, TextureCraftApp>, file: &std::path::Path) {
    let file = file.to_path_buf();
    h.state_mut().file_dialogs.set_override(Some(Box::new(move |_: &Request| Answer::Paths(vec![file.clone()]))));
    click(h, Role::Button, "Select...");
}

fn reimport_ticked(h: &Harness<'_, TextureCraftApp>) -> bool {
    h.state().ui.windows.project_config.as_ref().is_some_and(|c| c.reimport)
}

#[test]
fn the_menu_opens_the_window() {
    let dir = temp_dir("menu");
    let mut h = app_on_a_quad(&dir);
    assert!(!window_is_open(&h));
    open_window(&mut h);
    assert!(window_is_open(&h));
    assert!(h.query_by_label("3D MESH").is_some(), "the 3D mesh section");
    assert!(!reimport_ticked(&h), "nothing is ticked at first");
    assert!(h.state().ui.rail_panel.is_none(), "the Display panel stays where it was");
}

#[test]
fn another_file_shows_its_path_and_ticks_reimport() {
    let dir = temp_dir("select");
    let mut h = app_on_a_quad(&dir);
    let tri = dir.join("tri.obj");
    std::fs::write(&tri, TRIANGLE).unwrap();
    open_window(&mut h);
    select(&mut h, &tri);
    assert!(h.query_by_label_contains("tri.obj").is_some(), "the chosen file is shown");
    assert!(reimport_ticked(&h));
    assert_eq!(h.state().session.mesh.triangle_count(), 2, "nothing changes before OK");
}

#[test]
fn ok_moves_the_project_to_the_chosen_mesh_and_keeps_the_layers() {
    let dir = temp_dir("ok");
    let mut h = app_on_a_quad(&dir);
    let tri = dir.join("tri.obj");
    std::fs::write(&tri, TRIANGLE).unwrap();
    let layers = h.state().session.doc.project.layers.len();
    open_window(&mut h);
    select(&mut h, &tri);
    click(&mut h, Role::Button, "OK");
    let session = &h.state().session;
    assert_eq!(session.mesh.triangle_count(), 1, "the new mesh is in use");
    assert_eq!(session.doc.project.mesh_file, "tri.obj");
    assert!(session.doc.project.mesh_path.ends_with("tri.obj"), "Reimport mesh now follows the new file");
    assert_eq!(session.doc.project.layers.len(), layers, "the layers stay");
    assert!(!window_is_open(&h));
}

#[test]
fn ticking_reimport_alone_reads_the_same_file_again() {
    let dir = temp_dir("reimport");
    let mut h = app_on_a_quad(&dir);
    let project = h.state().session.project_id;
    let layers = h.state().session.doc.project.layers.len();
    // The file changed on disk since it was imported
    std::fs::write(dir.join("quad.obj"), TRIANGLE).unwrap();
    open_window(&mut h);
    click(&mut h, Role::CheckBox, "Reimport mesh");
    assert!(reimport_ticked(&h));
    click(&mut h, Role::Button, "OK");
    let session = &h.state().session;
    assert_eq!(session.mesh.triangle_count(), 1, "the edited file was read again");
    assert_eq!(session.project_id, project);
    assert_eq!(session.doc.project.layers.len(), layers);
    assert!(!window_is_open(&h));
}

#[test]
fn ok_with_nothing_chosen_changes_nothing() {
    let dir = temp_dir("nothing");
    let mut h = app_on_a_quad(&dir);
    std::fs::write(dir.join("quad.obj"), TRIANGLE).unwrap();
    let mesh = h.state().session.mesh_id;
    open_window(&mut h);
    click(&mut h, Role::Button, "OK");
    assert_eq!(h.state().session.mesh_id, mesh);
    assert_eq!(h.state().session.mesh.triangle_count(), 2);
    assert!(!window_is_open(&h));
}

#[test]
fn cancel_and_escape_change_nothing() {
    let dir = temp_dir("cancel");
    let mut h = app_on_a_quad(&dir);
    let tri = dir.join("tri.obj");
    std::fs::write(&tri, TRIANGLE).unwrap();
    let mesh = h.state().session.mesh_id;
    open_window(&mut h);
    select(&mut h, &tri);
    click(&mut h, Role::Button, "Cancel");
    assert!(!window_is_open(&h));
    assert_eq!(h.state().session.mesh_id, mesh);
    assert_eq!(h.state().session.mesh.triangle_count(), 2);

    // Opened again, the old choice is gone, and Esc closes it too
    open_window(&mut h);
    assert!(!reimport_ticked(&h));
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(!window_is_open(&h));
    assert_eq!(h.state().session.mesh_id, mesh);
}

#[test]
fn a_failing_change_is_reported_as_a_toast() {
    let dir = temp_dir("error");
    let mut h = app_on_a_quad(&dir);
    let missing = dir.join("missing.obj");
    open_window(&mut h);
    select(&mut h, &missing);
    click(&mut h, Role::Button, "OK");
    assert!(h.state().toasts.iter().any(|t| t.error), "the error shows as a toast");
    assert_eq!(h.state().session.mesh.triangle_count(), 2, "the project keeps its mesh");
}
