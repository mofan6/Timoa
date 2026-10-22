//! Versioned, validated persistence. The application owns a separate profile.
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, io::Write, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};

// Stable IDs keep older backups and custom audio references unchanged.
// The last two positive IDs are reserved and never allocated to custom sounds.
pub const BUILTIN_SOUNDS: [(i32, &str); 9] = [
    (0, "静音"), (1, "轻柔铃声"), (2, "马林巴琴"), (3, "钟声"),
    (4, "钢琴旋律"), (5, "电钢琴旋律"), (6, "铃声旋律"),
    (i32::MAX - 1, "花火大会"), (i32::MAX, "黄昏之时"),
];
pub fn builtin_sound_index(id: i32) -> Option<usize> {
    BUILTIN_SOUNDS.iter().position(|(sound, _)| *sound == id)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub default_sound: i32,
    pub custom_name: String,
    pub startup: bool,
    pub width: f32,
    pub height: f32,
    pub selected_tab: i32,
    pub language: i32,
    pub theme: i32,
    pub color_theme: i32,
    pub custom_sounds: Vec<CustomSound>,
    pub single_focus: bool,
    pub stopwatch_enabled: bool,
    pub focus_timer_id: String,
    pub focus_duration_seconds: u64,
    pub active_focus_mode: i32,
    pub quick_snooze_seconds: Vec<u64>,
    pub auto_cycle: bool,
    pub cycle_interval_seconds: u64,
    pub hourly_chime: bool,
    pub half_hour_chime: bool,
}
impl Default for Settings {
    fn default() -> Self { Self { default_sound: 0, custom_name: String::new(), startup: false,
        width: 480., height: 760., selected_tab: 0, language: 0, theme: 0, color_theme: 0, custom_sounds: vec![],
        single_focus: false, stopwatch_enabled: false, focus_timer_id: String::new(), focus_duration_seconds: 1500, active_focus_mode: 0,
        quick_snooze_seconds: vec![300, 600, 900], auto_cycle: false, cycle_interval_seconds: 3,
        hourly_chime: false, half_hour_chime: false } }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CustomSound { pub id: i32, pub name: String }

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Alarm {
    pub id: String,
    pub label: String,
    pub hour: u32,
    pub minute: u32,
    pub days: u8,
    pub end_minute: Option<u32>,
    pub fire_at: i64,
    pub sound: i32,
    pub warn_before: bool,
    pub enabled: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Countdown {
    pub focus_slots: Vec<crate::stats::FocusSlot>,
    pub id: String,
    pub label: String,
    pub original_ms: u64,
    pub remaining_ms: u64,
    pub ends_at_ms: i64,
    pub paused: bool,
    pub finished: bool,
    pub sound: i32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Stopwatch {
    pub focus_slots: Vec<crate::stats::FocusSlot>,
    pub elapsed_ms: u64,
    pub running: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Document {
    pub countdown_names: Vec<String>,
    pub alarm_names: Vec<String>,
    pub name_history_initialized: bool,
    pub focus_days: Vec<crate::stats::FocusDay>,
    pub format: String,
    pub settings: Settings,
    pub alarms: Vec<Alarm>,
    pub countdowns: Vec<Countdown>,
    pub total_focus_ms: u64,
    pub focus_stats_initialized: bool,
    pub stopwatch: Stopwatch,
}
impl Default for Document {
    fn default() -> Self { Self { format: "TIMO.Native.v1".into(), settings: Settings::default(),
        alarms: vec![], countdowns: vec![], total_focus_ms: 0, focus_stats_initialized: false, stopwatch: Stopwatch::default(), focus_days: vec![],
        countdown_names: vec![], alarm_names: vec![], name_history_initialized: false } }
}

#[derive(Clone)]
pub struct Folders { pub roaming: PathBuf, pub local: PathBuf }
impl Folders {
    pub fn system() -> Self {
        let profile = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
        Self {
            roaming: std::env::var_os("APPDATA").map(PathBuf::from)
                .unwrap_or_else(|| profile.join("AppData/Roaming")).join("TIMO"),
            local: std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
                .unwrap_or_else(|| profile.join("AppData/Local")).join("TIMO"),
        }
    }
    pub fn data(&self) -> PathBuf { self.roaming.join("data.json") }
    pub fn custom(&self) -> PathBuf { self.roaming.join("sounds/custom.wav") }
    pub fn custom_sound(&self, id: i32) -> PathBuf {
        if id == 7 { self.custom() } else { self.roaming.join(format!("sounds/custom-{id}.wav")) }
    }
    pub fn log(&self, message: &str) {
        let _ = fs::create_dir_all(&self.roaming);
        let path = self.roaming.join("native.log");
        if fs::metadata(&path).is_ok_and(|m| m.len() > 256 * 1024) {
            let _ = fs::remove_file(self.roaming.join("native.log.old"));
            let _ = fs::rename(&path, self.roaming.join("native.log.old"));
        }
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{}  {}", Local::now().format("%Y-%m-%d %H:%M:%S"), message);
        }
    }
}

pub fn text(s: &str) -> String { s.trim().chars().take(200).collect() }
pub fn remember_name(history: &mut Vec<String>, name: &str) {
    let name = text(name);
    if name.is_empty() { return; }
    history.retain(|item| item != &name);
    history.insert(0, name);
    history.truncate(100);
}
pub fn initialize_name_history(doc: &mut Document) {
    if !doc.name_history_initialized {
        for timer in &doc.countdowns { remember_name(&mut doc.countdown_names, &timer.label); }
        for alarm in &doc.alarms { remember_name(&mut doc.alarm_names, &alarm.label); }
        doc.name_history_initialized = true;
    }
    for history in [&mut doc.countdown_names, &mut doc.alarm_names] {
        let mut seen = std::collections::HashSet::new();
        *history = history.iter().map(|name| text(name)).filter(|name| !name.is_empty() && seen.insert(name.clone())).take(100).collect();
    }
}
pub fn sound(v: i32) -> i32 { if v >= 0 { v } else { 0 } }
pub fn now_ms() -> i64 { Utc::now().timestamp_millis() }
pub fn new_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("{:x}-{:x}", Utc::now().timestamp_nanos_opt().unwrap_or(now_ms()), COUNTER.fetch_add(1, Ordering::Relaxed))
}
pub fn local_time(date: NaiveDate, hour: u32, minute: u32) -> Option<i64> {
    let naive = date.and_hms_opt(hour, minute, 0)?;
    Local.from_local_datetime(&naive).earliest().map(|v| v.timestamp())
}
pub fn next_occurrence(after: i64, hour: u32, minute: u32, days: u8) -> Option<i64> {
    let date = Local.timestamp_opt(after, 0).single()?.date_naive();
    (0..=8).filter_map(|offset| {
        let candidate_date = date.checked_add_signed(chrono::Duration::days(offset))?;
        if days & (1 << candidate_date.weekday().num_days_from_monday()) == 0 { return None; }
        local_time(candidate_date, hour, minute).filter(|t| *t > after)
    }).next()
}
pub fn previous_occurrence(before: i64, hour: u32, minute: u32, days: u8) -> Option<i64> {
    let date = Local.timestamp_opt(before, 0).single()?.date_naive();
    (0..=8).filter_map(|offset| {
        let candidate_date = date.checked_sub_signed(chrono::Duration::days(offset))?;
        if days & (1 << candidate_date.weekday().num_days_from_monday()) == 0 { return None; }
        local_time(candidate_date, hour, minute).filter(|t| *t <= before)
    }).next()
}

pub fn sanitize(mut doc: Document) -> Document {
    doc.format = "TIMO.Native.v1".into();
    doc.settings.default_sound = sound(doc.settings.default_sound);
    doc.settings.custom_name = text(&doc.settings.custom_name);
    if !doc.settings.width.is_finite() { doc.settings.width = 480.; }
    if !doc.settings.height.is_finite() { doc.settings.height = 760.; }
    doc.settings.width = doc.settings.width.clamp(420., 1600.);
    doc.settings.height = doc.settings.height.clamp(620., 1800.);
    doc.settings.selected_tab = doc.settings.selected_tab.clamp(0, 4);
    if doc.settings.selected_tab == 4 && !doc.settings.stopwatch_enabled { doc.settings.selected_tab = 0; }
    doc.settings.focus_duration_seconds = doc.settings.focus_duration_seconds.clamp(1, 359999);
    doc.settings.active_focus_mode = if doc.settings.stopwatch_enabled { doc.settings.active_focus_mode.clamp(0, 1) } else { 0 };
    doc.settings.focus_timer_id = text(&doc.settings.focus_timer_id);
    doc.focus_days = crate::stats::sanitize_days(doc.focus_days);
    crate::stats::sanitize_slots(&mut doc.stopwatch.focus_slots);
    doc.settings.quick_snooze_seconds.truncate(3);
    while doc.settings.quick_snooze_seconds.len() < 3 {
        doc.settings.quick_snooze_seconds.push([300, 600, 900][doc.settings.quick_snooze_seconds.len()]);
    }
    for seconds in &mut doc.settings.quick_snooze_seconds { *seconds = (*seconds).clamp(1, 5999); }
    doc.settings.cycle_interval_seconds = doc.settings.cycle_interval_seconds.min(3600);
    doc.stopwatch.elapsed_ms = doc.stopwatch.elapsed_ms.min(359_999_000);
    doc.settings.theme = doc.settings.theme.clamp(0, 1);
    doc.settings.color_theme = doc.settings.color_theme.clamp(0, 5);
    doc.settings.language = doc.settings.language.clamp(0, 2);
    let mut sound_ids = std::collections::HashSet::new();
    doc.settings.custom_sounds.retain(|s| s.id >= 7 && builtin_sound_index(s.id).is_none() && sound_ids.insert(s.id));
    doc.settings.custom_sounds.truncate(1000);
    for item in &mut doc.settings.custom_sounds { item.name = text(&item.name); }
    let mut ids = std::collections::HashSet::new();
    doc.alarms.retain(|a| a.hour < 24 && a.minute < 60 && a.days <= 127 && a.fire_at > 0
        && Local.timestamp_opt(a.fire_at, 0).single().is_some() && (a.id.is_empty() || ids.insert(a.id.clone())));
    doc.alarms.truncate(2000);
    for a in &mut doc.alarms {
        if a.id.is_empty() { a.id = new_id(); }
        a.label = text(&a.label);
        a.sound = sound(a.sound);
        if a.end_minute.is_some_and(|e| e >= 1440 || e <= a.hour * 60 + a.minute) { a.end_minute = None; }
    }
    let mut ids = std::collections::HashSet::new();
    doc.countdowns.retain(|t| t.original_ms > 0 && t.original_ms <= 360_000_000 && (t.id.is_empty() || ids.insert(t.id.clone())));
    doc.countdowns.truncate(100);
    for t in &mut doc.countdowns {
        crate::stats::sanitize_slots(&mut t.focus_slots);
        if t.id.is_empty() { t.id = new_id(); }
        t.label = text(&t.label); t.sound = sound(t.sound); t.remaining_ms = t.remaining_ms.min(t.original_ms);
    }
    if !doc.countdowns.iter().any(|timer| timer.id == doc.settings.focus_timer_id) { doc.settings.focus_timer_id.clear(); }
    initialize_name_history(&mut doc);
    doc
}

fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value.as_object()?.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v)
}
fn number(value: &Value, name: &str, fallback: i64) -> i64 { field(value, name).and_then(Value::as_i64).unwrap_or(fallback) }
fn flag(value: &Value, name: &str, fallback: bool) -> bool { field(value, name).and_then(Value::as_bool).unwrap_or(fallback) }
fn label(value: &Value, name: &str) -> String { field(value, name).and_then(Value::as_str).map(text).unwrap_or_default() }
fn legacy_timestamp(value: &Value, name: &str) -> Option<i64> {
    let date = field(value, name)?.as_str()?;
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date) { return Some(dt.timestamp()); }
    let naive = NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%.f").ok()?;
    Local.from_local_datetime(&naive).earliest().map(|d| d.timestamp())
}

