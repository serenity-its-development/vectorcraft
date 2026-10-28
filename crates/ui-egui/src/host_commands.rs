//! The OneOS host bridge's half inside the window: the assistant's run (`craft:command {method: "command.batch"}`), the file the host opened,
//! and the place the host's read-only first view was left.
//!
//! A run is judged and recorded by `vectorcraft_engine::host_commands` (the allow-list, the arguments no run may carry, `$step`
//! references, ONE undo step or none). Here every registry step goes through [`VectorcraftApp::run`], the one path the menus and
//! the control channel use, so the interface follows (the selection, the views, the status line) exactly as it does for the
//! person. A step that would open a dialog is refused and the run taken back: a dialog never sits over the person's work.

use serde_json::Value;
use vectorcraft_engine::host_commands::run_batch;

use crate::VectorcraftApp;

/// Where the host's read-only first view was left, in the host's CSS pixels (the loader's `__craftFirstView`): the scroll offsets,
/// the size of what scrolled and the size of its window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FirstViewPlace {
    pub scroll_left: f64,
    pub scroll_top: f64,
    pub scroll_width: f64,
    pub scroll_height: f64,
    pub client_width: f64,
    pub client_height: f64,
}

impl FirstViewPlace {
    /// The first view's scale: CSS pixels of the host's picture per document point (`None` when it states none).
    fn scale(&self, artboard_width: f64) -> Option<f64> {
        let s = self.scroll_width / artboard_width.max(1.0);
        (s.is_finite() && s > 0.0).then_some(s)
    }

    /// The document point at the middle of the first view's window, for an artboard whose top-left is at `origin`.
    pub fn center(&self, artboard_width: f64, origin: [f64; 2]) -> Option<[f64; 2]> {
        let s = self.scale(artboard_width)?;
        let at = |o: f64, scroll: f64, client: f64| o + (scroll + client / 2.0) / s;
        let c = [at(origin[0], self.scroll_left, self.client_width), at(origin[1], self.scroll_top, self.client_height)];
        (c[0].is_finite() && c[1].is_finite()).then_some(c)
    }
}

impl VectorcraftApp {
    /// One `command.batch`: all of it, as ONE undo step, or none of it.
    pub fn run_command_batch(&mut self, params: &Value) -> Result<Value, String> {
        run_batch(self, |app| &mut app.session, |app, name, args| app.run_host_step(name, args), params)
    }

    fn run_host_step(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        if matches!(name, "inspect" | "render") {
            return self.session.run_argument_step(name, args);
        }
        let had_dialog = self.ui.dialog.is_some();
        let result = self.run(name, args.clone())?;
        if self.ui.dialog.is_some() && !had_dialog {
            // A command in the list never opens one; if it does, the dialog must not sit over the person's work.
            self.ui.dialog = None;
            return Err(format!("{name} needed a dialog"));
        }
        if result.get("pending").is_some() {
            return Err(format!("{name} did not finish at once"));
        }
        Ok(result)
    }

    /// Open the drawing the host sent. The document is bound to the host's file `name`: File › Save writes the next version of it,
    /// Save As makes a copy.
    pub fn open_host_file(&mut self, name: &str, bytes: &[u8]) -> Result<Value, String> {
        // The host's file is THE drawing of this window: opening it again (a restored version, the same file asked for twice) replaces
        // what was open rather than stacking a second tab bound to the same file.
        while let Some(last) = self.session.documents().len().checked_sub(1) {
            if !self.session.close_document(last) {
                break;
            }
        }
        self.sync_views();
        crate::io::open_bytes(self, name, bytes, Some(name.to_string()))
    }

    /// Which document the window shows (changes when a file is opened): the hand-over waits for it to change.
    pub fn shown_document(&self) -> Option<u64> {
        self.session.active().map(|st| st.uid)
    }

