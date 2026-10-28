//! The OneOS host bridge: `postMessage` to the ONE parent the frame page declared (`globalThis.__craftHost =
//! {app, version, parent}`, set by the build's `bootstrap.js` from the page's `data-parent`). The frame lives on its
//! own origin, so the parent is a DIFFERENT origin: messages go to that origin only, and only the parent window on
//! that origin is heard. Every message is a plain object with a `type`; `session` is the per-open nonce the host
//! mints in `craft:open` and every later message echoes (`plans/mona/craft-integration-design-2026-10-25.md` 2.2):
//!
//! - `craft:ready {app, version}` (to the parent): listening; send the file now.
//! - `craft:open {session, name, mime, bytes, versionId, readOnly, collab}` (from the parent): open these bytes.
//! - `craft:save {session, name, mime, bytes, baseVersionId, saveAs}` (to the parent): the person saved; the bytes are a
//!   transferred `ArrayBuffer`. The host makes a NEW version of the file through the compare-and-set and answers
//!   `craft:saved {session, versionId}` (the next save's base) or `craft:refused {session, code, reason}`.
//! - `craft:dirty {session, dirty}` (to the parent): unsaved work appeared or went away.
//! - `craft:failed {error}` (to the parent): the editor could not start.
//! - `craft:opened {session}` (to the parent): the drawing is loaded and drawn; the editor takes input. `craft:view {session, scrollTop,
//!   scrollHeight, clientHeight, ...}` (from the parent, kept by the loader on `globalThis.__craftFirstView`): where the host's
//!   read-only first view was left; the editor opens there and answers `craft:viewed {session}` once that is drawn (`handoff.rs`).
//! - `craft:command {session, id, method: "command.batch", params: {label, steps: [{name, args?}]}}` (from the parent) is
//!   the assistant's allow-listed run (`vectorcraft_engine::host_commands`): all of it as ONE undo step, or none of it. It is
//!   answered `craft:result {session, id, ok:true, result}` or `{ok:false, error: "<code>: <reason>"}` (a command outside
//!   the list, a step the drawing forbids, a file open read-only, or a `session` that is not the open one).
//!
//! A file opened read-only is never saved (the save fails by name); a document saved under a different name than the one
//! opened is saved as a copy (`saveAs`), never over the host's file. There is no server, no download and no file system:
//! every byte in or out moves through this bridge.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// Files the host opened, waiting for the next frame (the listener pushes, the shell drains).
pub type HostInbox = Arc<Mutex<Vec<(String, Vec<u8>)>>>;
/// Messages for the person (a save the host refused, by name), drained on the next frame.
pub type HostNotices = Arc<Mutex<Vec<String>>>;

/// One relayed run: the host's request id and its `command.batch` params.
pub struct HostRun {
    pub id: serde_json::Value,
    pub method: String,
    pub params: serde_json::Value,
}
/// Runs waiting for the next frame.
pub type HostRuns = Arc<Mutex<Vec<HostRun>>>;
/// The context a message wakes: the running app's (a restart after lost graphics sets the new one).
pub type Wake = Rc<RefCell<Option<egui::Context>>>;

/// What the host's last `craft:open` said.
#[derive(Default)]
struct Opened {
    session: String,
    name: String,
    version_id: String,
    read_only: bool,
}

#[derive(Clone)]
pub struct Bridge {
    window: web_sys::Window,
    parent: web_sys::Window,
    /// The declared parent origin: the only target and the only sender.
    origin: String,
    app: String,
    version: String,
    opened: Rc<RefCell<Opened>>,
}

/// `scheme://host[:port]` exactly: https, or http only on localhost (the dev estate).
pub fn plain_origin(v: &str) -> bool {
    let Some((scheme, rest)) = v.split_once("://") else { return false };
    let local = rest == "localhost" || rest.starts_with("localhost:") || rest.starts_with("127.0.0.1");
    (scheme == "https" || (scheme == "http" && local))
        && !rest.is_empty()
        && rest.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
}

impl Bridge {
    /// `None` when the page is not framed, or declares no parent origin.
    pub fn new() -> Option<Self> {
        let window = web_sys::window()?;
        let parent = window.parent().ok().flatten()?;
        if js_sys::Object::is(&parent, &window) {
            return None;
        }
        let declared = js_sys::Reflect::get(&window, &JsValue::from_str("__craftHost")).ok().filter(|d| d.is_object())?;
        let origin = string(&declared, "parent").filter(|o| plain_origin(o))?;
        Some(Self {
            window,
            parent,
            origin,
            app: string(&declared, "app").unwrap_or_else(|| "vector".into()),
            version: string(&declared, "version").unwrap_or_else(|| env!("CARGO_PKG_VERSION").into()),
            opened: Rc::default(),
        })
    }