pub fn parse(bytes: &[u8]) -> Result<Document, String> {
    if bytes.len() > 5 * 1024 * 1024 { return Err("备份文件超过 5 MB，无法读取。".into()); }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let value: Value = serde_json::from_slice(bytes).map_err(|_| "这不是有效的 Timoa 备份文件。")?;
    if let Some(format) = field(&value, "format") {
        if format.as_str() != Some("TIMO.Native.v1") { return Err("无法识别这个文件，请选择 Timoa 导出的 JSON 备份。".into()); }
        if !value.get("settings").is_some_and(Value::is_object)
            || !value.get("alarms").is_some_and(Value::is_array)
            || !value.get("countdowns").is_some_and(Value::is_array) {
            return Err("备份数据不完整。".into());
        }
        return serde_json::from_value(value).map(sanitize).map_err(|_| "备份数据不完整。".into());
    }
    let legacy_settings = field(&value, "Settings").unwrap_or(&value);
    if field(&value, "Alarms").is_none() && field(&value, "RecurringAlarms").is_none()
        && field(legacy_settings, "DefaultSound").is_none() && field(legacy_settings, "LaunchAtStartup").is_none() {
        return Err("无法识别这个文件，请选择 Timoa 导出的 JSON 备份。".into());
    }
    let old_settings = field(&value, "Settings").unwrap_or(&value);
    let mut doc = Document::default();
    doc.settings.default_sound = sound(number(old_settings, "DefaultSound", 0) as i32);
    doc.settings.custom_name = label(old_settings, "CustomSoundName");
    // Startup belongs to the executable that registered it; migration does not hijack that entry.
    doc.settings.startup = false;
    doc.settings.selected_tab = number(old_settings, "SelectedTab", 0).clamp(0, 2) as i32;
    if let Some(list) = field(&value, "Alarms").and_then(Value::as_array) {
        for old in list.iter().take(2000) {
            if let Some(fire_at) = legacy_timestamp(old, "FireAt") {
                let Some(dt) = Local.timestamp_opt(fire_at, 0).single() else { continue; };
                doc.alarms.push(Alarm { id: label(old, "Id"), label: label(old, "Label"),
                    hour: dt.hour(), minute: dt.minute(), days: 0, end_minute: None, fire_at,
                    sound: sound(number(old, "Sound", 0) as i32), warn_before: flag(old, "WarnBefore", false),
                    enabled: flag(old, "Enabled", true) });
            }
        }
    }
    if let Some(list) = field(&value, "RecurringAlarms").and_then(Value::as_array) {
        for old in list.iter().take(2000) {
            let hour = number(old, "Hour", -1);
            let minute = number(old, "Minute", -1);
            let days = number(old, "Days", 0) as u8 & 127;
            if !(0..24).contains(&hour) || !(0..60).contains(&minute) || days == 0 { continue; }
            if let Some(fire_at) = legacy_timestamp(old, "NextFireAt") {
                doc.alarms.push(Alarm { id: label(old, "Id"), label: label(old, "Label"),
                    hour: hour as u32, minute: minute as u32, days,
                    end_minute: field(old, "EndMinute").and_then(Value::as_u64).map(|v| v as u32),
                    fire_at, sound: sound(number(old, "Sound", 0) as i32),
                    warn_before: flag(old, "WarnBefore", false), enabled: flag(old, "Enabled", true) });
            }
        }
    }
    Ok(sanitize(doc))
}

