//! File > Import resources...: the window that collects files and imports them together. File
//! pickers are answered by the override, never shown.

use std::path::{Path, PathBuf};

use egui::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use texturecraft_engine::Session;
use texturecraft_ui_egui::TextureCraftApp;
use texturecraft_ui_egui::file_dialogs::{Answer, Request};

/// A test folder removed when its owner drops
struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path = std::env::temp_dir().join(format!("texturecraft-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();

        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

const TITLE: &str = "Import resources";
/// The prefix field's tooltip, which is its accessible label.
const PREFIX_HINT: &str = "Put in front of the resource's name. Characters other than letters, digits and _ . - become _";

fn app() -> Harness<'static, TextureCraftApp> {
    let session = Session::new_project("Test", texturecraft_engine::geom::primitives::cube(1.0), "builtin:cube", None, 128).unwrap();
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 960.0)).build_eframe(|cc| TextureCraftApp::new(cc, session));
    h.state_mut().ui.prefs.language = "en".into();
    h.run_steps(4);

    h
}

fn png(dir: &Path, name: &str) -> PathBuf {
    let file = dir.join(name);
    image::RgbaImage::from_pixel(4, 4, image::Rgba([10, 200, 30, 255])).save(&file).unwrap();

    file
}

fn invoke(h: &mut Harness<'_, TextureCraftApp>) {
    let ctx = h.ctx.clone();
    texturecraft_ui_egui::menus::invoke(h.state_mut(), &ctx, "resource.import");
    h.run_steps(3);
}

fn is_open(h: &Harness<'_, TextureCraftApp>) -> bool {
    h.query_by_label(TITLE).is_some()
}

fn click(h: &mut Harness<'_, TextureCraftApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(3);
}

/// Add resources, answered with `files`.
fn add(h: &mut Harness<'_, TextureCraftApp>, files: &[PathBuf]) {
    let files = files.to_vec();
    h.state_mut().file_dialogs.set_override(Some(Box::new(move |_: &Request| Answer::Paths(files.clone()))));
    click(h, "Add resources");
}

/// The dropdowns showing `value` (they have no label, only a value).
fn combos<'h>(h: &'h Harness<'_, TextureCraftApp>, value: &'h str) -> impl Iterator<Item = egui_kittest::Node<'h>> {
    h.get_all_by_role(Role::ComboBox).filter(move |node| node.value().as_deref() == Some(value))
}

fn resources(h: &Harness<'_, TextureCraftApp>) -> Vec<String> {
    h.state().session.doc.resources.keys().cloned().collect()
}

/// Pick `usage` in the dropdown of the `n`th row (the dropdowns show "undefined" until changed).
fn choose_usage(h: &mut Harness<'_, TextureCraftApp>, n: usize, usage: &str) {
    let combo = combos(h, "undefined").nth(n).expect("a usage dropdown");
    combo.click();
    h.run_steps(3);
    h.get_by_label(usage).click();
    h.run_steps(3);
}

#[test]
fn the_menu_opens_the_window_without_importing_or_asking() {
    let mut h = app();
    invoke(&mut h);

    assert!(is_open(&h));
    assert!(resources(&h).is_empty());
    assert!(h.query_by_label("Import your resources and set their usage").is_some());
    assert!(combos(&h, "All imported resources (0)").next().is_some());
    assert!(h.get_by_label("Remove selected resources").accesskit_node().is_disabled(), "nothing is selected");
}

#[test]
fn added_files_are_listed_and_import_keeps_their_names() {
    let dir = TempDir::new("import-resources-add");
    let (a, b) = (png(dir.path(), "bluenoise.png"), png(dir.path(), "scratch.png"));
    let mut h = app();
    invoke(&mut h);
    add(&mut h, &[a.clone(), b]);

    assert!(combos(&h, "All imported resources (2)").next().is_some());
    assert!(h.query_by_label("bluenoise.png").is_some() && h.query_by_label("scratch.png").is_some());
    assert!(resources(&h).is_empty(), "adding only lists");

    // The same file twice is one row
    add(&mut h, &[a]);
    assert!(combos(&h, "All imported resources (2)").next().is_some());

    click(&mut h, "Import");
    assert!(!is_open(&h));
    assert_eq!(resources(&h), ["bluenoise.png", "scratch.png"]);
}