    fn post(&self, message: &js_sys::Object, transfer: Option<&JsValue>) -> Result<(), String> {
        let sent = match transfer {
            Some(t) => self.parent.post_message_with_transfer(message, &self.origin, &js_sys::Array::of1(t)),
            None => self.parent.post_message(message, &self.origin),
        };
        sent.map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))
    }

    pub fn post_ready(&self) {
        if let Some(m) = message(&[("type", "craft:ready".into()), ("app", self.app.as_str().into()), ("version", self.version.as_str().into())]) {
            let _ = self.post(&m, None);
        }
    }

    /// `craft:opened {session}`: the drawing is loaded and drawn, the editor takes input (handoff.rs).
    pub fn post_opened(&self) {
        let session = self.opened.borrow().session.clone();
        if let Some(m) = message(&[("type", "craft:opened".into()), ("session", session.as_str().into())]) {
            let _ = self.post(&m, None);
        }
    }

    /// `craft:viewed {session}`: the first view's place is applied and drawn; the host may let the first view go.
    pub fn post_viewed(&self) {
        let session = self.opened.borrow().session.clone();
        if let Some(m) = message(&[("type", "craft:viewed".into()), ("session", session.as_str().into())]) {
            let _ = self.post(&m, None);
        }
    }

    /// The app couldn't start (no WebGL, for example): let the parent say so.
    pub fn post_failed(&self, error: &str) {
        if let Some(m) = message(&[("type", "craft:failed".into()), ("error", error.into())]) {
            let _ = self.post(&m, None);
        }
    }

    /// Hear the host: `craft:open` queues the file; `craft:saved` and `craft:refused` answer a save; `craft:command` queues a run.
    pub fn listen(&self, inbox: HostInbox, notices: HostNotices, runs: HostRuns, wake: Wake) {
        let bridge = self.clone();
        let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            if event.origin() != bridge.origin || !event.source().is_some_and(|s| js_sys::Object::is(&s, &bridge.parent)) {
                return;
            }
            let data = event.data();
            let Some(kind) = string(&data, "type") else { return };
            match kind.as_str() {
                "craft:open" => {
                    let Some(session) = string(&data, "session").filter(|s| !s.is_empty()) else { return };
                    let name = string(&data, "name").filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "drawing.svg".to_string());
                    let Ok(bytes) = js_sys::Reflect::get(&data, &JsValue::from_str("bytes")) else { return };
                    let bytes = if let Some(array) = bytes.dyn_ref::<js_sys::Uint8Array>() {
                        array.to_vec()
                    } else if bytes.is_instance_of::<js_sys::ArrayBuffer>() {
                        js_sys::Uint8Array::new(&bytes).to_vec()
                    } else {
                        return;
                    };
                    *bridge.opened.borrow_mut() = Opened {
                        session,
                        name: name.clone(),
                        version_id: string(&data, "versionId").unwrap_or_default(),
                        read_only: boolean(&data, "readOnly"),
                    };
                    if let Ok(mut q) = inbox.lock() {
                        q.push((name, bytes));
                    }
                    crate::handoff::wasm::note_open();
                }
                // The loader keeps the place on `__craftFirstView`; the app reads it (handoff.rs) - this wakes it.
                "craft:view" => {}
                "craft:saved" => {
                    let mut opened = bridge.opened.borrow_mut();
                    if string(&data, "session").as_deref() == Some(opened.session.as_str())
                        && let Some(version) = string(&data, "versionId")
                    {
                        opened.version_id = version;
                    }
                }
                "craft:refused" => {
                    if string(&data, "session").as_deref() == Some(bridge.opened.borrow().session.as_str()) {
                        let reason = string(&data, "reason").unwrap_or_else(|| "refused".into());
                        if let Ok(mut q) = notices.lock() {
                            q.push(format!("Couldn't save: {reason}"));
                        }
                    }
                }
                "craft:command" => {
                    let session = string(&data, "session").unwrap_or_default();
                    let id = js_sys::Reflect::get(&data, &JsValue::from_str("id")).ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let (current, read_only) = {
                        let opened = bridge.opened.borrow();
                        (opened.session.clone(), opened.read_only)
                    };
                    if session.is_empty() || session != current {
                        bridge.post_result(&session, id, Err("session-mismatch: this command is for another open of the file".into()));
                    } else if read_only {
                        bridge.post_result(&session, id, Err("read-only: this file is open to view; the assistant cannot change it".into()));
                    } else {
                        let params = js_sys::Reflect::get(&data, &JsValue::from_str("params"))
                            .ok()
                            .and_then(|p| js_sys::JSON::stringify(&p).ok())
                            .and_then(|s| s.as_string())
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or(serde_json::Value::Null);
                        let run = HostRun { id: serde_json::json!(id), method: string(&data, "method").unwrap_or_default(), params };
                        if let Ok(mut q) = runs.lock() {
                            q.push(run);
                        }
                    }
                }
                _ => {}
            }
            if let Some(ctx) = wake.borrow().as_ref() {
                ctx.request_repaint();
            }
        });
        if self.window.add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref()).is_ok() {
            // The listener lives as long as the page.
            on_message.forget();
        }
    }

    /// `craft:result {session, id, ok, result|error}` to the parent.
    pub fn post_result(&self, session: &str, id: f64, outcome: Result<serde_json::Value, String>) {
        let mut fields: Vec<(&str, JsValue)> =
            vec![("type", "craft:result".into()), ("session", session.into()), ("id", id.into()), ("ok", outcome.is_ok().into())];
        match &outcome {
            Ok(value) => {
                let result = js_sys::JSON::parse(&value.to_string()).unwrap_or(JsValue::NULL);
                fields.push(("result", result));
            }
            Err(error) => fields.push(("error", error.as_str().into())),
        }
        if let Some(m) = message(&fields) {
            let _ = self.post(&m, None);
        }
    }

    /// Answer a relayed run, on the session it was asked in.
    pub fn answer(&self, id: &serde_json::Value, outcome: Result<serde_json::Value, String>) {
        let session = self.opened.borrow().session.clone();
        self.post_result(&session, id.as_f64().unwrap_or(0.0), outcome);
    }

    /// The write service while framed: the bytes of a save, a Save As or an export go to the host (which makes a new version of the
    /// opened file, or a copy under another name). Nothing is downloaded.
    pub fn save_fn(&self) -> vectorcraft_ui_egui::DownloadFn {
        let bridge = self.clone();
        Box::new(move |path: &str, bytes: &[u8]| {
            let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
            let (session, base, as_copy) = {
                let opened = bridge.opened.borrow();
                if opened.session.is_empty() {
                    return Err("the host has not opened a drawing yet".to_string());
                }
                if opened.read_only {
                    return Err("this file is open to view; saving is off".to_string());
                }
                (opened.session.clone(), opened.version_id.clone(), name != opened.name)
            };
            let buffer = js_sys::Uint8Array::from(bytes).buffer();
            let m = message(&[
                ("type", "craft:save".into()),
                ("session", session.as_str().into()),
                ("name", name.into()),
                ("mime", mime_of(name).into()),
                ("baseVersionId", base.as_str().into()),
                ("saveAs", as_copy.into()),
                ("bytes", buffer.clone().into()),
            ])
            .ok_or_else(|| "couldn't build the message".to_string())?;
            bridge.post(&m, Some(&buffer))
        })
    }

    /// Tell the host whether there is unsaved work (once per change).
    pub fn dirty_fn(&self) -> Box<dyn FnMut(bool)> {
        let bridge = self.clone();
        let mut last: Option<(String, bool)> = None;
        Box::new(move |dirty: bool| {
            let session = bridge.opened.borrow().session.clone();
            if session.is_empty() || last.as_ref() == Some(&(session.clone(), dirty)) {
                return;
            }
            if let Some(m) = message(&[("type", "craft:dirty".into()), ("session", session.as_str().into()), ("dirty", dirty.into())])
                && bridge.post(&m, None).is_ok()
            {
                last = Some((session, dirty));
            }
        })
    }
}