pub fn load(folders: &Folders) -> (Document, String) {
    if let Ok(bytes) = fs::read(folders.data()) {
        match parse(&bytes) {
            Ok(doc) => return (doc, String::new()),
            Err(error) => {
                let _ = fs::copy(folders.data(), folders.roaming.join("data.json.corrupt"));
                folders.log(&error);
                return (Document::default(), "本地数据无法读取，已保留损坏文件副本。".into());
            }
        }
    }
    if let Some(parent) = folders.roaming.parent() {
        let legacy = parent.join("Tidsro");
        if let Ok(bytes) = fs::read(legacy.join("data.json")) {
            if let Ok(doc) = parse(&bytes) {
                let _ = fs::create_dir_all(folders.roaming.join("sounds"));
                if legacy.join("sounds/custom.wav").is_file() {
                    let _ = fs::copy(legacy.join("sounds/custom.wav"), folders.custom());
                }
                let message = format!("已从上一版复制设置与 {} 项闹钟。", doc.alarms.len());
                return (doc, message);
            }
        }
    }
    (Document::default(), String::new())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("没有可写入的文件夹。")?;
    fs::create_dir_all(parent).map_err(|e| format!("无法创建保存目录：{e}"))?;
    let staging = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = fs::File::create(&staging).map_err(|e| e.to_string())?;
        file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|e| e.to_string())?;
        drop(file);
        replace(&staging, path)
    })();
    if result.is_err() { let _ = fs::remove_file(staging); }
    result
}

pub fn replace(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let ok = unsafe { windows_sys::Win32::Storage::FileSystem::MoveFileExW(source.as_ptr(), target.as_ptr(), 0x1 | 0x8) };
    if ok == 0 { Err(format!("无法保存文件：{}", std::io::Error::last_os_error())) } else { Ok(()) }
}

pub fn save(folders: &Folders, doc: &Document) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?;
    atomic_write(&folders.data(), &bytes)
}

pub fn format_duration(ms: u64) -> String {
    let seconds = ms.div_ceil(1000);
    if seconds >= 3600 { format!("{:02}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60) }
    else { format!("{:02}:{:02}", seconds / 60, seconds % 60) }
}
pub fn days_label(days: u8) -> String {
    use crate::i18n::t;
    match days {
        0 => t("仅一次").into(), 127 => t("每天").into(), 31 => t("工作日").into(), 96 => t("周末").into(),
        _ => ["周一", "周二", "周三", "周四", "周五", "周六", "周日"].iter().enumerate()
            .filter(|(i, _)| days & (1 << i) != 0).map(|(_, s)| t(s)).collect::<Vec<_>>().join(" "),
    }
}
