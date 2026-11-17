//! The shelf's side panes left of the grid, toggled from the toolbar: saved searches and the folder tree

use egui::{RichText, Ui};
use egui_phosphor::regular as ic;
use serde::{Deserialize, Serialize};

use crate::TextureCraftApp;
use crate::i18n::{n_, tr};
use crate::saved_search::SavedSearch;
use crate::shelf::TABS;
use crate::theme::Tokens;
use crate::widgets::icon_button;

/// The panes' column width as a fraction of the panel, and its least width in points
const COLUMN_FRACTION: f32 = 0.45;
const COLUMN_MIN_WIDTH: f32 = 110.0;
/// The splitter's height between the two panes in points
const SPLITTER_HEIGHT: f32 = 6.0;
/// The least height either pane keeps while the splitter moves, in points
const PANE_MIN_HEIGHT: f32 = 40.0;
/// The hint on rows whose function TextureCraft lacks
pub(crate) const NOT_AVAILABLE: &str = n_("Not available yet");
/// The reference's built-in saved searches after "All", shown greyed until they work
const BUILT_IN_SEARCHES: [&str; 3] = [n_("Favorites"), n_("3D Assets"), n_("Flattened layers")];
/// The reference's font libraries after the asset libraries, shown greyed until they work
const FONT_LIBRARIES: [&str; 2] = [n_("System fonts"), n_("User fonts")];
/// The pane header's caption size in points
const HEADER_SIZE: f32 = 10.0;

/// Which side panes are open
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShelfPanes {
    pub saved_searches: bool,
    pub filter_by_path: bool,
    /// Filter by path lists only the folders with assets matching the search and tab
    pub hide_non_applicable: bool,
}

/// The toolbar's buttons that open and close the panes
pub(crate) fn toggles(app: &mut TextureCraftApp, ui: &mut Ui, size: f32) {
    let panes = &mut app.ui.shelf_panes;
    if icon_button(ui, ic::BOOKMARKS_SIMPLE, tr("Saved searches"), panes.saved_searches, size).clicked() {
        panes.saved_searches = !panes.saved_searches;
    }
    if icon_button(ui, ic::TREE_STRUCTURE, tr("Filter by path"), panes.filter_by_path, size).clicked() {
        panes.filter_by_path = !panes.filter_by_path;
    }
}

/// The width of the panes' column in a panel `width` wide, or None while no pane is open
pub(crate) fn column_width(app: &TextureCraftApp, width: f32) -> Option<f32> {
    let panes = &app.ui.shelf_panes;
    (panes.saved_searches || panes.filter_by_path).then(|| (width * COLUMN_FRACTION).max(COLUMN_MIN_WIDTH).min(width))
}

/// The open panes, top to bottom, with a splitter between them; the tree shows `paths`, or `applicable` while it hides folders with nothing matching
pub(crate) fn column(app: &mut TextureCraftApp, ui: &mut Ui, paths: &[Vec<String>], applicable: &[Vec<String>]) {
    // Names past the narrow column end in "…" rather than breaking mid-word
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
    let shown = if app.ui.shelf_panes.hide_non_applicable { applicable } else { paths };
    let panes = &app.ui.shelf_panes;
    if !(panes.saved_searches && panes.filter_by_path) {
        // One pane open takes the whole column
        if panes.saved_searches {
            saved_searches(app, ui);
        } else if panes.filter_by_path {
            filter_by_path(app, ui, shown);
        }
        return;
    }

    let rect = ui.available_rect_before_wrap();
    let splitter = splitter(app, ui, rect);
    let top = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), splitter.top()));
    let bottom = egui::Rect::from_min_max(egui::pos2(rect.left(), splitter.bottom()), rect.max);
    ui.scope_builder(egui::UiBuilder::new().max_rect(top), |ui| {
        ui.set_clip_rect(top.intersect(ui.clip_rect()));
        saved_searches(app, ui);
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(bottom), |ui| {
        ui.set_clip_rect(bottom.intersect(ui.clip_rect()));
        filter_by_path(app, ui, shown);
    });
    ui.allocate_rect(rect, egui::Sense::hover());
}

/// The splitter between the panes in `rect`: a drag moves it and keeps both panes at least [`PANE_MIN_HEIGHT`] high
fn splitter(app: &mut TextureCraftApp, ui: &mut Ui, rect: egui::Rect) -> egui::Rect {
    let usable = (rect.height() - SPLITTER_HEIGHT).max(0.0);
    let clamp = |share: f32| {
        let least = (PANE_MIN_HEIGHT / usable.max(1.0)).min(0.5);
        share.clamp(least, 1.0 - least)
    };
    let top_of = |share: f32| rect.top() + usable * clamp(share);
    let handle = egui::Rect::from_min_size(egui::pos2(rect.left(), top_of(app.ui.shelf_panes_split)), egui::vec2(rect.width(), SPLITTER_HEIGHT));

    let response = ui.interact(handle, ui.scope_id().with("shelf_panes_splitter"), egui::Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeVertical);
    if response.dragged()
        && let Some(pointer) = response.interact_pointer_pos()
        && usable > 0.0
    {
        app.ui.shelf_panes_split = clamp((pointer.y - SPLITTER_HEIGHT / 2.0 - rect.top()) / usable);
    }

    let handle = handle.translate(egui::vec2(0.0, top_of(app.ui.shelf_panes_split) - handle.top()));
    let visuals = ui.visuals();
    let stroke = if response.hovered() || response.dragged() { visuals.widgets.hovered.fg_stroke } else { visuals.widgets.noninteractive.bg_stroke };
    ui.painter().hline(handle.x_range(), handle.center().y, stroke);
    handle
}

