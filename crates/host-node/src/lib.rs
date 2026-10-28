//! Vector's host window for node.
//!
//! What the browser build does with the host's `craft:open`, `craft:command` and `craft:save` (`apps/vectorcraft-web`), on the engine alone:
//! a [`Window`] opens a drawing from its bytes, runs the assistant's `command.batch` (`vectorcraft_engine::host_commands`, the function the
//! window answers `craft:command` with) and writes the file a Save would hand the host (`file_b64`). OneOS's tests drive a drawing through
//! the `vector.*` tools with it; nothing here is a second implementation.

use vectorcraft_engine::Session;
use wasm_bindgen::prelude::*;

/// One open drawing.
#[wasm_bindgen]
pub struct Window {
    session: Session,
    name: String,
}

fn err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

#[wasm_bindgen]
impl Window {
    /// Open `name`'s bytes (base64), or a new 200 by 120 pt drawing when there are none. `client` is the party number the host would
    /// give the window; this build has no collaboration, so it is unused.
    #[wasm_bindgen(constructor)]
    pub fn new(_client: u32, name: String, file_b64: String) -> Result<Window, JsError> {
        let mut session = Session::new();
        if file_b64.is_empty() {
            session.execute("file.new", &serde_json::json!({"width": 200, "height": 120, "units": "Points"})).map_err(err)?;
        } else {
            session.execute("document.open", &serde_json::json!({"name": name, "dataBase64": file_b64})).map_err(err)?;
        }
        Ok(Window { session, name })
    }

    /// The window's `command.batch`: `params` is the host's `{label, steps}` as JSON; the answer is the `result` of `craft:result`
    /// as JSON, or an error carrying its `<code>: <reason>` text.
    pub fn command_batch(&mut self, params_json: &str) -> Result<String, JsError> {
        let params: serde_json::Value = serde_json::from_str(params_json).map_err(err)?;
        let result = self.session.run_command_batch(&params).map_err(|e| JsError::new(&e))?;
        Ok(result.to_string())
    }

    /// A registry command as the person's own menu runs it (Undo, in the tests), answered as JSON.
    pub fn run(&mut self, command: &str, params_json: &str) -> Result<String, JsError> {
        let params: serde_json::Value = serde_json::from_str(params_json).map_err(err)?;
        let result = self.session.execute(command, &params).map_err(err)?;
        Ok(result.to_string())
    }

    /// The file a Save would hand the host, base64: the drawing encoded as the file it was opened as.
    pub fn file_b64(&mut self) -> Result<String, JsError> {
        let name = self.name.clone();
        let bytes = self.session.export_active(&name).map_err(|e| JsError::new(&e))?;
        Ok(vectorcraft_format::base64_encode(&bytes))
    }

    /// Whether the drawing has changes the host does not hold yet.
    pub fn dirty(&self) -> bool {
        self.session.active().is_some_and(|d| d.is_dirty())
    }
}