impl crate::handoff::Link for Bridge {
    fn open_seq(&self) -> u64 {
        crate::handoff::wasm::opens()
    }
    fn take_view(&self) -> Option<crate::handoff::FirstView> {
        crate::handoff::wasm::take_view(&self.opened.borrow().session)
    }
    fn post_opened(&self) {
        Bridge::post_opened(self);
    }
    fn post_viewed(&self) {
        Bridge::post_viewed(self);
    }
}

/// The media type the host stores a saved file under: the format's own, except that a PDF-compatible `.ai` is stated as Illustrator's.
fn mime_of(name: &str) -> &'static str {
    match name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("ai") => "application/illustrator",
        _ => vectorcraft_engine::cmd::fileio::format_for_name(name).map_or("application/octet-stream", |f| f.mime),
    }
}

fn message(fields: &[(&str, JsValue)]) -> Option<js_sys::Object> {
    let m = js_sys::Object::new();
    for (key, value) in fields {
        js_sys::Reflect::set(&m, &JsValue::from_str(key), value).ok()?;
    }
    Some(m)
}

fn string(data: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(data, &JsValue::from_str(key)).ok()?.as_string()
}

fn boolean(data: &JsValue, key: &str) -> bool {
    js_sys::Reflect::get(data, &JsValue::from_str(key)).ok().and_then(|v| v.as_bool()).unwrap_or(false)
}
