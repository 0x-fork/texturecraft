//! Edit > Project Configuration...: the 3D mesh section. File pickers are answered by the
//! override, never shown.

use std::path::PathBuf;

use egui::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use serde_json::json;
use texturecraft_engine::Session;
use texturecraft_ui_egui::TextureCraftApp;
use texturecraft_ui_egui::file_dialogs::{Answer, Request};

const QUAD: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf 1/1 2/2 3/3 4/4\n";
const TRIANGLE: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nf 1/1 2/2 3/3\n";
const TITLE: &str = "Project configuration";

/// A new folder for one test, removed when the test ends (also when it fails).
struct TempDir(PathBuf);

impl std::ops::Deref for TempDir {
    type Target = std::path::Path;
    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn temp_dir(test: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!("texturecraft-project-config-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
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

/// The Reimport mesh box as the window shows it.
fn reimport_ticked(h: &Harness<'_, TextureCraftApp>) -> bool {
    h.get_by_role_and_label(Role::CheckBox, "Reimport mesh").accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

/// The tick the user made themselves, as stored.
fn own_tick(h: &Harness<'_, TextureCraftApp>) -> bool {
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
    assert!(!own_tick(&h), "shown ticked, not stored as the user's tick");
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
    assert!(reimport_ticked(&h) && own_tick(&h));
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
fn cancel_changes_nothing() {
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

    // Opened again, the old choice is gone
    open_window(&mut h);
    assert!(!reimport_ticked(&h));
    assert!(h.state().ui.windows.project_config.as_ref().is_some_and(|c| c.mesh.is_none()));
}

#[test]
fn escape_closes_the_window_and_leaves_a_stroke_alone() {
    let dir = temp_dir("escape");
    let mut h = app_on_a_quad(&dir);
    assert_eq!(h.state().ui.mode, texturecraft_ui_egui::state::Mode::Painting, "Painting is where Escape cancels strokes");
    texturecraft_engine::execute(&mut h.state_mut().session, "layer.add.paint", &json!({})).unwrap();
    h.state_mut().session.begin_stroke_uv(texturecraft_engine::geom::glam::Vec2::splat(0.5), 1.0).unwrap();
    open_window(&mut h);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(!window_is_open(&h));
    assert!(h.state().session.stroke_active(), "Escape went to the window, not to the stroke");
}

#[test]
fn enter_confirms_but_not_a_text_fields_enter() {
    let dir = temp_dir("enter");
    let mut h = app_on_a_quad(&dir);
    std::fs::write(dir.join("quad.obj"), TRIANGLE).unwrap();
    open_window(&mut h);
    // Ticked without a click, which would leave the box focused
    h.state_mut().ui.windows.project_config.as_mut().unwrap().reimport = true;
    // Renaming a layer: Enter belongs to the text field
    let layer = h.state().session.selected_layer().unwrap().0;
    let name = h.state().session.doc.project.layers.values().find(|l| l.id.0 == layer).unwrap().name.clone();
    h.state_mut().ui.rename = Some((layer, name));
    h.run_steps(3);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(h.state().ui.rename.is_none(), "Enter ended the rename");
    assert!(window_is_open(&h), "and did not press OK");
    assert_eq!(h.state().session.mesh.triangle_count(), 2);

    // With nothing focused, Enter is OK
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(!window_is_open(&h));
    assert_eq!(h.state().session.mesh.triangle_count(), 1, "the tick was applied");
}

#[test]
fn picking_the_projects_own_file_again_drops_the_choice_and_its_tick() {
    let dir = temp_dir("own");
    let mut h = app_on_a_quad(&dir);
    let tri = dir.join("tri.obj");
    std::fs::write(&tri, TRIANGLE).unwrap();
    std::fs::write(dir.join("quad.obj"), TRIANGLE).unwrap();
    let mesh = h.state().session.mesh_id;
    open_window(&mut h);
    select(&mut h, &tri);
    let own = h.state().session.doc.project.mesh_path.clone();
    select(&mut h, std::path::Path::new(&own));
    assert!(!reimport_ticked(&h), "the tick was only ever the file's");
    click(&mut h, Role::Button, "OK");
    assert_eq!(h.state().session.mesh_id, mesh, "OK does nothing");
    assert_eq!(h.state().session.mesh.triangle_count(), 2);
}

#[test]
fn a_missing_file_cannot_be_reimported() {
    let dir = temp_dir("missing");
    let mut h = app_on_a_quad(&dir);
    std::fs::remove_file(dir.join("quad.obj")).unwrap();
    open_window(&mut h);
    assert!(h.query_by_label("File not found").is_some());
    assert!(h.get_by_role_and_label(Role::CheckBox, "Reimport mesh").accesskit_node().is_disabled());
    click(&mut h, Role::Button, "OK");
    assert_eq!(h.state().file_dialogs.pending(), 0, "no second dialog");
    assert!(!window_is_open(&h));
    assert!(h.state().toasts.is_empty());
}

#[test]
fn a_built_in_mesh_shows_its_name_and_has_nothing_to_reimport() {
    let dir = temp_dir("builtin");
    let mut h = app_on_a_quad(&dir);
    h.state_mut().session.doc.project.mesh_path.clear();
    open_window(&mut h);
    assert!(h.query_by_label("quad.obj").is_some(), "the name it was imported under");
    assert!(h.query_by_label("File not found").is_none());
    assert!(h.get_by_role_and_label(Role::CheckBox, "Reimport mesh").accesskit_node().is_disabled());
    h.state_mut().session.doc.project.mesh_file.clear();
    h.run_steps(2);
    assert!(h.query_by_label("Built-in mesh").is_some());
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
    assert!(window_is_open(&h), "the window stays open");
    assert_eq!(h.state().ui.windows.project_config.as_ref().and_then(|c| c.mesh.clone()), Some(missing.display().to_string()), "with the choice kept");
    assert_eq!(h.state().session.mesh.triangle_count(), 2, "the project keeps its mesh");
}

fn project_normal_format(h: &Harness<'_, TextureCraftApp>) -> texturecraft_engine::core::NormalFormat {
    h.state().session.doc.project.normal_format
}

#[test]
fn the_normal_map_format_changes_on_ok_and_shows_in_the_texture_set_panel() {
    use texturecraft_engine::core::NormalFormat;
    let dir = temp_dir("normal-format");
    let mut h = app_on_a_quad(&dir);
    open_window(&mut h);
    click(&mut h, Role::ComboBox, "OpenGL");
    click(&mut h, Role::Button, "DirectX");
    assert_eq!(project_normal_format(&h), NormalFormat::OpenGl, "nothing changes before OK");
    click(&mut h, Role::Button, "OK");
    assert_eq!(project_normal_format(&h), NormalFormat::DirectX);
    h.state_mut().ui.dock.focus(texturecraft_ui_egui::docking::Panel::TextureSetSettings);
    h.run_steps(3);
    assert!(h.query_by_label("DirectX").is_some(), "the Texture Set Settings panel shows the project's format");
    // One undo step puts it back
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(project_normal_format(&h), NormalFormat::OpenGl);
}

#[test]
fn cancel_discards_a_normal_map_format() {
    use texturecraft_engine::core::NormalFormat;
    let dir = temp_dir("normal-format-cancel");
    let mut h = app_on_a_quad(&dir);
    open_window(&mut h);
    click(&mut h, Role::ComboBox, "OpenGL");
    click(&mut h, Role::Button, "DirectX");
    click(&mut h, Role::Button, "Cancel");
    assert_eq!(project_normal_format(&h), NormalFormat::OpenGl);
}

#[test]
fn uv_tiles_are_shown_read_only() {
    let dir = temp_dir("udim");
    let mut h = app_on_a_quad(&dir);
    open_window(&mut h);
    assert!(h.query_by_label("UV Tile settings (UDIMs)").is_some());
    assert!(h.query_by_label("Off").is_some(), "a project without UV tiles");
    assert!(h.query_by_label("You can't change UV Tile settings once a project has been created.").is_some());
    click(&mut h, Role::Button, "Cancel");

    texturecraft_engine::execute(&mut h.state_mut().session, "textureSet.uvTiles", &json!({"ts": 0, "enabled": true})).unwrap();
    open_window(&mut h);
    assert!(h.query_by_label("Off").is_none());
    assert!(h.query_by_label("1001").is_some(), "the tile list of a UDIM project");
}

#[test]
fn preserving_strokes_is_an_import_setting_of_a_mesh_that_is_read() {
    let dir = temp_dir("preserve");
    let mut h = app_on_a_quad(&dir);
    let tri = dir.join("tri.obj");
    std::fs::write(&tri, TRIANGLE).unwrap();
    let box_of = |h: &Harness<'_, TextureCraftApp>| h.get_by_role_and_label(Role::CheckBox, "Preserve stroke positions on mesh").accesskit_node().is_disabled();
    open_window(&mut h);
    assert!(box_of(&h), "greyed while no mesh is going to be read");
    assert!(h.state().ui.windows.project_config.as_ref().unwrap().preserve, "on by default");
    select(&mut h, &tri);
    assert!(!box_of(&h));
    click(&mut h, Role::CheckBox, "Preserve stroke positions on mesh");
    assert!(!h.state().ui.windows.project_config.as_ref().unwrap().preserve);
    click(&mut h, Role::Button, "OK");
    assert_eq!(h.state().session.mesh.triangle_count(), 1, "the mesh changed without reprojecting strokes");
}
