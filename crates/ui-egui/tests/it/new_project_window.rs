//! File > New project: the sections and their order, the defaults, settings shown disabled with a
//! hint until TextureCraft has them, Create waiting for a mesh file, and the settings that reach
//! the `file.new` command.

use egui::Role;
use egui::accesskit::Toggled;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use texturecraft_engine::template_cmds::DEFAULT_TEMPLATE;
use texturecraft_ui_egui::TextureCraftApp;
use texturecraft_ui_egui::new_project_advanced::create_command;
use texturecraft_ui_egui::project_form::NOT_AVAILABLE;

/// A quad whose UVs lie in UV tile 1002 and a camera-free OBJ
const TILE_1002_QUAD: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 1.1 0.1\nvt 1.9 0.1\nvt 1.9 0.9\nvt 1.1 0.9\nf 1/1 2/2 3/3 4/4\n";

/// The app with New project opened from the menu, tall enough to show the whole form
fn harness() -> Harness<'static, TextureCraftApp> {
    harness_sized(egui::vec2(1600.0, 2400.0))
}

fn harness_sized(size: egui::Vec2) -> Harness<'static, TextureCraftApp> {
    let session = texturecraft_engine::Session::new_project("Open", texturecraft_engine::geom::primitives::cube(1.0), "builtin:cube", None, 128).unwrap();
    let mut h = Harness::builder().with_size(size).build_eframe(|cc| TextureCraftApp::new(cc, session));
    h.state_mut().ui.prefs.language = "en".into();
    h.run_steps(2);
    let ctx = h.ctx.clone();
    texturecraft_ui_egui::menus::invoke(h.state_mut(), &ctx, "ui.newProject");
    h.run_steps(4);
    h
}

