use serde_json::{Value, json};

use super::*;
use crate::Session;

fn session_with_drawing() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 200, "height": 120, "units": "Points"})).unwrap();
    s
}

fn run(s: &mut Session, label: &str, steps: Value) -> Result<Value, String> {
    s.run_command_batch(&json!({"label": label, "steps": steps}))
}

fn undo_depth(s: &Session) -> usize {
    s.active().unwrap().history.undo.len()
}

fn object_count(s: &mut Session) -> usize {
    let r = s.execute("select.all", &json!({})).unwrap();
    r["count"].as_u64().unwrap_or(0) as usize
}

/// Names that say "save" or "print" but touch only the document: a selection saved into a channel, a pattern saved as a swatch, a view option.
const NAME_ONLY: &[&str] = &["select.save", "object.pattern.saveCopy", "view.printTiling"];

#[test]
fn nothing_allowed_is_listed_twice_or_left_out_too_and_nothing_allowed_is_undo_or_a_file() {
    let words = |id: &str| -> Vec<String> {
        let mut out = vec![];
        let mut cur = String::new();
        for ch in id.chars() {
            if ch == '.' || ch.is_uppercase() {
                if !cur.is_empty() {
                    out.push(cur.to_lowercase());
                }
                cur = if ch == '.' { String::new() } else { ch.to_string() };
            } else {
                cur.push(ch);
            }
        }
        out.push(cur.to_lowercase());
        out
    };
    let forbidden = ["file", "files", "save", "export", "import", "download", "upload", "plugin", "script", "print", "package", "library", "revert", "recovery"];
    let mut seen = std::collections::HashSet::new();
    for id in ASSISTANT_COMMANDS {
        assert!(seen.insert(*id), "{id} is listed twice");
        assert!(!matches!(*id, "edit.undo" | "edit.redo" | "edit.undoGroup.begin" | "edit.undoGroup.end" | "edit.clearHistory"), "{id}: undoing a run is the person's");
        assert!(!id.starts_with("file.") && !id.starts_with("clipboard.") && !id.starts_with("links."), "{id} reaches a file");
        if NAME_ONLY.contains(id) {
            continue;
        }
        assert!(!words(id).iter().any(|x| forbidden.contains(&x.as_str())), "{id} reaches a file");
    }
    for (reason, ids) in LEFT_OUT {
        assert!(reason.len() > 20, "a reason: {reason}");
        for id in *ids {
            assert!(seen.insert(*id), "{id} is both allowed and left out, or left out twice");
        }
    }
    for id in ARGUMENT_COMMANDS {
        assert!(seen.insert(*id), "{id} is listed twice");
    }
    assert_eq!(allowed().len(), ASSISTANT_COMMANDS.len() + ARGUMENT_COMMANDS.len());
}

#[test]
fn a_run_is_judged_before_anything_runs() {
    let mut s = session_with_drawing();
    let before = undo_depth(&s);
    for (steps, code) in [
        (
            json!([{"name": "shape.rectangle", "args": {"x": 0, "y": 0, "width": 10, "height": 10}}, {"name": "file.saveAs", "args": {"path": "/tmp/x.svg"}}]),
            "command-not-allowed",
        ),
        (json!([{"name": "edit.undo"}]), "command-not-allowed"),
        (json!([{"name": "shape.rectangle", "args": {"x": 0, "y": 0, "width": 10, "height": 10, "path": "/etc/passwd"}}]), "argument-not-allowed"),
        (json!([{"name": "shape.rectangle", "args": {"nested": {"url": "https://x"}}}]), "argument-not-allowed"),
        (json!([]), "run-empty"),
    ] {
        let e = run(&mut s, "x", steps).unwrap_err();
        assert!(e.starts_with(code), "{e}");
    }
    let many: Vec<Value> = (0..=MAX_STEPS).map(|_| json!({"name": "select.none"})).collect();
    assert!(run(&mut s, "x", Value::Array(many)).unwrap_err().starts_with("run-too-long"));
    assert!(run(&mut s, &"l".repeat(MAX_LABEL + 1), json!([{"name": "select.none"}])).unwrap_err().starts_with("label-too-long"));
    assert_eq!(undo_depth(&s), before, "nothing ran");
    assert_eq!(object_count(&mut s), 0);
}

