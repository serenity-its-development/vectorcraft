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

    /// The bytes File › Save would write for the host's file `name`, as its document stands: the answer to the host's ask for an
    /// export (auto-save, save on close). Nothing here is a save: the window and the document are only read (`&self`), so no text
    /// being typed and no tool is committed, the document keeps its unsaved state, its history and its dates, nothing is shown or
    /// journaled, and nothing goes through the write service. The format and its options are those of a plain Save of the host's
    /// file (`fileio::bytes_of_a_save`), with the view a Save notes first (`io::remember_view`) when the window shows that document.
    ///
    /// Refused when no open document is the host's file: it did not open, the person saved it under another name, or it was opened
    /// from a format Save does not write back (EPS and every other import, a `.ai` file without the native document, a template, a
    /// PDF opened in part), which is never exported as another format. Refused too where a Save would ask first (see there).
    pub fn host_export_bytes(&self, name: &str) -> Result<Vec<u8>, String> {
        let st = self.session.documents().iter().find(|st| st.path.as_deref() == Some(name)).ok_or_else(|| format!("no open document is {name}"))?;
        let shown = self.session.active().is_some_and(|active| active.uid == st.uid);
        let view = self.view().filter(|v| shown && v.fitted).map(|v| vectorcraft_doc::SavedView {
            name: String::new(),
            center: v.center,
            zoom: v.zoom,
            rotation: v.rotation,
        });
        vectorcraft_engine::cmd::fileio::bytes_of_a_save(&self.session, st, view.as_ref()).map_err(|e| e.to_string())
    }

    /// The host's ask for an export, answered: the file's name and bytes. A document whose own file Save writes back
    /// (`fileio::saves_back`) answers with those bytes under the host's own name ([`Self::host_export_bytes`]). A document opened
    /// from a file Save does not write back (EPS and the other imports, a `.ai` file without the native document, a lossy read, a
    /// converted or recovered document) answers with the native file Save As writes, named `<stem>.vectorcraft`: the host keeps it
    /// as a NEW file beside the original, which is never written (oneos files/038, the import rule). Refused when no open document
    /// is the host's file (it did not open, or the person saved it under another name), and when the save is more than one file.
    pub fn host_export(&self, name: &str) -> Result<(String, Vec<u8>), String> {
        let docs = self.session.documents();
        if docs.iter().any(|st| vectorcraft_engine::cmd::fileio::saves_back(st, name)) {
            return self.host_export_bytes(name).map(|bytes| (name.to_string(), bytes));
        }
        let st = docs
            .iter()
            .find(|st| st.path.as_deref() == Some(name))
            // An import has no file of its own until it is saved; one saved under another name is not the host's file any more.
            .or_else(|| docs.iter().find(|st| st.path.is_none() && (st.imported_from.as_deref() == Some(name) || docs.len() == 1)))
            .ok_or_else(|| format!("no open document is {name}"))?;
        let stem = vectorcraft_engine::cmd::fileio::file_stem(name);
        vectorcraft_engine::cmd::fileio::bytes_of_a_native_save(&self.session, st, &stem).map_err(|e| e.to_string())
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

    /// The host's ask for the bytes (`craft:export`) is never a save: the bytes are what Save would write, and the document is as
    /// it was (unsaved, same revision, same history, nothing written, journaled or shown).
    #[test]
    fn an_export_for_the_host_is_the_bytes_of_a_save_and_leaves_the_document_unsaved_and_unchanged() {
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
        assert!(app.host_export_bytes("logo.svg").is_err(), "nothing is open yet");
        let svg =
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect x="5" y="5" width="40" height="30" fill="#0000ff"/></svg>"##;
        app.open_host_file("logo.svg", svg).unwrap();
        let opened = app.host_export_bytes("logo.svg").unwrap();
        app.run_command_batch(&json!({"label": "Add", "steps": [{"name": "shape.ellipse", "args": {"x": 60, "y": 10, "width": 30, "height": 30}}]}))
            .unwrap();
        assert!(app.host_dirty());
        assert!(app.host_export_bytes("other.svg").is_err(), "only the host's own file is exported");

        let before = {
            let st = app.session.active().unwrap();
            (st.doc.clone(), st.revision, st.path.clone(), st.format, st.view.clone(), st.history.undo.len(), st.history.redo.len())
        };
        let (journal, status, dialog) = (app.session.journal.len(), app.ui.status.clone(), app.ui.dialog.is_some());
        let first = app.host_export_bytes("logo.svg").unwrap();
        let second = app.host_export_bytes("logo.svg").unwrap();
        assert!(String::from_utf8_lossy(&first).contains("<svg"), "the host's file's own format");
        assert_ne!(first, opened, "with the work not saved yet");
        assert_eq!(first, second, "asked twice, the same bytes");

        let st = app.session.active().unwrap();
        assert!(std::sync::Arc::ptr_eq(&st.doc, &before.0), "the very same document: no date was stamped on it");
        assert_eq!((st.revision, st.path.clone(), st.format, st.view.clone()), (before.1, before.2.clone(), before.3, before.4.clone()));
        assert_eq!((st.history.undo.len(), st.history.redo.len()), (before.5, before.6), "no step was added to the history");
        assert!(app.host_dirty(), "an export is not a save: the work is still unsaved");
        assert_eq!((app.session.journal.len(), app.ui.status.clone(), app.ui.dialog.is_some()), (journal, status, dialog));
        assert!(written.borrow().is_empty(), "nothing went through the write service");

        // The person's Save of the same document writes exactly those bytes (`modified: null`: no clock, as in the browser).
        app.run("file.save", json!({"modified": null})).unwrap();
        assert_eq!(written.borrow().as_slice(), &[("logo.svg".to_string(), first)]);
        assert!(!app.host_dirty());

        // Saved under another name, no open document is the host's file any more: nothing is exported in its place.
        app.run("file.saveAs", json!({"path": "copy.svg", "options": {}})).unwrap();
        assert_eq!(app.session.active().unwrap().path.as_deref(), Some("copy.svg"));
        assert!(app.host_export_bytes("logo.svg").is_err());
    }

    /// Every format Save writes back over the host's file answers the export with exactly the bytes its Save writes: compressed SVG,
    /// an Illustrator file and a PDF, each made by Vector's own Save As and opened as the host's file.
    #[test]
    fn an_export_for_the_host_is_the_bytes_of_a_save_in_svgz_ai_and_pdf_too() {
        let written: Rc<RefCell<Vec<(String, Vec<u8>)>>> = Rc::default();
        let services = |sink: Rc<RefCell<Vec<(String, Vec<u8>)>>>| crate::Services {
            download: Some(Box::new(move |name: &str, bytes: &[u8]| {
                sink.borrow_mut().push((name.to_string(), bytes.to_vec()));
                Ok(())
            })),
            ..Default::default()
        };
        let svg =
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect x="5" y="5" width="40" height="30" fill="#0000ff"/></svg>"##;
        for name in ["logo.svgz", "logo.ai", "logo.pdf"] {
            // The file as Vector itself writes it in that format.
            let mut maker = VectorcraftApp::new(vectorcraft_engine::Session::new(), services(written.clone()));
            maker.open_host_file("logo.svg", svg).unwrap();
            maker.run("file.saveAs", json!({"path": name, "options": {}, "modified": null})).unwrap();
            let file = written.borrow_mut().pop().unwrap_or_else(|| panic!("{name}: Save As wrote nothing")).1;

            let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), services(written.clone()));
            app.open_host_file(name, &file).unwrap();
            app.run_command_batch(&json!({"label": "Add", "steps": [{"name": "shape.ellipse", "args": {"x": 60, "y": 10, "width": 30, "height": 30}}]}))
                .unwrap();
            let exported = app.host_export_bytes(name).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(app.host_dirty(), "{name}: an export is not a save");
            assert!(written.borrow().is_empty(), "{name}: nothing went through the write service");
            app.run("file.save", json!({"modified": null})).unwrap();
            assert_eq!(written.borrow_mut().pop().map(|(n, b)| (n, b == exported)), Some((name.to_string(), true)), "{name}: Save writes the exported bytes");
        }
    }

    /// A document Save does not write back over the host's file (here: converted from an older version, and restored by Data
    /// Recovery) is answered with the NATIVE file under `<stem>.vectorcraft`, holding the edit; the host keeps it beside the
    /// original. NEGATIVE: a file Save writes back is answered in its own format under its own name, and a document saved under
    /// another name is not answered at all.
    #[test]
    fn an_import_is_exported_as_a_native_file_named_beside_the_original() {
        let svg =
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect x="5" y="5" width="40" height="30" fill="#0000ff"/></svg>"##;
        let services = crate::Services { download: Some(Box::new(|_: &str, _: &[u8]| Ok(()))), ..Default::default() };
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), services);
        app.open_host_file("logo.svg", svg).unwrap();
        let (own, bytes) = app.host_export("logo.svg").unwrap();
        assert_eq!(own, "logo.svg", "a file Save writes back keeps its own name");
        assert!(String::from_utf8_lossy(&bytes).contains("<svg"), "and its own format");
        app.run_command_batch(&json!({"label": "Add", "steps": [{"name": "shape.ellipse", "args": {"x": 60, "y": 10, "width": 30, "height": 30}}]}))
            .unwrap();
        for flag in ["converted", "recovered"] {
            {
                let st = app.session.active_mut().unwrap();
                st.converted = flag == "converted";
                st.recovered = flag == "recovered";
            }
            let revision = app.session.active().unwrap().revision;
            let (name, native) = app.host_export("logo.svg").unwrap_or_else(|e| panic!("{flag}: {e}"));
            assert_eq!(name, "logo.vectorcraft", "{flag}: the native file, named beside the original");
            let mut back = VectorcraftApp::new(vectorcraft_engine::Session::new(), crate::Services::default());
            back.open_host_file("logo.vectorcraft", &native).unwrap_or_else(|e| panic!("{flag}: the native file opens: {e}"));
            let count = |s: &mut vectorcraft_engine::Session| s.execute("select.all", &json!({})).unwrap()["count"].as_u64().unwrap_or(0);
            assert_eq!(count(&mut back.session), 2, "{flag}: the rectangle and the edit's ellipse");
            assert!(app.host_dirty() && app.session.active().unwrap().revision == revision, "{flag}: an export is not a save");
        }
        {
            let st = app.session.active_mut().unwrap();
            st.converted = false;
            st.recovered = false;
        }
        app.run("file.saveAs", json!({"path": "copy.svg", "options": {}})).unwrap();
        assert_eq!(app.session.active().unwrap().path.as_deref(), Some("copy.svg"));
        assert!(app.host_export("logo.svg").is_err(), "saved under another name: nothing is exported in its place");
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