fn mesh_file(test: &str, contents: &str) -> String {
    let dir = std::env::temp_dir().join(format!("texturecraft-np-window-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tiles.obj");
    std::fs::write(&path, contents).unwrap();
    path.display().to_string()
}

fn top(h: &Harness<'_, TextureCraftApp>, label: &str) -> f32 {
    h.get_all_by_label(label).map(|node| node.rect().top()).fold(f32::INFINITY, f32::min)
}

fn disabled(h: &Harness<'_, TextureCraftApp>, role: Role, label: &str) -> bool {
    h.get_by_role_and_label(role, label).accesskit_node().is_disabled()
}

fn ticked(h: &Harness<'_, TextureCraftApp>, role: Role, label: &str) -> bool {
    h.get_by_role_and_label(role, label).accesskit_node().toggled() == Some(Toggled::True)
}

#[test]
fn the_sections_follow_the_reference_order() {
    let h = harness();
    let order = [
        "BASIC SETTINGS",
        "3D mesh",
        "Project template",
        "Document resolution (px)",
        "ADVANCED SETTINGS",
        "Normal map format",
        "Compute tangent space per fragment",
        "UV Tile settings (UDIMs)",
        "You can change UV tiles later in Texture Set Settings.",
        "Use UV Tile workflow",
        "Preserve UV Tile layout per material and enable painting across tiles",
        "Convert UV Tiles into individual Texture Sets (legacy)",
        "Import settings",
        "Import cameras",
        "Auto-unwrap",
        "Import baked maps",
        "Import mesh normal maps and baked maps for all materials.",
        "Automatic normal maps assignation scheme: $(MATERIAL_NAME)_normal",
        "Physical size",
        "Use mesh file's internal unit scale (recommended)",
        "Custom unit scale",
        "Switch fill layer scaling to Physical size when assigning materials",
        "Color management",
        "Working color space",
    ];
    let tops: Vec<f32> = order.iter().map(|label| top(&h, label)).collect();
    for (pair, labels) in tops.windows(2).zip(order.windows(2)) {
        assert!(pair[0] < pair[1], "{} comes before {}: {pair:?}", labels[0], labels[1]);
    }
    assert!(h.query_by_label("Linear - sRGB Chrominance (ITU-R BT.709)").is_some());
}

#[test]
fn the_defaults_match_the_reference() {
    let h = harness();

    assert!(h.query_by_label("No file selected").is_some());
    assert!(h.query_by_value(DEFAULT_TEMPLATE).is_some(), "the default template");
    assert!(h.query_by_value("1024 x 1024").is_some());
    assert!(h.query_by_value("OpenGL").is_some());
    assert!(h.query_by_value("Legacy").is_some());
    assert!(!ticked(&h, Role::CheckBox, "Compute tangent space per fragment"));
    assert!(!ticked(&h, Role::CheckBox, "Use UV Tile workflow"));
    assert!(ticked(&h, Role::RadioButton, "Preserve UV Tile layout per material and enable painting across tiles"));
    assert!(disabled(&h, Role::RadioButton, "Preserve UV Tile layout per material and enable painting across tiles"), "greyed while UV tiles are off");
    assert!(ticked(&h, Role::CheckBox, "Import cameras") && !disabled(&h, Role::CheckBox, "Import cameras"));
    assert!(ticked(&h, Role::CheckBox, "Auto-unwrap"));
    assert!(ticked(&h, Role::RadioButton, "Use mesh file's internal unit scale (recommended)"));
    assert!(!ticked(&h, Role::CheckBox, "Switch fill layer scaling to Physical size when assigning materials"));
}

#[test]
fn template_controls_name_and_built_in_meshes_are_gone() {
    let h = harness();

    for label in ["Template name", "Save current project as template", "Name", "UV tiles (UDIM)"] {
        assert!(h.query_by_label(label).is_none(), "{label} is not in the window");
    }
    assert!(h.query_by_label_contains("Built-in:").is_none(), "no built-in mesh dropdown");
}

#[test]
fn create_waits_for_a_mesh_file() {
    let mut h = harness();

    assert!(disabled(&h, Role::Button, "Create"));

    h.state_mut().ui.new_project.file = Some(mesh_file("create-enabled", TILE_1002_QUAD));
    h.run_steps(2);

    assert!(!disabled(&h, Role::Button, "Create"));
    assert!(h.query_by_label("tiles.obj").is_some(), "the chosen file is shown");
}

#[test]
fn settings_not_available_yet_are_disabled_with_a_hint() {
    let mut h = harness();
    let unavailable = [
        (Role::CheckBox, "Compute tangent space per fragment"),
        (Role::RadioButton, "Convert UV Tiles into individual Texture Sets (legacy)"),
        (Role::CheckBox, "Auto-unwrap"),
        (Role::Button, "Auto-unwrap options"),
        (Role::RadioButton, "Custom unit scale"),
        (Role::CheckBox, "Switch fill layer scaling to Physical size when assigning materials"),
    ];
    for (role, label) in unavailable {
        assert!(disabled(&h, role, label), "{label} is disabled");
    }
    assert!(!disabled(&h, Role::RadioButton, "Use mesh file's internal unit scale (recommended)"));

    for (role, label) in unavailable {
        let center = h.get_by_role_and_label(role, label).rect().center();
        h.hover_at(egui::pos2(1.0, 1.0));
        h.run_steps(2);
        assert!(h.query_by_label(NOT_AVAILABLE).is_none(), "no hint before hovering {label}");
        h.hover_at(center);
        h.run_steps(30);
        assert!(h.query_by_label(NOT_AVAILABLE).is_some(), "hovering {label} says it is not available yet");
    }
}

#[test]
fn opencolorio_shows_its_settings_disabled_and_creates_legacy() {
    let mut h = harness();

    assert!(h.query_by_label("OpenColorIO configuration").is_none(), "Legacy shows only the working color space");

    h.get_by_value("Legacy").scroll_to_me();
    h.run_steps(3);
    h.get_by_value("Legacy").click();
    h.run_steps(3);
    h.get_by_label("OpenColorIO").click();
    h.run_steps(3);

    assert!(h.query_by_label("OpenColorIO is not available yet. The project uses Legacy.").is_some());
    for label in ["OpenColorIO configuration", "Configuration file", "Standard sRGB color space", "Auto detect color spaces", "Material color space default"] {
        assert!(h.query_by_label(label).is_some(), "{label} is shown");
    }
    assert!(h.query_by_label("ACEScg").is_some(), "the OpenColorIO working color space");
    let ocio_config = h.get_by_value("ACES 2.0");
    assert!(ocio_config.accesskit_node().is_disabled(), "the OpenColorIO settings are disabled");
    let center = ocio_config.rect().center();
    h.hover_at(center);
    h.run_steps(30);
    assert!(h.query_by_label(NOT_AVAILABLE).is_some(), "with the not-available hint");
    let params = create_command(&texturecraft_ui_egui::state::NewProject { file: Some("a.obj".into()), ..h.state().ui.new_project.clone() }).unwrap();
    assert!(params.get("colorManagement").is_none(), "nothing about color management reaches the command");
}

#[test]
fn the_menu_opens_a_fresh_form() {
    let mut h = harness();
    {
        let form = &mut h.state_mut().ui.new_project;
        form.file = Some("/old/mesh.obj".into());
        form.resolution = 4096;
        form.uv_tiles = Some(true);
        form.import_cameras = false;
        form.baked_maps = vec!["/maps/Ball_normal.png".into()];
    }
    h.state_mut().ui.windows.new_project = false;
    h.run_steps(2);
    let ctx = h.ctx.clone();
    texturecraft_ui_egui::menus::invoke(h.state_mut(), &ctx, "ui.newProject");
    h.run_steps(3);

    let form = &h.state().ui.new_project;
    assert_eq!((form.file.as_deref(), form.resolution, form.uv_tiles, form.import_cameras), (None, 1024, None, true));
    assert_eq!(form.template, DEFAULT_TEMPLATE);
    assert_eq!(form.baked_maps, ["/maps/Ball_normal.png"], "files kept after a failed import stay listed");
}

#[test]
fn the_supported_settings_reach_the_create_command() {
    let mut form = texturecraft_ui_egui::state::NewProject::default();

    assert!(create_command(&form).is_none(), "no command without a mesh file");

    form.file = Some("/meshes/robot.fbx".into());
    let params = create_command(&form).unwrap();

    assert_eq!(params["mesh"], "/meshes/robot.fbx");
    assert_eq!(params["resolution"], 1024);
    assert_eq!(params["uvTiles"], false, "UV tiles are off unless ticked");
    assert_eq!(params["importCameras"], true);
    assert_eq!(params["template"], DEFAULT_TEMPLATE);
    assert!(params.get("name").is_none(), "the project is named after the file");

    form.uv_tiles = Some(true);
    form.import_cameras = false;
    form.resolution = 2048;
    let params = create_command(&form).unwrap();

    assert_eq!((params["uvTiles"].as_bool(), params["importCameras"].as_bool(), params["resolution"].as_u64()), (Some(true), Some(false), Some(2048)));
}

#[test]
fn ticking_uv_tiles_creates_a_uv_tile_project() {
    for (tick, expected) in [(false, Vec::<u32>::new()), (true, vec![1002])] {
        let mut h = harness();
        h.state_mut().ui.new_project.file = Some(mesh_file(&format!("udim-{tick}"), TILE_1002_QUAD));
        h.state_mut().ui.new_project.resolution = 128;
        h.run_steps(2);
        if tick {
            h.get_by_role_and_label(Role::CheckBox, "Use UV Tile workflow").click();
            h.run_steps(3);
            assert!(!disabled(&h, Role::RadioButton, "Preserve UV Tile layout per material and enable painting across tiles"));
        }
        h.get_by_role_and_label(Role::CheckBox, "Import cameras").click();
        h.run_steps(2);
        h.get_by_role_and_label(Role::Button, "Create").click();
        h.run_steps(3);

        let app = h.state();
        assert!(!app.ui.windows.new_project, "the window closes");
        assert_eq!(app.session.doc.project.name, "tiles");
        assert_eq!(app.session.doc.project.texture_sets[0].uv_tiles, expected, "UV tiles ticked: {tick}");
        assert_eq!(app.session.doc.project.texture_sets[0].resolution, 128);
    }
}

#[test]
fn create_stays_on_screen_in_a_short_window() {
    let h = harness_sized(egui::vec2(1024.0, 600.0));

    let create = h.get_by_role_and_label(Role::Button, "Create").rect();
    let cancel = h.get_by_role_and_label(Role::Button, "Cancel").rect();

    assert!(create.bottom() <= 600.0 && create.top() >= 0.0, "Create is visible without scrolling: {create:?}");
    assert!(cancel.bottom() <= 600.0, "{cancel:?}");
}

#[test]
fn picking_a_template_sets_its_resolution_and_normal_map_format() {
    let mut h = harness();
    h.state_mut().session.templates.dir = None;
    h.state_mut().run("textureSet.settings", serde_json::json!({"normalFormat": "directX"})).unwrap();
    h.state_mut().run("project.template.save", serde_json::json!({"name": "Small DirectX"})).unwrap();
    h.run_steps(2);

    h.get_by_value(DEFAULT_TEMPLATE).click();
    h.run_steps(3);
    h.get_by_label("Small DirectX").click();
    h.run_steps(3);

    assert!(h.query_by_value("Small DirectX").is_some());
    assert!(h.query_by_value("128 x 128").is_some(), "the template's resolution (the open project's 128 px)");
    assert!(h.query_by_value("DirectX").is_some(), "the template's normal map format");
    assert_eq!(h.state().ui.new_project.resolution, 128);
}
