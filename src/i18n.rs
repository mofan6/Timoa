//! One embedded catalog for declarative UI, system integration and application messages.
use std::{collections::BTreeMap, sync::{OnceLock, atomic::{AtomicU8, Ordering}}};
static LANGUAGE: AtomicU8 = AtomicU8::new(0);
static CATALOG: OnceLock<BTreeMap<String, [String; 2]>> = OnceLock::new();
fn catalog() -> &'static BTreeMap<String, [String; 2]> {
    CATALOG.get_or_init(|| serde_json::from_str(include_str!("../i18n/messages.json")).expect("embedded language catalog"))
}
pub fn language() -> i32 { LANGUAGE.load(Ordering::Relaxed) as i32 }
pub fn select(value: i32) {
    let value = value.clamp(0, 2);
    LANGUAGE.store(value as u8, Ordering::Relaxed);
    let _ = slint::select_bundled_translation(["zh_CN", "zh_TW", "en"][value as usize]);
}
pub fn t(source: &str) -> &str {
    let language = language();
    if language == 0 { source }
    else { catalog().get(source).map(|row| row[language as usize - 1].as_str()).unwrap_or(source) }
}
fn parts(template: &str) -> Vec<&str> {
    let mut result = Vec::new(); let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        let Some(end) = remaining[start..].find('}') else { break; };
        result.push(&remaining[..start]); remaining = &remaining[start + end + 1..];
    }
    result.push(remaining); result
}
fn capture<'a>(template: &str, input: &'a str) -> Option<Vec<&'a str>> {
    let pieces = parts(template);
    if pieces.len() < 2 || pieces[0].is_empty() { return None; }
    let mut remaining = input.strip_prefix(pieces[0])?;
    let mut values = Vec::new();
    for piece in &pieces[1..] {
        if piece.is_empty() { values.push(remaining); remaining = ""; }
        else {
            let position = remaining.find(piece)?;
            values.push(&remaining[..position]); remaining = &remaining[position + piece.len()..];
        }
    }
    remaining.is_empty().then_some(values)
}
// Canonical and already displayed system messages can both be refreshed when language changes.
// Callers keep user-entered names outside this translation path.
pub fn message(input: &str) -> String {
    if catalog().contains_key(input) { return t(input).into(); }
    for (source, row) in catalog() {
        if row.iter().any(|value| value == input) { return t(source).into(); }
    }
    for (source, row) in catalog().iter().filter(|(source, _)| source.contains('{')) {
        for template in [source.as_str(), row[0].as_str(), row[1].as_str()] {
            if let Some(values) = capture(template, input) {
                let target = parts(t(source));
                if target.len() != values.len() + 1 { continue; }
                let mut output = target[0].to_owned();
                for (value, next) in values.iter().zip(&target[1..]) { output.push_str(value); output.push_str(next); }
                return output;
            }
        }
    }
    input.into()
}
pub fn date(date: chrono::NaiveDate) -> String {
    date.format(if language() == 2 { "%b %d" } else { "%m月%d日" }).to_string()
}
pub fn weekday(index: usize) -> &'static str {
    t(["周一", "周二", "周三", "周四", "周五", "周六", "周日"][index.min(6)])
}