#[test]
fn a_run_is_one_undo_step_and_later_steps_name_the_object_an_earlier_one_made() {
    let mut s = session_with_drawing();
    let before = undo_depth(&s);
    let r = run(
        &mut s,
        "A red card",
        json!([
            {"name": "shape.rectangle", "args": {"x": 20, "y": 20, "width": 80, "height": 50, "radius": 6}},
            {"name": "paint.setFill", "args": {"ids": [{"$step": 0}], "color": "#ff0000"}},
            {"name": "text.create", "args": {"x": 30, "y": 50, "text": "Hello", "size": 18}},
            {"name": "inspect", "args": {"depth": 3}},
        ]),
    )
    .unwrap();
    assert_eq!(r["undoStep"], json!(true));
    assert_eq!(undo_depth(&s), before + 1, "three edits, ONE undo step");
    assert_eq!(s.active().unwrap().history.undo.last().unwrap().label, "A red card");
    let seen = serde_json::to_string(&r["steps"][3]["result"]).unwrap();
    assert!(seen.contains("#ff0000") || seen.contains("ff0000") || seen.contains("255"), "{seen}");
    assert_eq!(object_count(&mut s), 2);
    // One undo takes the whole run back.
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(object_count(&mut s), 0);
}

#[test]
fn a_failing_step_takes_back_every_step_before_it() {
    let mut s = session_with_drawing();
    let depth = undo_depth(&s);
    let journal = s.journal.len();
    let e = run(
        &mut s,
        "Doomed",
        json!([
            {"name": "shape.rectangle", "args": {"x": 0, "y": 0, "width": 30, "height": 30}},
            {"name": "shape.ellipse", "args": {"x": 40, "y": 0, "width": 30, "height": 30}},
            {"name": "object.move", "args": {"ids": [999999], "dx": 5, "dy": 5}},
        ]),
    )
    .unwrap_err();
    assert!(e.starts_with("step-failed: step 3 (object.move)"), "{e}");
    assert!(e.ends_with("nothing was changed"), "{e}");
    assert_eq!(object_count(&mut s), 0, "the shapes are gone");
    assert_eq!(undo_depth(&s), depth, "no undo step");
    assert_eq!(s.journal.len(), journal + 1, "the journal forgot the run (the select.all above is the only new entry)");
}

#[test]
fn a_step_that_names_a_step_that_has_not_run_is_refused() {
    let mut s = session_with_drawing();
    let e = run(&mut s, "x", json!([{"name": "object.move", "args": {"ids": [{"$step": 3}], "dx": 1, "dy": 1}}])).unwrap_err();
    assert!(e.contains("must be an earlier step"), "{e}");
}

#[test]
fn render_gives_a_png_no_larger_than_1600_a_side() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 3200, "height": 400, "units": "Points"})).unwrap();
    run(&mut s, "Box", json!([{"name": "shape.rectangle", "args": {"x": 10, "y": 10, "width": 300, "height": 100}}])).unwrap();
    let r = run(&mut s, "look", json!([{"name": "render"}])).unwrap();
    let png = &r["steps"][0]["result"];
    assert_eq!(png["mime"], json!("image/png"));
    let bytes = vectorcraft_format::base64_decode(png["png"].as_str().unwrap()).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    assert!(width <= 1600 && height <= 1600, "{width}x{height}");
    assert!(width >= 1500, "{width}x{height}");
}

#[test]
fn the_assistant_edits_a_drawing_and_the_next_version_is_the_edited_svg() {
    // The scripted policy of the AI-drive test, on the engine alone: open an SVG, draw a badge on it, save.
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80" viewBox="0 0 120 80"><rect x="5" y="5" width="40" height="30" fill="#0000ff"/></svg>"##;
    let mut s = Session::new();
    s.execute("document.open", &json!({"name": "badge.svg", "dataBase64": vectorcraft_format::base64_encode(svg)})).unwrap();
    let before = s.export_active("badge.svg").unwrap();
    let before_text = String::from_utf8_lossy(&before).to_lowercase();
    assert!(before_text.contains("0000ff"), "the logo is blue in the file as opened: {before_text}");
    run(
        &mut s,
        "Add a badge",
        json!([
            {"name": "shape.ellipse", "args": {"x": 60, "y": 10, "width": 40, "height": 40}},
            {"name": "paint.setFill", "args": {"ids": [{"$step": 0}], "color": "#00aa00"}},
            {"name": "text.create", "args": {"x": 62, "y": 70, "text": "OK", "size": 12}},
        ]),
    )
    .unwrap();
    let after = s.export_active("badge.svg").unwrap();
    let text = String::from_utf8_lossy(&after).to_string();
    assert!(text.starts_with("<?xml") || text.starts_with("<svg"), "{text}");
    assert!(text.to_lowercase().contains("0000ff"), "the original logo is still there: {text}");
    assert!(text.contains("ellipse") || text.contains("circle") || text.matches("<path").count() >= 1, "the ellipse was drawn: {text}");
    assert!(text.to_lowercase().contains("00aa00"), "and filled green: {text}");
    // The version history of the drawing is one step.
    assert_eq!(s.active().unwrap().history.undo.last().unwrap().label, "Add a badge");
}