#[test]
fn remove_selected_drops_only_the_selected_rows() {
    let dir = TempDir::new("import-resources-remove");
    let mut h = app();
    invoke(&mut h);
    add(&mut h, &[png(dir.path(), "one.png"), png(dir.path(), "two.png")]);

    h.get_by_label("one.png").click();
    h.run_steps(3);
    click(&mut h, "Remove selected resources");

    assert!(h.query_by_label("one.png").is_none());
    assert!(h.query_by_label("two.png").is_some());
    assert!(combos(&h, "All imported resources (1)").next().is_some());
    click(&mut h, "Import");
    assert_eq!(resources(&h), ["two.png"]);
}

#[test]
fn the_usage_defaults_to_undefined_and_the_choice_reaches_the_import() {
    let dir = TempDir::new("import-resources-usage");
    let mut h = app();
    invoke(&mut h);
    add(&mut h, &[png(dir.path(), "plain.png"), png(dir.path(), "brush.png"), png(dir.path(), "sky.png")]);
    assert_eq!(combos(&h, "undefined").count(), 3);

    choose_usage(&mut h, 1, "alpha");
    choose_usage(&mut h, 1, "environment");
    click(&mut h, "Import");

    assert_eq!(resources(&h), ["brush.png", "plain.png", "sky.png"]);
    let project = &h.state().session.doc.project;
    assert_eq!(project.display.environment_resource(), Some("sky.png"), "the environment usage lights the scene");
    assert_eq!(h.state().session.tool.brush.alpha_image.as_deref(), Some("brush.png"), "the alpha usage sets the brush alpha");
}

#[test]
fn a_prefix_goes_in_front_of_the_resource_name() {
    let dir = TempDir::new("import-resources-prefix");
    let mut h = app();
    invoke(&mut h);
    add(&mut h, &[png(dir.path(), "dust.png")]);

    h.get_by_role_and_label(Role::TextInput, PREFIX_HINT).click();
    h.run_steps(2);
    h.get_by_role_and_label(Role::TextInput, PREFIX_HINT).type_text("wear/");
    h.run_steps(3);
    click(&mut h, "Import");

    assert_eq!(resources(&h), ["wear_dust.png"]);
}

#[test]
fn unsupported_usages_and_the_user_library_are_listed_but_disabled() {
    let dir = TempDir::new("import-resources-disabled");
    let mut h = app();
    invoke(&mut h);
    add(&mut h, &[png(dir.path(), "x.png")]);

    combos(&h, "undefined").next().expect("the dropdown").click();
    h.run_steps(3);
    for usage in ["colorlut", "postfx"] {
        assert!(h.get_by_label(usage).accesskit_node().is_disabled(), "{usage}");
    }
    assert!(!h.get_by_label("texture").accesskit_node().is_disabled());
    h.get_by_label("texture").click();
    h.run_steps(3);

    combos(&h, "current session").next().expect("the dropdown").click();
    h.run_steps(3);
    assert!(h.get_by_label("library 'your_assets'").accesskit_node().is_disabled());
    assert!(h.query_by_label("To add a new library, see the Edit / Settings menu").is_some(), "the note at the bottom left");
}

#[test]
fn cancel_imports_nothing() {
    let dir = TempDir::new("import-resources-cancel");
    let mut h = app();
    invoke(&mut h);
    add(&mut h, &[png(dir.path(), "never.png")]);
    click(&mut h, "Cancel");

    assert!(!is_open(&h));
    assert!(resources(&h).is_empty());

    // Reopening starts empty
    invoke(&mut h);
    assert!(combos(&h, "All imported resources (0)").next().is_some());
}

#[test]
fn the_engine_command_still_imports_a_path_with_an_optional_prefix() {
    let dir = TempDir::new("import-resources-command");
    let file = png(dir.path(), "tile.png");
    let mut session = Session::new_project("Test", texturecraft_engine::geom::primitives::cube(1.0), "builtin:cube", None, 128).unwrap();

    let plain = texturecraft_engine::execute(&mut session, "resource.import", &serde_json::json!({"path": file.display().to_string()})).unwrap();
    let prefixed =
        texturecraft_engine::execute(&mut session, "resource.import", &serde_json::json!({"path": file.display().to_string(), "prefix": "a/b/"})).unwrap();

    assert_eq!(plain["name"], "tile.png");
    assert_eq!(prefixed["name"], "a_b_tile.png");
}