    /// Whether any document has changes the host does not hold yet.
    pub fn host_dirty(&self) -> bool {
        self.session.documents().iter().any(|d| d.is_dirty())
    }

    /// Open the active document at the place the host's first view was left (not an edit: the view's camera, nothing in the
    /// undo history).
    pub fn open_at_first_view(&mut self, place: FirstViewPlace) {
        let Some(board) = self.session.active().and_then(|st| st.doc.artboards.first().map(|a| a.rect)) else { return };
        let Some(center) = place.center(board.width(), [board.x0, board.y0]) else { return };
        let Some(scale) = place.scale(board.width()) else { return };
        if let Some(view) = self.view_mut() {
            view.zoom = scale.clamp(0.0313, 640.0);
            view.center = vectorcraft_geom::Point::new(center[0], center[1]);
            view.fitted = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn app() -> VectorcraftApp {
        let mut s = vectorcraft_engine::Session::new();
        s.execute("file.new", &json!({"width": 400, "height": 300, "units": "Points"})).unwrap();
        VectorcraftApp::new(s, crate::Services::default())
    }

    #[test]
    fn a_run_goes_through_the_window_and_is_one_undo_step() {
        let mut app = app();
        let before = app.session.active().unwrap().history.undo.len();
        let r = app
            .run_command_batch(&json!({"label": "Two shapes", "steps": [
                {"name": "shape.rectangle", "args": {"x": 10, "y": 10, "width": 50, "height": 30}},
                {"name": "shape.ellipse", "args": {"x": 80, "y": 10, "width": 40, "height": 40}},
            ]}))
            .unwrap();
        assert_eq!(r["undoStep"], json!(true));
        assert_eq!(app.session.active().unwrap().history.undo.len(), before + 1);
        assert_eq!(app.session.active().unwrap().history.undo.last().unwrap().label, "Two shapes");
    }

    #[test]
    fn a_step_the_drawing_forbids_takes_the_whole_run_back() {
        let mut app = app();
        let before = app.session.active().unwrap().history.undo.len();
        let e = app
            .run_command_batch(&json!({"steps": [
                {"name": "shape.rectangle", "args": {"x": 10, "y": 10, "width": 50, "height": 30}},
                {"name": "object.move", "args": {"ids": [999999], "dx": 1, "dy": 1}},
            ]}))
            .unwrap_err();
        assert!(e.starts_with("step-failed: step 2"), "{e}");
        assert_eq!(app.session.active().unwrap().history.undo.len(), before);
        assert_eq!(app.session.run_host_step("select.all", &json!({})).unwrap()["count"], json!(0));
    }

    #[test]
    fn the_allow_list_and_the_commands_left_out_are_the_whole_command_set_of_the_window() {
        use vectorcraft_engine::host_commands::{ARGUMENT_COMMANDS, ASSISTANT_COMMANDS, LEFT_OUT};
        // Every id a menu, a toolbar, a panel or the control channel can dispatch: the engine's registry and the window's own.
        let mut all: Vec<String> = vectorcraft_engine::command_specs().iter().map(|c| c.id.to_string()).filter(|id| !id.starts_with("test.")).collect();
        all.extend(crate::menus::UI_COMMANDS.iter().map(|c| c.0.to_string()));
        let mut seen = std::collections::HashSet::new();
        let (mut allowed, mut left) = (0, 0);
        for id in ASSISTANT_COMMANDS {
            assert!(all.iter().any(|c| c == id), "{id} is not a command of the app");
            assert!(seen.insert(id.to_string()), "{id} is listed twice");
            allowed += 1;
        }
        for (_, ids) in LEFT_OUT {
            for id in *ids {
                // (A command left out may be an alias the control channel also takes; what matters is that nothing real is forgotten.)
                assert!(seen.insert(id.to_string()), "{id} is both allowed and left out, or left out twice");
                left += 1;
            }
        }
        let missing: Vec<&String> = all.iter().filter(|c| !seen.contains(c.as_str()) && !ARGUMENT_COMMANDS.contains(&c.as_str())).collect();
        assert!(missing.is_empty(), "commands neither allowed nor left out (with a reason): {missing:?}");
        eprintln!("vector: {} commands, {allowed} allowed, {left} left out", all.len());
    }

    #[test]
    fn the_windows_own_commands_run_inside_the_same_run() {
        let mut app = app();
        let r = app
            .run_command_batch(&json!({"label": "Pick the pen and draw", "steps": [
                {"name": "tool.select", "args": {"tool": "pen"}},
                {"name": "shape.rectangle", "args": {"x": 10, "y": 10, "width": 30, "height": 20}},
            ]}))
            .unwrap();
        assert_eq!(r["steps"][0]["name"], json!("tool.select"));
        assert_eq!(app.session.active().unwrap().history.undo.last().unwrap().label, "Pick the pen and draw");
    }

    #[test]
    fn a_run_outside_the_list_changes_nothing() {
        let mut app = app();
        let e = app.run_command_batch(&json!({"steps": [{"name": "file.close"}]})).unwrap_err();
        assert!(e.starts_with("command-not-allowed"), "{e}");
        assert_eq!(app.session.documents().len(), 1);
    }

    #[test]
    fn the_host_file_is_bound_and_saves_as_its_next_version() {
        let written: Rc<RefCell<Vec<(String, Vec<u8>)>>> = Rc::default();
        let sink = written.clone();
        let services = crate::Services {
            download: Some(Box::new(move |name: &str, bytes: &[u8]| {
                sink.borrow_mut().push((name.to_string(), bytes.to_vec()));
                Ok(())
            })),
            ..Default::default()
        };
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), services);
        let svg =
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect x="5" y="5" width="40" height="30" fill="#0000ff"/></svg>"##;
        app.open_host_file("logo.svg", svg).unwrap();
        assert_eq!(app.session.active().unwrap().path.as_deref(), Some("logo.svg"));
        // Asked for again, the same file replaces the open one: still one document.
        app.open_host_file("logo.svg", svg).unwrap();
        assert_eq!(app.session.documents().len(), 1);
        assert!(!app.host_dirty());
        app.run_command_batch(&json!({"label": "Add", "steps": [{"name": "shape.ellipse", "args": {"x": 60, "y": 10, "width": 30, "height": 30}}]}))
            .unwrap();
        assert!(app.host_dirty());
        app.run("file.save", json!({"acknowledgeLoss": true})).unwrap();
        let writes = written.borrow();
        assert_eq!(writes.len(), 1, "{:?}", writes.iter().map(|w| &w.0).collect::<Vec<_>>());
        assert_eq!(writes[0].0, "logo.svg");
        let text = String::from_utf8_lossy(&writes[0].1).to_string();
        assert!(text.contains("<svg"), "{text}");
        drop(writes);
        assert!(!app.host_dirty(), "saved: the host holds it");
    }

    #[test]
    fn the_first_views_place_is_the_middle_of_its_window_in_document_points() {
        let place = FirstViewPlace {
            scroll_left: 200.0,
            scroll_top: 100.0,
            scroll_width: 800.0,
            scroll_height: 600.0,
            client_width: 400.0,
            client_height: 300.0,
        };
        // 800 css px for a 400 pt wide artboard: 2 css px per point, the window's middle at (400, 250) css = (200, 125) pt.
        assert_eq!(place.center(400.0, [0.0, 0.0]), Some([200.0, 125.0]));
        assert_eq!(FirstViewPlace::default().center(400.0, [0.0, 0.0]), None);
        let mut app = app();
        app.open_at_first_view(place);
        let view = app.view().unwrap();
        assert_eq!((view.center.x, view.center.y), (200.0, 125.0));
        assert!((view.zoom - 2.0).abs() < 1e-9);
        assert!(view.fitted);
    }
}