/// "All libraries", then each library's folders as a tree; a click shows that folder
fn filter_by_path(app: &mut TextureCraftApp, ui: &mut Ui, paths: &[Vec<String>]) {
    let size = ui.spacing().interact_size.y;
    header(ui, tr("Filter by path"), |ui| {
        let hiding = app.ui.shelf_panes.hide_non_applicable;
        if icon_button(ui, ic::EYE_SLASH, tr("Hide non-applicable folders"), hiding, size).clicked() {
            app.ui.shelf_panes.hide_non_applicable = !hiding;
        }
    });

    let mut picked = None;
    egui::ScrollArea::vertical().id_salt("shelf_path_tree").auto_shrink([false, true]).show(ui, |ui| {
        if ui.selectable_label(app.ui.shelf_path.is_empty(), tr("All libraries")).clicked() {
            picked = Some(Vec::new());
        }
        folders(ui, paths, &[], &app.ui.shelf_path, &mut picked);
        // Nothing is ever in them yet, so hiding non-applicable folders hides them too
        if !app.ui.shelf_panes.hide_non_applicable {
            for library in FONT_LIBRARIES {
                ui.add_enabled(false, egui::Button::selectable(false, tr(library))).on_disabled_hover_text(tr(NOT_AVAILABLE));
            }
        }
    });
    if let Some(path) = picked {
        app.ui.shelf_path = path;
    }
}

/// The folders below `prefix`: one with folders inside opens and closes, a click on a name picks it
fn folders(ui: &mut Ui, paths: &[Vec<String>], prefix: &[String], current: &[String], picked: &mut Option<Vec<String>>) {
    for child in crate::shelf_folders::children(paths, prefix) {
        let path: Vec<String> = prefix.iter().cloned().chain([child.clone()]).collect();
        let name = crate::shelf_folders::name(prefix.len(), &child);
        let selected = current == path.as_slice();
        if crate::shelf_folders::children(paths, &path).is_empty() {
            ui.indent(("shelf_path_leaf", &path), |ui| {
                if ui.selectable_label(selected, name).clicked() {
                    *picked = Some(path.clone());
                }
            });
            continue;
        }

        let id = ui.make_persistent_id(("shelf_path", &path));
        let open_by_default = current.starts_with(&path);
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open_by_default)
            .show_header(ui, |ui| {
                if ui.selectable_label(selected, name).clicked() {
                    *picked = Some(path.clone());
                }
            })
            .body(|ui| folders(ui, paths, &path, current, picked));
    }
}

/// A pane's caption with a button at its right
fn header(ui: &mut Ui, title: &str, button: impl FnOnce(&mut Ui)) {
    let tokens = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(title.to_uppercase()).size(HEADER_SIZE).color(tokens.text_dim));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), button);
    });
}

/// "All", then the saved queries; the header's button saves the current one, a right-click deletes one
fn saved_searches(app: &mut TextureCraftApp, ui: &mut Ui) {
    let size = ui.spacing().interact_size.y;
    let current = current_query(app);
    header(ui, tr("Saved searches"), |ui| {
        if icon_button(ui, ic::FLOPPY_DISK, tr("Save the current search"), false, size).clicked() && !app.ui.shelf_saved_searches.contains(&current) {
            app.ui.shelf_saved_searches.push(current.clone());
        }
    });

    let everything = current.filter.is_empty() && current.path.is_empty() && current.tab == crate::shelf_tabs::NO_TYPE;
    // "All" and the built-in searches come first, in the list so a short pane scrolls them too
    let fixed_rows = 1 + BUILT_IN_SEARCHES.len();
    let mut deleted = None;
    let row_height = ui.spacing().interact_size.y;
    let total_rows = fixed_rows + app.ui.shelf_saved_searches.len();
    egui::ScrollArea::vertical().id_salt("shelf_saved_searches").auto_shrink([false, true]).show_rows(ui, row_height, total_rows, |ui, visible_rows| {
        for row in visible_rows {
            if row == 0 {
                if ui.selectable_label(everything, tr("All")).clicked() {
                    crate::shelf_toolbar::reset_query(app);
                }
                continue;
            }
            if let Some(name) = BUILT_IN_SEARCHES.get(row - 1) {
                ui.add_enabled(false, egui::Button::selectable(false, tr(name))).on_disabled_hover_text(tr(NOT_AVAILABLE));
                continue;
            }

            let i = row - fixed_rows;
            let Some(saved) = app.ui.shelf_saved_searches.get(i) else { continue };

            let same = saved.filter == current.filter && saved.tab == current.tab && saved.path == current.path;
            let response = ui.selectable_label(same, &saved.name);
            if response.clicked() {
                (app.ui.shelf_filter, app.ui.shelf_tab, app.ui.shelf_path) = (saved.filter.clone(), saved.tab, saved.path.clone());
            }
            response.context_menu(|ui| {
                if ui.button(tr("Delete")).clicked() {
                    deleted = Some(i);
                    ui.close();
                }
            });
        }
    });

    if let Some(i) = deleted {
        app.ui.shelf_saved_searches.remove(i);
    }
}

/// The shelf's query, named after its search text, else its folder, else its tab
fn current_query(app: &TextureCraftApp) -> SavedSearch {
    let (filter, tab, path) = (app.ui.shelf_filter.clone(), app.ui.shelf_tab, app.ui.shelf_path.clone());
    let name = if !filter.is_empty() {
        filter.clone()
    } else if let Some(folder) = path.last() {
        crate::shelf_folders::name(path.len() - 1, folder).to_string()
    } else {
        TABS.get(tab).map_or(tr("All"), |(_, _, label)| tr(label)).to_string()
    };
    SavedSearch { name, filter, tab, path }
}
