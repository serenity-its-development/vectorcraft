//! Tagged PDF: each text object is a paragraph of the structure tree, and each of its lines is
//! marked content whose ActualText is the line's characters in logical order, which text
//! extraction reads instead of glyphs drawn in visual order, along a path or as outlines.
//!
//! The SVG files in `tests/data` are unmodified copies of resvg test files (Apache-2.0 OR MIT,
//! attributed in ASSETS.md).

use serde_json::json;

use crate::*;

const TEXT_PATH: &str = include_str!("../tests/data/resvg-textPath-complex.svg");
const BIDI: &str = include_str!("../tests/data/resvg-bidi-reordering.svg");

/// The export of SVG `svg`, content streams left readable.
fn pdf(svg: &str, outline_text: bool) -> Vec<u8> {
    let doc = vectorcraft_testkit::svg::import(svg).unwrap();
    let settings: PdfSettings =
        serde_json::from_value(json!({"compression": {"compressText": false}, "advanced": {"outlineText": outline_text}})).unwrap();
    export(&doc, &PdfOptions { settings, ..Default::default() }).unwrap()
}

/// The ActualText strings of `pdf`'s marked content (UTF-16BE with a byte order mark, written
/// as hex strings), decoded.
fn actual_texts(pdf: &[u8]) -> Vec<String> {
    let s = String::from_utf8_lossy(pdf);
    s.split("/ActualText<")
        .skip(1)
        .map(|rest| {
            let hex = rest.split('>').next().unwrap();
            let bytes: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
            assert_eq!(bytes.get(..2), Some(&[0xfe, 0xff][..]), "ActualText has a UTF-16BE byte order mark");
            let units: Vec<u16> = bytes[2..].chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16(&units).unwrap()
        })
        .collect()
}

fn assert_tagged(pdf: &[u8]) {
    let s = String::from_utf8_lossy(pdf);
    assert!(s.contains("/StructTreeRoot"), "a structure tree");
    assert!(s.contains("/MarkInfo"), "marked as a tagged PDF");
    assert!(s.contains("/Marked true"), "marked as a tagged PDF");
    assert!(s.contains("/MCID 0"), "marked content tied to the structure tree");
}

/// What `pdftotext` (poppler) reads in `pdf`, or None (with the reason printed) when it isn't
/// installed.
fn pdftotext(pdf: &[u8], name: &str) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("vectorcraft-tagged-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.pdf");
    std::fs::write(&path, pdf).unwrap();
    let out = std::process::Command::new("pdftotext").args(["-enc", "UTF-8"]).arg(&path).arg("-").output();
    let _ = std::fs::remove_dir_all(&dir);
    match out {
        Ok(o) if o.status.success() => Some(String::from_utf8_lossy(&o.stdout).into_owned()),
        Ok(o) => panic!("pdftotext failed: {}", String::from_utf8_lossy(&o.stderr)),
        Err(e) => {
            println!("skipped the pdftotext check: pdftotext (poppler-utils) can't be run ({e})");
            None
        }
    }
}

/// The share of `words` found in `text`.
fn recall(text: &str, words: &[&str]) -> f64 {
    words.iter().filter(|w| text.contains(*w)).count() as f64 / words.len() as f64
}

#[test]
fn type_on_a_path_is_read_once_as_a_whole() {
    for outline_text in [false, true] {
        let pdf = pdf(TEXT_PATH, outline_text);
        assert_tagged(&pdf);
        let texts = actual_texts(&pdf);
        let all = texts.concat();
        // The whole string once, not glyph by glyph.
        assert_eq!(all.matches("非常に長いテキ").count(), 1, "{texts:?}");
        assert_eq!(all.matches("スト。").count(), 1, "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("非常に長いテキ")), "{texts:?}");
        // Outline Text writes no font, and poppler reads an ActualText only over shown text.
        assert_eq!(String::from_utf8_lossy(&pdf).contains("/Font"), !outline_text);
        if !outline_text && let Some(text) = pdftotext(&pdf, "path") {
            let r = recall(&text, &["非常に長いテキ", "スト。"]);
            println!("textPath: pdftotext recall {r}: {text:?}");
            assert_eq!(r, 1.0, "{text:?}");
        }
    }
}

#[test]
fn bidirectional_text_carries_its_logical_order() {
    for outline_text in [false, true] {
        let pdf = pdf(BIDI, outline_text);
        assert_tagged(&pdf);
        let texts = actual_texts(&pdf);
        assert!(texts.iter().any(|t| t == "اقرأ المزيد عن SVG أيضًا."), "{texts:?}");
        // No ActualText nests in the line's: a reader that takes the innermost would lose the
        // rest of the line.
        assert_eq!(texts.len(), 1, "{texts:?}");
        if !outline_text && let Some(text) = pdftotext(&pdf, "bidi") {
            // Poppler places an ActualText's characters left to right across the glyphs' box and
            // then reads a right-to-left line as visual order, reversing the Arabic words: only
            // the Latin word is read as written.
            let r = recall(&text, &["اقرأ", "المزيد", "عن", "SVG", "أيضًا."]);
            println!("bidi: pdftotext recall {r}: {text:?}");
            assert!(text.contains("SVG"), "{text:?}");
        }
    }
}

#[test]
fn a_document_without_text_is_not_tagged() {
    let pdf = pdf(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="5" height="5"/></svg>"#, false);
    assert!(!String::from_utf8_lossy(&pdf).contains("/StructTreeRoot"));
}
