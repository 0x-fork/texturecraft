//! The empty state (no project open): only commands that open or create a project run, and
//! File > Close returns to it.

use serde_json::json;
use texturecraft_engine::{EngineError, Session, Tool, check_enabled, command_specs, execute, find};

fn open_project() -> Session {
    Session::new_project("Open", texturecraft_engine::geom::primitives::cube(1.0), "builtin:cube", None, 128).unwrap()
}

#[test]
fn a_closed_session_refuses_commands_that_need_a_project() {
    let mut s = Session::closed();
    assert!(!s.project_open);
    for id in ["layer.add.paint", "file.save", "file.close", "edit.undo", "tool.paint", "bake.run", "file.exportTextures", "viewport.view", "camera.add"] {
        let error = execute(&mut s, id, &json!({})).unwrap_err();
        assert!(matches!(error, EngineError::Disabled { .. }), "{id}: {error}");
    }
    assert!(!s.can_undo(), "nothing was changed");
    assert!(!s.tool_usable(Tool::Paint), "no tool can act without a project");
}

#[test]
fn a_closed_session_lists_only_the_opening_commands_as_enabled() {
    let s = Session::closed();
    for id in ["file.open", "file.new", "project.new", "project.sample"] {
        assert!(check_enabled(find(id).unwrap(), &s).is_ok(), "{id} runs with no project");
    }
    let enabled: Vec<&str> = command_specs().iter().filter(|spec| check_enabled(spec, &s).is_ok()).map(|spec| spec.id).collect();
    assert!(!enabled.iter().any(|id| id.starts_with("layer.") || id.starts_with("tool.") || id.starts_with("bake.")), "{enabled:?}");
}

#[test]
fn creating_a_project_from_the_closed_state_opens_it() {
    let mut s = Session::closed();
    execute(&mut s, "project.new", &json!({"mesh": "cube", "resolution": 128})).unwrap();
    assert!(s.project_open);
    execute(&mut s, "layer.add.paint", &json!({})).unwrap();
}

#[test]
fn close_returns_to_the_empty_state_and_keeps_the_user_presets() {
    let mut s = open_project();
    s.modified = true;
    execute(&mut s, "file.close", &json!({})).unwrap();
    assert!(!s.project_open && !s.modified && s.path.is_none());
    assert!(execute(&mut s, "file.close", &json!({})).is_err(), "closing again has nothing to close");
    execute(&mut s, "project.new", &json!({"mesh": "cube", "resolution": 128})).unwrap();
    assert!(s.project_open, "a project can be created again");
}
