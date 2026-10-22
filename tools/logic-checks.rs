//! Non-UI checks using the application's real persistence/scheduling modules.
#![allow(dead_code)]
#[path = "../src/data.rs"] mod data;
#[path = "../src/stats.rs"] mod stats;
#[path = "../src/engine.rs"] mod engine;
mod i18n { pub fn t(source: &str) -> &str { source } }

#[test]
fn reports_and_raw_names_roundtrip() {
    let mut document = data::Document::default();
    document.settings.hourly_chime = true;
    document.settings.half_hour_chime = true;
    data::remember_name(&mut document.countdown_names, "  数学真题 · Chapter 2  ");
    data::remember_name(&mut document.alarm_names, "喝水 Drink water");
    document.name_history_initialized = true;
    let bytes = serde_json::to_vec(&document).unwrap();
    let restored = data::parse(&bytes).unwrap();
    assert!(restored.settings.hourly_chime && restored.settings.half_hour_chime);
    assert_eq!(restored.countdown_names, ["数学真题 · Chapter 2"]);
    assert_eq!(restored.alarm_names, ["喝水 Drink water"]);
}

#[test]
fn old_profiles_default_reports_off_and_migrate_names_once() {
    let mut document = data::parse(br#"{"format":"TIMO.Native.v1","settings":{},"alarms":[{"label":"Alarm","hour":8,"minute":0,"fireAt":2000000000}],"countdowns":[{"label":"Focus","originalMs":60000,"remainingMs":60000,"paused":true}]}"#).unwrap();
    assert!(!document.settings.hourly_chime && !document.settings.half_hour_chime);
    assert_eq!(document.countdown_names, ["Focus"]);
    assert_eq!(document.alarm_names, ["Alarm"]);
    document.countdown_names.clear(); document.alarm_names.clear();
    let reopened = data::parse(&serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(reopened.countdown_names.is_empty() && reopened.alarm_names.is_empty());
}

#[test]
fn history_deduplicates_sanitizes_caps_and_preserves_recent_order() {
    let mut names = Vec::new();
    for index in 0..105 { data::remember_name(&mut names, &format!("name {index}")); }
    assert_eq!(names.len(), 100); assert_eq!(names[0], "name 104");
    data::remember_name(&mut names, "  name 80  ");
    assert_eq!(names[0], "name 80");
    assert_eq!(names.iter().filter(|name| name.as_str() == "name 80").count(), 1);
    data::remember_name(&mut names, "  "); assert_eq!(names.len(), 100);
    data::remember_name(&mut names, &"字".repeat(300));
    assert_eq!(names[0].chars().count(), 200);
}

#[test]
fn successful_creation_records_names_and_rejected_creation_does_not() {
    let mut engine = engine::Engine::new(data::Document::default(), String::new());
    assert!(engine.start(0, "invalid", 0).is_err());
    assert!(engine.doc.countdown_names.is_empty());
    engine.start(60, "Focus", 0).unwrap();
    engine.start(60, "Focus", 0).unwrap();
    assert_eq!(engine.doc.countdown_names, ["Focus"]);
    engine.put_alarm(data::Alarm { label: "Alarm".into(), ..Default::default() });
    assert_eq!(engine.doc.alarm_names, ["Alarm"]);
}
