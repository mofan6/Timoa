#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio;
mod data;
mod engine;
mod native;
mod i18n;
mod shortcuts;
mod stats;

use chrono::{Datelike, Local, NaiveDate, TimeZone, Timelike};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::{cell::RefCell, collections::VecDeque, fs, rc::Rc, sync::{atomic::Ordering, mpsc}, time::{Duration, Instant}};
use data::{Alarm, Document, Folders};
use engine::{Alert, Engine};

slint::include_modules!();
type Controller = Rc<RefCell<App>>;

enum Confirmation { Import(Document), Clear, Reset }
struct Editor { id: Option<String>, days: u8 }
struct App {
    ui: slint::Weak<MainWindow>,
    mini: MiniWindow,
    compact: bool,
    mini_positioned: bool,
    engine: Engine,
    folders: Folders,
    player: audio::Player,
    import: Option<audio::ImportJob>,
    timer_rows: Rc<VecModel<TimerRow>>,
    alarm_rows: Rc<VecModel<AlarmRow>>,
    day_rows: Rc<VecModel<DayRow>>,
    week_rows: Rc<VecModel<AlarmRow>>,
    focus_bars: Rc<VecModel<FocusBar>>,
    countdown_names: Rc<VecModel<SharedString>>,
    alarm_names: Rc<VecModel<SharedString>>,
    week_offset: i64,
    week_day: i32,
    editor: Option<Editor>,
    confirm: Option<(ConfirmWindow, Confirmation)>,
    about: Option<AboutWindow>,
    alerts: VecDeque<Alert>,
    alert: Option<(AlertWindow, Alert)>,
    clock_alert: Option<(AlertWindow, Alert)>,
    pending_clock: Option<Alert>,
    sound_owner: Option<String>,
    quit_pending: bool,
    import_notice: String,
    preview_index: i32,
    preview_deadline: Option<Instant>,
    message_seen: String,
    message_deadline: Option<Instant>,
}

fn clock_duration(ms: u64, round_up: bool) -> String {
    let seconds = if round_up { ms.div_ceil(1000) } else { ms / 1000 };
    format!("{:02}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

fn delay_label(seconds: u64) -> String {
    if seconds.is_multiple_of(60) { format!("{} {}", seconds / 60, i18n::t("分钟")) }
    else if seconds < 60 { format!("{} {}", seconds, i18n::t("秒")) }
    else { format!("{:02}:{:02}", seconds / 60, seconds % 60) }
}
fn finish_label(engine: &Engine, timer: &data::Countdown) -> String {
    if let Some(wait) = engine.cycle_remaining(&timer.id) {
        return format!("{} · {}", i18n::t("下一轮"), data::format_duration(wait));
    }
    if timer.finished { return i18n::t("已完成").into(); }
    if timer.paused { return i18n::t("已暂停").into(); }
    let end = Local::now() + chrono::Duration::milliseconds(engine.remaining(timer).min(360_000_000) as i64);
    let clock = if end.date_naive() == Local::now().date_naive() { end.format("%H:%M").to_string() }
        else { format!("{} {}", i18n::date(end.date_naive()), end.format("%H:%M")) };
    if i18n::language() == 2 { format!("{} {}", i18n::t("结束"), clock) } else { format!("{} {}", clock, i18n::t("结束")) }
}

fn sync_model<T: Clone + PartialEq + 'static>(model: &VecModel<T>, rows: Vec<T>) {
    if model.row_count() != rows.len() { model.set_vec(rows); return; }
    for (index, row) in rows.into_iter().enumerate() {
        if model.row_data(index).as_ref() != Some(&row) { model.set_row_data(index, row); }
    }
}
fn alarm_row(alarm: &Alarm) -> AlarmRow {
    let date = Local.timestamp_opt(alarm.fire_at, 0).single();
    let mut detail = if alarm.days == 0 {
        date.map(|v| i18n::date(v.date_naive())).unwrap_or_default()
    } else { data::days_label(alarm.days) };
    if let Some(end) = alarm.end_minute { detail += &i18n::message(&format!(" · 至 {:02}:{:02}", end / 60, end % 60)); }
    if alarm.warn_before { detail += i18n::t(" · 提前提醒"); }
    AlarmRow { id: alarm.id.clone().into(), label: if alarm.label.is_empty() { i18n::t("未命名闹钟").into() } else { alarm.label.clone().into() },
        time: format!("{:02}:{:02}", alarm.hour, alarm.minute).into(), detail: detail.into(),
        meta: if alarm.days == 0 { i18n::t("仅一次").into() } else { i18n::t("重复").into() }, enabled: alarm.enabled }
}
impl App {
    fn relocalize(&mut self) {
        i18n::select(self.engine.doc.settings.language);
        if let Some(ui) = self.ui.upgrade() {
            self.refresh_sound_library();
            ui.set_footer_text(i18n::message(ui.get_footer_text().as_str()).into());
        }
        if let Some(ui) = self.ui.upgrade() {
            ui.set_editor_error_message(i18n::message(ui.get_editor_error_message().as_str()).into());
        }
        self.apply_theme();
        if let Some((ui, _)) = &self.confirm {
            ui.set_heading(i18n::message(ui.get_heading().as_str()).into());
            ui.set_message(i18n::message(ui.get_message().as_str()).into());
            ui.set_action_label(i18n::message(ui.get_action_label().as_str()).into());
        }
        if let Some((ui, alert)) = &self.alert {
            ui.set_heading(i18n::t(&alert.title).into());
            if alert.default_label { ui.set_label(i18n::t(&alert.label).into()); }
        }
        if let Some((ui, alert)) = &self.clock_alert { ui.set_heading(i18n::t(&alert.title).into()); }
    }
    fn apply_theme(&self) {
        let dark = self.engine.doc.settings.theme == 0;
        let variant = self.engine.doc.settings.color_theme;
        if let Some(ui) = self.ui.upgrade() {
            native::configure_chrome(ui.window(), false);
            ui.global::<Theme>().set_dark(dark); ui.global::<Theme>().set_variant(variant);
            ui.set_theme_index(self.engine.doc.settings.theme); ui.set_color_theme(variant);
            apply_native_theme(&ui, None);
            ui.window().request_redraw();
        }
        self.mini.global::<Theme>().set_dark(dark); self.mini.global::<Theme>().set_variant(variant);
        native::configure_chrome(self.mini.window(), true);
        apply_native_theme(&self.mini, None);
        self.mini.window().request_redraw();
        if let Some((ui, _)) = &self.confirm { ui.global::<Theme>().set_dark(dark); ui.global::<Theme>().set_variant(variant); apply_native_theme(ui, None); }
        if let Some((ui, _)) = &self.alert { ui.global::<Theme>().set_dark(dark); ui.global::<Theme>().set_variant(variant); apply_native_theme(ui, None); }
        if let Some((ui, _)) = &self.clock_alert { ui.global::<Theme>().set_dark(dark); ui.global::<Theme>().set_variant(variant); apply_native_theme(ui, None); }
        if let Some(ui) = &self.about { ui.global::<Theme>().set_dark(dark); ui.global::<Theme>().set_variant(variant); apply_native_theme(ui, None); }
    }
    fn set_compact(&mut self, compact: bool) {
        self.commit(); self.compact = compact; self.refresh();
        if let Some(ui) = self.ui.upgrade() {
            if compact {
                native::configure_chrome(self.mini.window(), true);
                if !self.mini_positioned {
                    native::place_mini(self.mini.window(), ui.window()); self.mini_positioned = true;
                }
                let _ = self.mini.show(); let _ = ui.hide();
                apply_native_theme(&self.mini, None);
            } else {
                native::show(ui.window()); let _ = self.mini.hide();
            }
        }
    }
    fn show_current_window(&self) {
        if self.compact { native::show(self.mini.window()); }
        else if let Some(ui) = self.ui.upgrade() { native::show(ui.window()); }
        self.apply_theme();
    }
    fn hide_current_window(&mut self) {
        self.commit();
        let _ = self.mini.hide();
        if let Some(ui) = self.ui.upgrade() { let _ = ui.hide(); }
    }
    fn select_focus_timer(&mut self) {
        let timer = self.engine.focus_timer()
            .or_else(|| self.engine.doc.countdowns.iter().rev().find(|timer| !timer.finished && !timer.paused))
            .or_else(|| self.engine.doc.countdowns.iter().rev().find(|timer| !timer.finished)).cloned();
        if let Some(timer) = timer {
            self.engine.doc.settings.focus_timer_id = timer.id.clone();
            self.engine.doc.settings.focus_duration_seconds = timer.original_ms / 1000;
            if let Some(ui) = self.ui.upgrade() {
                ui.set_draft_label(timer.label.clone().into()); ui.set_draft_sound(self.sound_index(timer.sound));
            }
        }
        self.restore_focus_draft();
    }
    fn start_countdown(&mut self) {
        if self.engine.doc.settings.single_focus && self.engine.focus_timer().is_some() { return; }
        if let Some(ui) = self.ui.upgrade() {
            let duration = ui.get_draft_hour().max(0) as u64 * 3600 + ui.get_draft_minute().max(0) as u64 * 60 + ui.get_draft_second().max(0) as u64;
            let selected_sound = self.sound_choice(ui.get_draft_sound());
            if !self.sound_exists(selected_sound) { self.engine.message = i18n::t("请先在设置中导入自定义提示音。").into(); }
            else { match self.engine.start(duration, ui.get_draft_label().as_str(), selected_sound) {
                Ok(()) => { self.engine.message.clear(); self.commit(); }
                Err(error) => self.engine.message = error,
            } }
            self.refresh();
        }
    }
    fn restore_focus_draft(&self) {
        if let Some(ui) = self.ui.upgrade() {
            let seconds = self.engine.doc.settings.focus_duration_seconds;
            ui.set_draft_hour((seconds / 3600) as i32); ui.set_draft_minute((seconds / 60 % 60) as i32); ui.set_draft_second((seconds % 60) as i32);
        }
    }
    fn stop_preview(&mut self) {
        if self.preview_index >= 0 { self.player.stop(); }
        self.preview_index = -1; self.preview_deadline = None;
        if let Some(ui) = self.ui.upgrade() { ui.set_preview_index(-1); }
    }
    fn preview(&mut self, choice: i32) {
        let index = self.sound_choice(choice);
        if self.alert.is_some() || self.clock_alert.is_some() || index <= 0 || !self.sound_exists(index) { return; }
        if self.preview_index == index { self.stop_preview(); return; }
        self.stop_preview();
        let folders = self.folders.clone();
        match self.player.play(index, &folders) {
            Ok(()) => {
                self.preview_index = index;
                let duration = self.player.duration();
                let preview_duration = if data::builtin_sound_index(index).is_some_and(|slot| slot >= 7) {
                    duration // Play the new short music tracks through their fade-out.
                } else { duration.min(Duration::from_secs(5)) };
                self.preview_deadline = Some(Instant::now() + preview_duration);
                if let Some(ui) = self.ui.upgrade() { ui.set_preview_index(self.sound_index(index)); }
            }
            Err(error) => { self.engine.message = error; self.refresh(); }
        }
    }
    fn commit(&mut self) {
        if let Some(ui) = self.ui.upgrade() {
            let scale = ui.window().scale_factor();
            let size = ui.window().size();
            if size.width > 0 && size.height > 0 && !native::maximized(ui.window()) {
                self.engine.doc.settings.width = (size.width as f32 / scale).clamp(420., 1600.);
                self.engine.doc.settings.height = (size.height as f32 / scale).clamp(620., 1800.);
            }
            self.engine.doc.settings.selected_tab = ui.get_page();
        }
        if let Err(error) = data::save(&self.folders, &self.engine.snapshot()) {
            self.folders.log(&error);
            self.engine.message = i18n::t("未能保存数据，请检查磁盘空间与文件夹权限。").into();
        }
    }
    fn sound_ids(&self) -> Vec<i32> { data::BUILTIN_SOUNDS.iter().map(|(id, _)| *id)
        .chain(self.engine.doc.settings.custom_sounds.iter().map(|s| s.id)).collect() }
    fn sound_choice(&self, index: i32) -> i32 { self.sound_ids().get(index.max(0) as usize).copied().unwrap_or(0) }
    fn sound_index(&self, id: i32) -> i32 { self.sound_ids().iter().position(|v| *v == id).unwrap_or(0) as i32 }
    fn sound_exists(&self, id: i32) -> bool { data::builtin_sound_index(id).is_some()
        || self.engine.doc.settings.custom_sounds.iter().any(|s| s.id == id) && self.folders.custom_sound(id).is_file() }
    fn sounds(&self) -> ModelRc<SharedString> {
        let mut sounds: Vec<SharedString> = data::BUILTIN_SOUNDS.iter().map(|(_, name)| i18n::t(name).into()).collect();
        sounds.extend(self.engine.doc.settings.custom_sounds.iter().map(|s| s.name.clone().into()));
        ModelRc::new(VecModel::from(sounds))
    }
    fn refresh_sound_library(&self) {
        let Some(ui) = self.ui.upgrade() else { return; };
        ui.set_sounds(self.sounds());
        ui.set_sound_available(ModelRc::new(VecModel::from(self.sound_ids().iter().map(|id| self.sound_exists(*id)).collect::<Vec<_>>())));
        ui.set_custom_sounds(ModelRc::new(VecModel::from(self.engine.doc.settings.custom_sounds.iter().enumerate().map(|(i,s)| CustomSoundRow {
            id: s.id, name: s.name.clone().into(), available: self.sound_exists(s.id), menu_index: (i + data::BUILTIN_SOUNDS.len()) as i32,
        }).collect::<Vec<_>>())));
    }
    fn refresh(&mut self) {
        let Some(ui) = self.ui.upgrade() else { return; };
        let now = Local::now();
        ui.set_clock_text(now.format("%H:%M").to_string().into());
        let weekday = i18n::weekday(now.weekday().num_days_from_monday() as usize);
        ui.set_date_text(format!("{} · {}", i18n::date(now.date_naive()), weekday).into());
        if self.engine.message.is_empty() { self.message_seen.clear(); self.message_deadline = None; }
        else if self.message_seen != self.engine.message || self.message_deadline.is_none() {
            self.message_seen = self.engine.message.clone(); self.message_deadline = Some(Instant::now() + Duration::from_secs(10));
        }
        ui.set_message(i18n::message(&self.engine.message).into());
        ui.set_window_maximized(native::maximized(ui.window()));
        ui.set_single_focus(self.engine.doc.settings.single_focus);
        ui.set_stopwatch_enabled(self.engine.doc.settings.stopwatch_enabled);
        ui.set_auto_cycle(self.engine.doc.settings.auto_cycle);
        ui.set_cycle_interval(self.engine.doc.settings.cycle_interval_seconds as i32);
        ui.set_quick_snooze_1(self.engine.doc.settings.quick_snooze_seconds[0] as i32);
        ui.set_quick_snooze_2(self.engine.doc.settings.quick_snooze_seconds[1] as i32);
        ui.set_quick_snooze_3(self.engine.doc.settings.quick_snooze_seconds[2] as i32);
        self.refresh_alert_options();
        let stopwatch_ms = self.engine.stopwatch_elapsed();
        ui.set_stopwatch_time(clock_duration(stopwatch_ms, false).into());
        ui.set_stopwatch_running(self.engine.doc.stopwatch.running);
        ui.set_stopwatch_has_time(stopwatch_ms > 0 || self.engine.doc.stopwatch.running);
        let timer = self.engine.focus_timer();
        ui.set_focus_active(timer.is_some());
        if let Some(timer) = timer {
            let remaining = self.engine.remaining(timer);
            ui.set_focus_id(timer.id.clone().into()); ui.set_focus_time(clock_duration(remaining, true).into());
            ui.set_focus_end_label(finish_label(&self.engine, timer).into());
            ui.set_focus_waiting(self.engine.cycle_remaining(&timer.id).is_some());
            ui.set_focus_status(i18n::t(if timer.finished { "已完成" } else if timer.paused { "已暂停" } else { "进行中" }).into());
            ui.set_focus_progress((1. - remaining as f64 / timer.original_ms.max(1) as f64).clamp(0., 1.) as f32);
            ui.set_focus_paused(timer.paused); ui.set_focus_finished(timer.finished);
        } else {
            ui.set_focus_waiting(false); ui.set_focus_end_label("".into());
            ui.set_focus_id("".into()); ui.set_focus_progress(0.); ui.set_focus_paused(false); ui.set_focus_finished(false);
        }
        let mini_timer = timer.or_else(|| if self.engine.doc.settings.single_focus { None } else {
            self.engine.doc.countdowns.iter().rev().find(|timer| !timer.finished && !timer.paused)
                .or_else(|| self.engine.doc.countdowns.iter().rev().find(|timer| !timer.finished))
        });
        let mini_stopwatch = self.engine.mini_uses_stopwatch();
        self.mini.set_is_stopwatch(mini_stopwatch);
        if mini_stopwatch {
            self.mini.set_focus_time(clock_duration(stopwatch_ms, false).into());
            self.mini.set_focus_label(i18n::t("正计时").into()); self.mini.set_finish_label("".into());
            self.mini.set_focus_status(i18n::t(if self.engine.doc.stopwatch.running { "进行中" } else if stopwatch_ms > 0 { "已暂停" } else { "尚未开始" }).into());
            self.mini.set_progress(0.);
        } else if let Some(timer) = mini_timer {
            let remaining = self.engine.remaining(timer);
            self.mini.set_focus_time(clock_duration(remaining, true).into());
            self.mini.set_finish_label(if !timer.finished && !timer.paused { finish_label(&self.engine, timer).into() } else { "".into() });
            self.mini.set_focus_label(if timer.label.is_empty() { i18n::t("专注计时").into() } else { timer.label.clone().into() });
            self.mini.set_focus_status(i18n::t(if timer.finished { "已完成" } else if timer.paused { "已暂停" } else { "进行中" }).into());
            self.mini.set_progress((1. - remaining as f64 / timer.original_ms.max(1) as f64).clamp(0., 1.) as f32);
        } else {
            self.mini.set_focus_time(clock_duration(self.engine.doc.settings.focus_duration_seconds * 1000, true).into());
            self.mini.set_focus_label(i18n::t("专注计时").into()); self.mini.set_finish_label("".into());
            self.mini.set_focus_status(i18n::t("尚未开始").into()); self.mini.set_progress(0.);
        }
        let upcoming = self.engine.upcoming_alarms(now.timestamp(), 1800);
        self.mini.set_upcoming_count(upcoming.len() as i32);
        let mut upcoming_text = upcoming.iter().take(2).map(|alarm| format!("{:02}:{:02}  {}", alarm.hour, alarm.minute,
            if alarm.label.is_empty() { i18n::t("闹钟提醒") } else { &alarm.label })).collect::<Vec<_>>().join(" · ");
        if upcoming.len() > 2 { upcoming_text.push_str(&i18n::message(&format!(" · 共 {} 项提醒", upcoming.len()))); }
        self.mini.set_upcoming_text(upcoming_text.into());
        ui.set_import_notice(i18n::message(&self.import_notice).into());
        ui.set_language_index(self.engine.doc.settings.language);
        ui.set_hourly_chime(self.engine.doc.settings.hourly_chime);
        ui.set_half_hour_chime(self.engine.doc.settings.half_hour_chime);
        sync_model(&self.countdown_names, self.engine.doc.countdown_names.iter().map(|name| name.as_str().into()).collect());
        sync_model(&self.alarm_names, self.engine.doc.alarm_names.iter().map(|name| name.as_str().into()).collect());
        ui.set_theme_index(self.engine.doc.settings.theme); ui.set_color_theme(self.engine.doc.settings.color_theme);
        let focused = self.engine.doc.total_focus_ms / 1000;
        ui.set_total_focus(format!("{:02}:{:02}:{:02}", focused / 3600, focused / 60 % 60, focused % 60).into());
        ui.set_preview_index(if self.preview_index < 0 { -1 } else { self.sound_index(self.preview_index) });
        let rows = self.engine.doc.countdowns.iter().map(|timer| {
            let remaining = self.engine.remaining(timer);
            TimerRow { id: timer.id.clone().into(), label: if timer.label.is_empty() { i18n::t("专注计时").into() } else { timer.label.clone().into() },
                remaining: data::format_duration(remaining).into(), status: finish_label(&self.engine, timer).into(),
                progress: (1. - remaining as f64 / timer.original_ms.max(1) as f64).clamp(0., 1.) as f32,
                paused: timer.paused, finished: timer.finished, waiting: self.engine.cycle_remaining(&timer.id).is_some() }
        }).collect();
        sync_model(&self.timer_rows, rows);
        let mut alarms = self.engine.doc.alarms.clone();
        alarms.sort_by_key(|a| (!a.enabled, a.fire_at));
        sync_model(&self.alarm_rows, alarms.iter().map(alarm_row).collect());
        if let Some(alarm) = alarms.iter().filter(|a| a.enabled).min_by_key(|a| a.fire_at) {
            ui.set_next_time(format!("{:02}:{:02}", alarm.hour, alarm.minute).into());
            let when = Local.timestamp_opt(alarm.fire_at, 0).single().map(|d| {
                if d.date_naive() == now.date_naive() { i18n::t("今天").into() }
                else if d.date_naive() == now.date_naive() + chrono::Duration::days(1) { i18n::t("明天").into() }
                else { i18n::date(d.date_naive()) }
            }).unwrap_or_default();
            ui.set_next_label(format!("{} · {}", when, if alarm.label.is_empty() { i18n::t("未命名闹钟") } else { &alarm.label }).into());
        } else {
            ui.set_next_time("--:--".into());
            ui.set_next_label(i18n::t("添加一个闹钟，安排下一段时间").into());
        }
        if ui.get_page() == 2 {
            let monday = now.date_naive() - chrono::Duration::days(now.weekday().num_days_from_monday() as i64)
                + chrono::Duration::weeks(self.week_offset);
            let sunday = monday + chrono::Duration::days(6);
            ui.set_week_title(format!("{} — {}", i18n::date(monday), i18n::date(sunday)).into());
            ui.set_week_day(self.week_day);
            let week_items = |day: usize| -> Vec<AlarmRow> {
                let date = monday + chrono::Duration::days(day as i64);
                let mut matches: Vec<Alarm> = self.engine.doc.alarms.iter().filter(|a| {
                    if a.days != 0 { a.days & (1 << day) != 0 }
                    else { Local.timestamp_opt(a.fire_at, 0).single().is_some_and(|d| d.date_naive() == date) }
                }).cloned().collect();
                matches.sort_by_key(|a| (a.hour, a.minute));
                matches.iter().map(|a| {
                    let mut row = alarm_row(a);
                    if let Some(end) = a.end_minute { row.time = format!("{:02}:{:02} — {:02}:{:02}", a.hour, a.minute, end / 60, end % 60).into(); }
                    row
                }).collect()
            };
            let days = (0..7).map(|index| {
                let date = monday + chrono::Duration::days(index as i64);
                let today = date == now.date_naive();
                let count = week_items(index).len();
                DayRow { label: i18n::weekday(index).into(), date: date.day().to_string().into(),
                    count: if today { i18n::t("今天").into() } else if count == 0 { "".into() } else { i18n::message(&format!("{count} 项")).into() }, today }
            }).collect();
            sync_model(&self.day_rows, days);
            sync_model(&self.week_rows, week_items(self.week_day.clamp(0, 6) as usize));
            let selected_date = monday + chrono::Duration::days(self.week_day.clamp(0, 6) as i64);
            let key = selected_date.format("%Y-%m-%d").to_string();
            let day = self.engine.doc.focus_days.iter().find(|day| day.date == key);
            let hours = day.map(|day| day.hours_ms).unwrap_or([0; 24]);
            let total = hours.iter().fold(0u64, |total, value| total.saturating_add(*value));
            let peak = hours.iter().copied().max().unwrap_or(0);
            let peak_hour = hours.iter().position(|value| *value == peak).unwrap_or(0);
            ui.set_stats_date(format!("{} · {}", i18n::date(selected_date), i18n::weekday(self.week_day.clamp(0, 6) as usize)).into());
            ui.set_stats_total(clock_duration(total, false).into());
            ui.set_stats_rounds(format!("{} {}", day.map(|day| day.rounds).unwrap_or(0), i18n::t("轮完成")).into());
            ui.set_stats_has_data(total > 0);
            ui.set_stats_peak(if total == 0 { i18n::t("完成一轮专注后，就会留下记录。").into() }
                else { format!("{}  {:02}:00–{:02}:00", i18n::t("最专注的时段"), peak_hour, peak_hour + 1).into() });
            let bars = (0..24).map(|hour| FocusBar {
                // A fixed one-hour scale makes dates and hours directly comparable.
                amount: (hours[hour] as f64 / 3_600_000.).clamp(0., 1.) as f32,
                over_limit: hours[hour] > 3_600_000,
                detail: format!("{:02}:00–{:02}:00  ·  {}", hour, hour + 1, data::format_duration(hours[hour])).into(),
            }).collect::<Vec<_>>();
            sync_model(&self.focus_bars, bars);
        }
        ui.set_default_sound(self.sound_index(self.engine.doc.settings.default_sound));
        let has_custom = self.folders.custom().is_file();
        ui.set_has_custom(has_custom);
        ui.set_custom_name(if has_custom { if self.engine.doc.settings.custom_name.is_empty() { i18n::t("自定义提示音").into() }
            else { self.engine.doc.settings.custom_name.clone().into() } } else { i18n::t("尚未选择提示音").into() });
        ui.set_startup_enabled(self.engine.doc.settings.startup);
        ui.set_import_busy(self.import.is_some());
        if let Some(job) = &self.import { ui.set_import_progress(job.started.elapsed().as_secs_f32().fract()); }
    }
    fn refresh_alert_options(&self) {
        if let Some((ui, _)) = &self.alert {
            let labels = self.engine.doc.settings.quick_snooze_seconds.iter()
                .map(|seconds| format!("+ {}", delay_label(*seconds)).into()).collect::<Vec<SharedString>>();
            let current = ui.get_quick_labels();
            // Keep pressed/hovered buttons alive across the periodic clock refresh.
            if current.row_count() != labels.len() || labels.iter().enumerate()
                .any(|(index, label)| current.row_data(index).as_ref() != Some(label)) {
                ui.set_quick_labels(ModelRc::new(VecModel::from(labels)));
            }
        }
    }
    fn receive_alerts(&mut self, alerts: Vec<Alert>) {
        // Sound is dispatched when a reminder becomes due, independently of the window queue.
        for alert in alerts {
            if alert.sound > 0 {
                self.stop_preview();
                let folders = self.folders.clone();
                if let Err(error) = self.player.play(alert.sound, &folders) { self.folders.log(&error); }
                self.sound_owner = Some(alert.token.clone());
            }
            if alert.clock_chime { self.pending_clock = Some(alert); }
            else { self.alerts.push_back(alert); }
        }
    }
    fn close_alert(&mut self) {
        if let Some((ui, alert)) = self.alert.take() {
            // Dismissing an older window must not stop the sound of a newer reminder.
            if self.sound_owner.as_deref() == Some(alert.token.as_str()) {
                self.player.stop(); self.sound_owner = None;
            }
            let _ = ui.hide();
        }
    }
    fn clear_alerts(&mut self) {
        self.close_alert(); self.close_clock_alert(); self.pending_clock = None;
        self.alerts.clear(); self.player.stop(); self.sound_owner = None;
    }
    fn close_clock_alert(&mut self) {
        if let Some((ui, alert)) = self.clock_alert.take() {
            if self.sound_owner.as_deref() == Some(alert.token.as_str()) { self.player.stop(); self.sound_owner = None; }
            let _ = ui.hide();
        }
    }
    fn discard_cycle_alerts(&mut self, id: &str) {
        let sounding = self.alerts.iter().chain(self.alert.as_ref().map(|(_, alert)| alert))
            .any(|alert| alert.countdown && alert.id == id && self.sound_owner.as_deref() == Some(alert.token.as_str()));
        if sounding { self.player.stop(); self.sound_owner = None; }
        self.alerts.retain(|alert| !alert.countdown || alert.id != id);
        if self.alert.as_ref().is_some_and(|(_, alert)| alert.countdown && alert.id == id) { self.close_alert(); }
    }
    fn request_quit(&mut self) {
        self.commit();
        if let Some(job) = &self.import { job.cancel.store(true, Ordering::Relaxed); self.quit_pending = true; }
        else { self.clear_alerts(); let _ = slint::quit_event_loop(); }
    }
}

fn apply_native_theme<C: ComponentHandle>(window: &C, owner: Option<windows_sys::Win32::Foundation::HWND>)
where for<'a> Theme<'a>: slint::Global<'a, C> {
    let theme = window.global::<Theme>();
    native::theme_frame(window.window(), owner, native::FramePalette {
        dark: theme.get_dark(), caption: theme.get_bg(), text: theme.get_text(), border: theme.get_line(),
    });
}

fn show_owned<C: ComponentHandle + 'static>(window: &C, parent: Option<&slint::Window>)
where for<'a> Theme<'a>: slint::Global<'a, C> {
    let _ = window.show();
    let owner = parent.and_then(native::hwnd);
    apply_native_theme(window, owner);
    if let Some(parent) = parent {
        {
            let position = parent.position();
            let owner_size = parent.size(); let size = window.window().size();
            window.window().set_position(slint::PhysicalPosition::new(position.x + (owner_size.width as i32 - size.width as i32) / 2,
                position.y + (owner_size.height as i32 - size.height as i32) / 2));
        }
    }
    let weak = window.as_weak();
    slint::Timer::single_shot(Duration::from_millis(10), move || {
        // Read the live palette again: a theme change before this callback must
        // not restore the colors captured when the popup was first opened.
        if let Some(window) = weak.upgrade() { apply_native_theme(&window, owner); }
    });
}

fn present_alert(controller: &Controller) {
    let alert = {
        let mut app = controller.borrow_mut();
        if app.alert.is_some() { return; }
        app.alerts.pop_front()
    };
    let Some(alert) = alert else { return; };
    controller.borrow_mut().stop_preview();
    let Ok(ui) = AlertWindow::new() else { return; };
    ui.global::<Theme>().set_dark(controller.borrow().engine.doc.settings.theme == 0);
    ui.global::<Theme>().set_variant(controller.borrow().engine.doc.settings.color_theme);
    ui.set_heading(i18n::t(&alert.title).into());
    ui.set_label(if alert.default_label { i18n::t(&alert.label).into() } else { alert.label.clone().into() });
    ui.set_warning(alert.warning); ui.set_countdown(alert.countdown);
    let weak = Rc::downgrade(controller);
    ui.on_dismiss(move || { if let Some(app) = weak.upgrade() { app.borrow_mut().close_alert(); present_alert(&app); } });
    let weak = Rc::downgrade(controller);
    ui.on_quick_round(move |index| { if let Some(app) = weak.upgrade() {
        let seconds = app.borrow().engine.doc.settings.quick_snooze_seconds.get(index.max(0) as usize).copied();
        if let Some(seconds) = seconds { start_quick_round(&app, seconds); }
    } });
    let weak = Rc::downgrade(controller);
    ui.on_restart(move || { if let Some(app) = weak.upgrade() {
        let mut state = app.borrow_mut(); let id = state.alert.as_ref().map(|(_, a)| a.id.clone());
        if let Some(id) = id { state.engine.timer_action(&id, "reset"); }
        state.close_alert(); state.commit(); state.refresh(); drop(state); present_alert(&app);
    } });
    let weak = Rc::downgrade(controller);
    ui.window().on_close_requested(move || {
        if let Some(app) = weak.upgrade() { app.borrow_mut().close_alert(); present_alert(&app); }
        slint::CloseRequestResponse::HideWindow
    });
    let parent = controller.borrow().ui.upgrade();
    {
        let state = controller.borrow();
        show_owned(&ui, if state.compact { Some(state.mini.window()) } else { parent.as_ref().map(|p| p.window()) });
    }
    {
        let mut app = controller.borrow_mut();
        app.alert = Some((ui, alert)); app.refresh_alert_options();
    }
}

fn present_clock_alert(controller: &Controller) {
    let alert = {
        let mut state = controller.borrow_mut();
        let Some(alert) = state.pending_clock.take() else { return; };
        state.close_clock_alert(); alert
    };
    let Ok(ui) = AlertWindow::new() else { return; };
    ui.set_clock_chime(true); ui.set_heading(i18n::t(&alert.title).into()); ui.set_label(alert.label.clone().into());
    let dark = controller.borrow().engine.doc.settings.theme == 0;
    ui.global::<Theme>().set_dark(dark);
    ui.global::<Theme>().set_variant(controller.borrow().engine.doc.settings.color_theme);
    let weak = Rc::downgrade(controller);
    ui.on_dismiss(move || { if let Some(app) = weak.upgrade() { app.borrow_mut().close_clock_alert(); } });
    let weak = Rc::downgrade(controller);
    ui.window().on_close_requested(move || {
        if let Some(app) = weak.upgrade() { app.borrow_mut().close_clock_alert(); }
        slint::CloseRequestResponse::HideWindow
    });
    let parent = controller.borrow().ui.upgrade();
    {
        let state = controller.borrow();
        show_owned(&ui, if state.compact { Some(state.mini.window()) } else { parent.as_ref().map(|main| main.window()) });
    }
    controller.borrow_mut().clock_alert = Some((ui, alert));
}

fn start_quick_round(controller: &Controller, seconds: u64) {
    let mut state = controller.borrow_mut();
    let Some((_, alert)) = &state.alert else { return; };
    let alert = alert.clone();
    let seconds = seconds.clamp(1, 5999);
    if alert.countdown {
        state.engine.cancel_cycle(&alert.id);
        if let Err(error) = state.engine.start(seconds, if alert.default_label { "" } else { &alert.label }, alert.sound) {
            state.engine.message = error; state.refresh(); return;
        }
    } else {
        let when = Local::now() + chrono::Duration::seconds(seconds as i64);
        state.engine.put_alarm(Alarm { id: data::new_id(), label: alert.label, hour: when.hour(), minute: when.minute(),
            days: 0, end_minute: None, fire_at: when.timestamp(), sound: alert.sound, warn_before: false, enabled: true });
    }
    state.close_alert(); state.commit(); state.refresh(); drop(state); present_alert(controller);
}

fn open_editor(controller: &Controller, id: Option<String>) {
    let mut app = controller.borrow_mut();
    let Some(ui) = app.ui.upgrade() else { return; };
    ui.set_page(1);
    if app.editor.is_some() { ui.set_editor_visible(true); return; }
    let alarm = id.as_ref().and_then(|id| app.engine.doc.alarms.iter().find(|a| &a.id == id)).cloned();
    let now = Local::now() + chrono::Duration::minutes(5);
    let (hour, minute, days) = alarm.as_ref().map(|a| (a.hour, a.minute, a.days)).unwrap_or((now.hour(), now.minute(), 0));
    ui.set_editor_editing(alarm.is_some()); ui.set_editor_label(alarm.as_ref().map(|a| a.label.clone()).unwrap_or_default().into());
    ui.set_editor_hour(hour as i32); ui.set_editor_minute(minute as i32);
    let date = alarm.as_ref().and_then(|a| Local.timestamp_opt(a.fire_at, 0).single()).unwrap_or(now).date_naive();
    ui.set_editor_date(date.format("%Y-%m-%d").to_string().into());
    ui.set_editor_sound_index(app.sound_index(alarm.as_ref().map(|a| a.sound).unwrap_or(app.engine.doc.settings.default_sound)));
    ui.set_editor_repeat_index(match days { 0 => 0, 127 => 1, 31 => 2, 96 => 3, _ => 4 });
    ui.set_editor_selected_days(ModelRc::new(VecModel::from((0..7).map(|i| days & (1 << i) != 0).collect::<Vec<_>>())));
    let end = alarm.as_ref().and_then(|a| a.end_minute);
    ui.set_editor_use_end(end.is_some()); ui.set_editor_end_hour(end.map(|e| e / 60).unwrap_or((hour + 1).min(23)) as i32);
    ui.set_editor_end_minute(end.map(|e| e % 60).unwrap_or(minute) as i32);
    ui.set_editor_warn_before(alarm.as_ref().is_some_and(|a| a.warn_before));
    ui.set_editor_error_message("".into()); ui.set_pending_delete_id("".into());
    app.editor = Some(Editor { id, days }); ui.set_editor_visible(true);
}

fn save_editor(controller: &Controller) {
    let mut app = controller.borrow_mut();
    let Some(editor) = &app.editor else { return; };
    let Some(ui) = app.ui.upgrade() else { return; };
    let hour = ui.get_editor_hour().clamp(0, 23) as u32; let minute = ui.get_editor_minute().clamp(0, 59) as u32;
    let days = if ui.get_editor_repeat_index() == 0 { 0 } else { editor.days };
    if ui.get_editor_repeat_index() != 0 && days == 0 { ui.set_editor_error_message(i18n::t("请至少选择一个重复日期。").into()); return; }
    let now = Local::now().timestamp();
    let fire_at = if days == 0 {
        let Ok(date) = NaiveDate::parse_from_str(ui.get_editor_date().as_str(), "%Y-%m-%d") else { ui.set_editor_error_message(i18n::t("日期格式为 YYYY-MM-DD，例如 2026-10-16。").into()); return; };
        match data::local_time(date, hour, minute) { Some(t) if t > now => t, _ => { ui.set_editor_error_message(i18n::t("请选择将来的日期与时间。").into()); return; } }
    } else { match data::next_occurrence(now, hour, minute, days) { Some(t) => t, None => { ui.set_editor_error_message(i18n::t("无法安排这个时间，请重新选择。").into()); return; } } };
    let end_minute = if ui.get_editor_use_end() {
        let end = ui.get_editor_end_hour() as u32 * 60 + ui.get_editor_end_minute() as u32;
        if end <= hour * 60 + minute || end >= 1440 { ui.set_editor_error_message(i18n::t("结束时间需要晚于开始时间，且在同一天。").into()); return; }
        Some(end)
    } else { None };
    let selected_sound = app.sound_choice(ui.get_editor_sound_index());
    if !app.sound_exists(selected_sound) { ui.set_editor_error_message(i18n::t("请先在设置中导入自定义提示音。").into()); return; }
    let id = editor.id.clone().unwrap_or_else(data::new_id);
    let enabled = app.engine.doc.alarms.iter().find(|a| a.id == id).map(|a| a.enabled).unwrap_or(true);
    let alarm = Alarm { id, label: data::text(ui.get_editor_label().as_str()), hour, minute, days, end_minute, fire_at,
        sound: selected_sound, warn_before: ui.get_editor_warn_before(), enabled };
    app.engine.put_alarm(alarm); app.editor = None; app.commit(); app.refresh(); ui.set_editor_visible(false);
}

fn confirm(controller: &Controller, action: Confirmation) {
    if let Some((window, _)) = &controller.borrow().confirm { native::show(window.window()); return; }
    let Ok(ui) = ConfirmWindow::new() else { return; };
    ui.global::<Theme>().set_dark(controller.borrow().engine.doc.settings.theme == 0);
    ui.global::<Theme>().set_variant(controller.borrow().engine.doc.settings.color_theme);
    match &action {
        Confirmation::Import(document) => {
            ui.set_heading(i18n::t("导入这份备份？").into()); ui.set_action_label(i18n::t("导入备份").into());
            ui.set_message(i18n::message(&format!("备份中有 {} 项闹钟与 {} 项倒计时。当前数据会先保存一份临时备份，然后由这些计划替换。", document.alarms.len(), document.countdowns.len())).into());
            ui.set_show_import_settings(true);
        }
        Confirmation::Clear => { ui.set_heading(i18n::t("清除全部计划？").into()); ui.set_message(i18n::t("将清除全部闹钟和倒计时，此操作无法撤销。").into()); ui.set_action_label(i18n::t("清除计划").into()); }
        Confirmation::Reset => { ui.set_heading(i18n::t("恢复默认设置？").into()); ui.set_message(i18n::t("界面和提示音偏好将恢复默认，开机启动将关闭。已有闹钟与倒计时会继续保留。").into()); ui.set_action_label(i18n::t("恢复默认").into()); }
    }
    let parent = controller.borrow().ui.upgrade();
    let weak = Rc::downgrade(controller);
    ui.on_cancel(move || { if let Some(app) = weak.upgrade() { if let Some((window, _)) = app.borrow_mut().confirm.take() { let _ = window.hide(); } } });
    let weak = Rc::downgrade(controller);
    ui.window().on_close_requested(move || { if let Some(app) = weak.upgrade() { app.borrow_mut().confirm = None; } slint::CloseRequestResponse::HideWindow });
    let weak = Rc::downgrade(controller);
    ui.on_accept(move || { if let Some(app) = weak.upgrade() {
        let mut state = app.borrow_mut();
        let Some((window, action)) = state.confirm.take() else { return; };
        let include_settings = window.get_include_settings(); let _ = window.hide();
        let mut imported = false;
        match action {
            Confirmation::Import(mut document) => {
                if state.import.is_some() {
                    state.engine.message = i18n::t("请等待音频导入完成后，再导入备份。").into(); state.refresh(); return;
                }
                let snapshot = state.engine.snapshot();
                let backup = serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())
                    .and_then(|bytes| data::atomic_write(&state.folders.roaming.join("data-before-import.json"), &bytes));
                if let Err(error) = backup {
                    state.engine.message = format!("导入已停止，无法创建当前数据的备份：{error}"); state.refresh(); return;
                }
                if !include_settings { document.settings = snapshot.settings; }
                document.settings.startup = native::startup_enabled();
                // Persist the replacement before changing live state. A disk error
                // leaves the current scheduler and its plans intact.
                let replacement = Engine::new(data::sanitize(document), i18n::t("备份已导入。").into());
                if let Err(error) = data::save(&state.folders, &replacement.snapshot()) {
                    state.engine.message = format!("导入失败，当前计划未更改：{error}"); state.refresh(); return;
                }
                state.stop_preview(); state.clear_alerts();
                state.engine = replacement; imported = true;
                state.restore_focus_draft();
                state.relocalize();
                if let Some(root) = state.ui.upgrade() {
                    root.set_draft_sound(state.sound_index(state.engine.doc.settings.default_sound)); state.refresh_sound_library();
                    if include_settings {
                        let settings = &state.engine.doc.settings;
                        root.window().set_size(slint::LogicalSize::new(settings.width, settings.height));
                        root.set_page(settings.selected_tab); root.set_settings_scroll_y(0.);
                    }
                }
            }
            Confirmation::Clear => {
                let mut settings = state.engine.doc.settings.clone(); settings.focus_timer_id.clear();
                state.clear_alerts();
                state.engine = Engine::new(Document { settings, total_focus_ms: state.engine.doc.total_focus_ms, focus_days: state.engine.doc.focus_days.clone(), focus_stats_initialized: true,
                    countdown_names: state.engine.doc.countdown_names.clone(), alarm_names: state.engine.doc.alarm_names.clone(), name_history_initialized: true, ..Document::default() }, i18n::t("已清除全部计划。").into());
            }
            Confirmation::Reset => {
                if let Err(error) = native::startup(false) { state.engine.message = error; state.refresh(); return; }
                let custom_name = state.engine.doc.settings.custom_name.clone();
                let custom_sounds = state.engine.doc.settings.custom_sounds.clone();
                if state.engine.doc.stopwatch.running { state.engine.stopwatch_action("toggle"); }
                state.engine.set_auto_cycle(false);
                state.engine.doc.settings = data::Settings { custom_name, custom_sounds, ..data::Settings::default() };
                state.restore_focus_draft();
                i18n::select(0); state.apply_theme();
                if let Some(root) = state.ui.upgrade() { root.set_sounds(state.sounds()); }
                if let Some(root) = state.ui.upgrade() {
                    root.set_draft_sound(0); root.set_default_sound(0);
                    root.window().set_size(slint::LogicalSize::new(480., 760.));
                }
                state.engine.message = i18n::t("已恢复默认设置。").into();
            }
        }
        state.editor = None;
        if let Some(ui) = state.ui.upgrade() { ui.set_editor_visible(false); ui.set_pending_delete_id("".into()); }
        if !imported { state.commit(); } state.refresh();
    } });
    controller.borrow_mut().confirm = Some((ui.clone_strong(), action));
    show_owned(&ui, parent.as_ref().map(|p| p.window()));
}

fn choose_audio(controller: &Controller) {
    if controller.borrow().import.is_some() { return; }
    let file = rfd::FileDialog::new().set_title(i18n::t("选择自定义提示音"))
        .add_filter(i18n::t("音频文件"), &["wav", "mp3", "m4a", "aac", "flac", "ogg", "oga", "opus", "wma", "aif", "aiff", "alac", "ape", "wv", "mka", "webm", "mp4", "ac3", "eac3", "dts", "amr", "3gp", "au", "caf", "m4b", "m4r", "tta", "tak", "dsf", "dff"])
        .add_filter(i18n::t("所有文件"), &["*"]).pick_file();
    if let Some(source) = file {
        let mut app = controller.borrow_mut();
        if app.engine.doc.settings.custom_sounds.len() >= 1000 { app.import_notice = i18n::t("最多保留 1000 段自定义提示音。").into(); app.refresh(); return; }
        let mut id = app.engine.doc.settings.custom_sounds.iter().map(|s| s.id).max().unwrap_or(7).saturating_add(1);
        while id < i32::MAX && (data::builtin_sound_index(id).is_some() || app.folders.custom_sound(id).exists()) { id += 1; }
        if id == i32::MAX { return; }
        app.import = Some(audio::start_import(source, app.folders.clone(), id));
        app.import_notice.clear();
        app.refresh();
    }
}
fn export_data(controller: &Controller) {
    let document = controller.borrow().engine.snapshot();
    let file = rfd::FileDialog::new().set_title(i18n::t("导出 Timoa 备份"))
        .set_file_name(format!("timoa-backup-{}.json", Local::now().format("%Y-%m-%d")))
        .add_filter(i18n::t("Timoa 备份"), &["json"]).save_file();
    if let Some(path) = file {
        let result = serde_json::to_vec_pretty(&document).map_err(|e| e.to_string()).and_then(|bytes| data::atomic_write(&path, &bytes));
        let mut app = controller.borrow_mut();
        app.engine.message = match result { Ok(()) => i18n::t("备份已导出。自定义音频需单独保存。").into(), Err(error) => format!("导出失败：{error}") };
        app.message_deadline = None;
        app.refresh();
    }
}
fn import_data(controller: &Controller) {
    if controller.borrow().import.is_some() { return; }
    let file = rfd::FileDialog::new().set_title(i18n::t("导入 Timoa 备份")).add_filter(i18n::t("Timoa 备份"), &["json"]).pick_file();
    if let Some(path) = file {
        let result = fs::metadata(&path).map_err(|e| e.to_string()).and_then(|m| {
            if m.len() > 5 * 1024 * 1024 { return Err(i18n::t("备份文件超过 5 MB。").into()); }
            fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| data::parse(&bytes))
        });
        match result {
            Ok(document) => confirm(controller, Confirmation::Import(document)),
            Err(error) => { let mut app = controller.borrow_mut(); app.engine.message = error; app.refresh(); }
        }
    }
}
fn about(controller: &Controller) {
    if let Some(window) = &controller.borrow().about { native::show(window.window()); return; }
    let Ok(ui) = AboutWindow::new() else { return; };
    ui.set_version_text(format!("{}  ·  Windows 7–11  ·  {}", env!("CARGO_PKG_VERSION"),
        if cfg!(target_arch = "x86") { "x86" } else { "x64" }).into());
    ui.global::<Theme>().set_dark(controller.borrow().engine.doc.settings.theme == 0);
    ui.global::<Theme>().set_variant(controller.borrow().engine.doc.settings.color_theme);
    let parent = controller.borrow().ui.upgrade();
    let weak = Rc::downgrade(controller);
    ui.on_dismiss_about(move || { if let Some(app) = weak.upgrade() { if let Some(window) = app.borrow_mut().about.take() { let _ = window.hide(); } } });
    let weak = Rc::downgrade(controller);
    ui.window().on_close_requested(move || { if let Some(app) = weak.upgrade() { app.borrow_mut().about = None; } slint::CloseRequestResponse::HideWindow });
    let weak = Rc::downgrade(controller);
    ui.on_licenses(move || { if let Some(app) = weak.upgrade() {
        let folders = app.borrow().folders.clone();
        let path = folders.roaming.join("licenses/Third-party-licenses.txt");
        let mut text = concat!("Timoa\r\nApache License 2.0\r\n\r\n",
            include_str!("../licenses/Apache-2.0.txt"), "\r\n\r\nSlint\r\n",
            include_str!("../licenses/Slint-Royalty-free-2.0.md"), "\r\n\r\nFemtoVG\r\n",
            include_str!("../licenses/FemtoVG-NOTICE.txt"), "\r\n\r\n",
            include_str!("../licenses/FemtoVG-MIT.txt"), "\r\n\r\nSarasa Gothic / Sarasa UI SC\r\n",
            include_str!("../licenses/Sarasa-Gothic-OFL.txt"), "\r\n\r\nFFmpeg\r\n",
            include_str!("../licenses/FFmpeg-NOTICE.txt"), "\r\n\r\nMinGW-w64 runtime\r\n",
            include_str!("../licenses/MinGW-w64-runtime.txt"), "\r\n\r\nUPX executable compression\r\n",
            include_str!("../licenses/UPX-License.txt"), "\r\n\r\n",
            include_str!("../licenses/UPX-GPL-2.0.txt")).to_owned();
        let mut rust_notices = String::new();
        use std::io::Read;
        let _ = flate2::read::GzDecoder::new(include_bytes!("../licenses/Rust-dependencies.txt.gz").as_slice()).read_to_string(&mut rust_notices);
        text.push_str("\r\n\r\n"); text.push_str(&rust_notices);
        if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
        if fs::write(&path, text.as_bytes()).is_ok() { native::open(&path); }
    } });
    controller.borrow_mut().about = Some(ui.clone_strong()); show_owned(&ui, parent.as_ref().map(|p| p.window()));
}

fn bind(controller: &Controller, ui: &MainWindow) {
    macro_rules! action {
        ($setter:ident, |$app:ident $(, $argument:ident)*| $body:block) => {{
            let weak = Rc::downgrade(controller);
            ui.$setter(move |$($argument),*| { if let Some($app) = weak.upgrade() $body });
        }};
    }
    action!(on_navigate, |app, index| {
        let mut app = app.borrow_mut(); app.engine.doc.settings.selected_tab = index;
        if index == 0 { app.engine.doc.settings.active_focus_mode = 0; }
        if index == 4 && app.engine.doc.settings.stopwatch_enabled { app.engine.doc.settings.active_focus_mode = 1; }
        if let Some(ui) = app.ui.upgrade() { ui.set_hint_visible(false); ui.set_focus_shortcuts_editing(false); }
        app.commit(); app.refresh();
    });
    action!(on_clear_message, |app| { let mut app = app.borrow_mut(); app.engine.message.clear(); app.refresh(); });
    action!(on_draft_changed, |app| {
        let mut app = app.borrow_mut();
        if let Some(ui) = app.ui.upgrade() {
            let duration = ui.get_draft_hour().max(0) as u64 * 3600 + ui.get_draft_minute().max(0) as u64 * 60 + ui.get_draft_second().max(0) as u64;
            app.engine.doc.settings.focus_duration_seconds = duration.clamp(1, 359999); app.commit(); app.refresh();
        }
    });
    action!(on_change_single_focus, |app| {
        let mut app = app.borrow_mut(); app.engine.doc.settings.single_focus = !app.engine.doc.settings.single_focus;
        if app.engine.doc.settings.single_focus { app.select_focus_timer(); }
        app.commit(); app.refresh();
    });
    action!(on_change_stopwatch_enabled, |app| {
        let mut app = app.borrow_mut(); app.engine.doc.settings.stopwatch_enabled = !app.engine.doc.settings.stopwatch_enabled;
        if !app.engine.doc.settings.stopwatch_enabled {
            app.engine.doc.settings.active_focus_mode = 0;
            if app.engine.doc.stopwatch.running { app.engine.stopwatch_action("toggle"); }
            if let Some(ui) = app.ui.upgrade() { if ui.get_page() == 4 { ui.set_page(0); } }
        } else if !app.engine.has_running_countdown() {
            app.engine.doc.settings.active_focus_mode = 1;
        }
        app.commit(); app.refresh();
    });
    action!(on_stopwatch_action, |app, verb| {
        let mut app = app.borrow_mut();
        if !app.engine.doc.settings.stopwatch_enabled { return; }
        app.engine.stopwatch_action(verb.as_str()); app.commit(); app.refresh();
    });
    action!(on_change_quick_round, |app, index, seconds| {
        let mut app = app.borrow_mut(); let seconds = seconds.clamp(1, 5999) as u64;
        if (0..3).contains(&index) { app.engine.doc.settings.quick_snooze_seconds[index as usize] = seconds; }
        app.commit(); app.refresh();
    });
    action!(on_change_auto_cycle, |app| {
        let mut app = app.borrow_mut(); let enabled = !app.engine.doc.settings.auto_cycle;
        app.engine.set_auto_cycle(enabled); app.commit(); app.refresh();
    });
    action!(on_change_hourly_chime, |app| {
        let mut app = app.borrow_mut(); app.engine.doc.settings.hourly_chime = !app.engine.doc.settings.hourly_chime;
        app.commit(); app.refresh();
    });
    action!(on_change_half_hour_chime, |app| {
        let mut app = app.borrow_mut(); app.engine.doc.settings.half_hour_chime = !app.engine.doc.settings.half_hour_chime;
        app.commit(); app.refresh();
    });
    action!(on_delete_name, |app, kind, name| {
        let mut app = app.borrow_mut();
        let history = if kind == 0 { &mut app.engine.doc.countdown_names } else { &mut app.engine.doc.alarm_names };
        history.retain(|item| item != name.as_str()); app.commit(); app.refresh();
    });
    action!(on_change_cycle_interval, |app, seconds| {
        let mut app = app.borrow_mut(); app.engine.doc.settings.cycle_interval_seconds = seconds.clamp(0, 3600) as u64;
        app.commit(); app.refresh();
    });
    action!(on_start_countdown, |app| {
        app.borrow_mut().start_countdown();
    });
    action!(on_timer_action, |app, id, verb| {
        let mut app = app.borrow_mut();
        if verb == "remove" && app.engine.doc.settings.focus_timer_id == id.as_str() {
            if let Some(timer) = app.engine.focus_timer() { app.engine.doc.settings.focus_duration_seconds = timer.original_ms / 1000; }
            app.restore_focus_draft();
        }
        app.engine.timer_action(id.as_str(), verb.as_str()); app.commit(); app.refresh();
    });
    action!(on_add_alarm, |app| { open_editor(&app, None); });
    action!(on_alarm_action, |app, id, verb| {
        match verb.as_str() {
            "edit" => open_editor(&app, Some(id.to_string())),
            "delete" => { if let Some(ui) = app.borrow().ui.upgrade() { ui.set_pending_delete_id(id); } }
            "cancel-delete" => { if let Some(ui) = app.borrow().ui.upgrade() { ui.set_pending_delete_id("".into()); } }
            "confirm-delete" => {
                let mut app = app.borrow_mut();
                if let Some(ui) = app.ui.upgrade() {
                    if ui.get_pending_delete_id() == id {
                        app.engine.remove_alarm(id.as_str()); ui.set_pending_delete_id("".into()); app.commit(); app.refresh();
                    }
                }
            }
            "toggle" => { let mut app = app.borrow_mut(); app.engine.toggle_alarm(id.as_str()); app.commit(); app.refresh(); }
            _ => {}
        }
    });
    action!(on_editor_repeat_changed, |app, index| {
        let mut app = app.borrow_mut(); let root = app.ui.upgrade();
        if let Some(editor) = &mut app.editor {
            editor.days = match index { 0 => 0, 1 => 127, 2 => 31, 3 => 96, _ => if editor.days == 0 { 31 } else { editor.days } };
            if let Some(ui) = root { ui.set_editor_selected_days(ModelRc::new(VecModel::from((0..7).map(|i| editor.days & (1 << i) != 0).collect::<Vec<_>>()))); }
        }
    });
    action!(on_editor_day_toggled, |app, index| {
        let mut app = app.borrow_mut(); let root = app.ui.upgrade();
        if let Some(editor) = &mut app.editor {
            editor.days ^= 1 << index.clamp(0, 6);
            if let Some(ui) = root { ui.set_editor_repeat_index(4); ui.set_editor_selected_days(ModelRc::new(VecModel::from((0..7).map(|i| editor.days & (1 << i) != 0).collect::<Vec<_>>()))); }
        }
    });
    action!(on_editor_save, |app| { save_editor(&app); });
    action!(on_editor_cancel, |app| { let mut app = app.borrow_mut(); app.editor = None; if let Some(ui) = app.ui.upgrade() { ui.set_editor_visible(false); } });
    action!(on_change_theme, |app, index| { let mut app = app.borrow_mut(); app.engine.doc.settings.theme = index.clamp(0, 1); app.apply_theme(); app.commit(); });
    action!(on_change_color_theme, |app, index| { let mut app = app.borrow_mut(); app.engine.doc.settings.color_theme = index.clamp(0, 5); app.apply_theme(); app.commit(); });
    action!(on_preview_sound, |app, index| { app.borrow_mut().preview(index); });
    action!(on_choose_week, |app, day| { let mut app = app.borrow_mut(); app.week_day = day.clamp(0, 6); app.refresh(); });
    action!(on_change_week, |app, offset| { let mut app = app.borrow_mut(); app.week_offset = (app.week_offset + offset as i64).clamp(-520, 520); app.refresh(); });
    action!(on_change_default_sound, |app, choice| {
        let mut app = app.borrow_mut(); app.engine.doc.settings.default_sound = app.sound_choice(choice);
        if let Some(ui) = app.ui.upgrade() { ui.set_draft_sound(choice); }
        app.commit(); app.refresh();
    });
    action!(on_change_language, |app, language| {
        let mut app = app.borrow_mut();
        app.engine.doc.settings.language = language.clamp(0, 2);
        app.relocalize(); app.commit(); app.refresh();
    });
    action!(on_change_startup, |app| {
        let mut app = app.borrow_mut(); let enabled = !app.engine.doc.settings.startup;
        match native::startup(enabled) { Ok(()) => { app.engine.doc.settings.startup = enabled; app.commit(); }, Err(error) => { app.engine.doc.settings.startup = native::startup_enabled(); app.engine.message = error; } }
        app.refresh();
    });
    action!(on_choose_audio, |app| { choose_audio(&app); });
    action!(on_preview_audio, |app| { let mut app = app.borrow_mut(); let index = app.sound_index(7); app.preview(index); });
    action!(on_remove_sound, |app, id| {
        let mut app = app.borrow_mut();
        if app.import.is_some() || !app.engine.doc.settings.custom_sounds.iter().any(|s| s.id == id) { return; }
        let draft = app.ui.upgrade().map(|ui| (app.sound_choice(ui.get_draft_sound()), app.sound_choice(ui.get_editor_sound_index())));
        if app.preview_index == id { app.stop_preview(); }
        match fs::remove_file(app.folders.custom_sound(id)) {
            Ok(()) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(_) => { app.engine.message = i18n::t("无法移除提示音文件，请检查文件是否被占用。").into(); app.refresh(); return; }
        }
        app.engine.doc.settings.custom_sounds.retain(|s| s.id != id);
        if app.engine.doc.settings.default_sound == id { app.engine.doc.settings.default_sound = 0; }
        for timer in &mut app.engine.doc.countdowns { if timer.sound == id { timer.sound = 0; } }
        for alarm in &mut app.engine.doc.alarms { if alarm.sound == id { alarm.sound = 0; } }
        for alert in &mut app.alerts { if alert.sound == id { alert.sound = 0; } }
        app.refresh_sound_library();
        if let Some(ui) = app.ui.upgrade() {
            if let Some((timer, editor)) = draft { ui.set_draft_sound(app.sound_index(timer)); ui.set_editor_sound_index(app.sound_index(editor)); }
        }
        app.import_notice = i18n::t("已删除提示音，使用它的计划已改为静音。").into(); app.commit(); app.refresh();
    });
    action!(on_cancel_import, |app| { if let Some(job) = &app.borrow().import { job.cancel.store(true, Ordering::Relaxed); } });
    action!(on_export_data, |app| { export_data(&app); });
    action!(on_import_data, |app| { import_data(&app); });
    action!(on_clear_alarms, |app| { confirm(&app, Confirmation::Clear); });
    action!(on_reset_settings, |app| { confirm(&app, Confirmation::Reset); });
    action!(on_about, |app| { about(&app); });
    action!(on_open_data, |app| { let folder = app.borrow().folders.roaming.clone(); let _ = fs::create_dir_all(&folder); native::open(&folder); });
    let hint_weak = ui.as_weak();
    ui.on_info_hint(move |kind, visible| {
        if let Some(ui) = hint_weak.upgrade() {
            let visible = visible && ui.get_page() == 3 && (kind == 3 || ui.get_more_settings_open());
            if visible {
                ui.set_hint_kind(kind);
                if let Some(position) = native::pointer_position(ui.window()) {
                    ui.set_hint_x(position.x); ui.set_hint_y(position.y);
                }
            }
            ui.set_hint_visible(visible);
        }
    });
    let weak = Rc::downgrade(controller);
    let main_weak = ui.as_weak();
    ui.on_caption_action(move |verb| {
        // Native maximize may dispatch nested window messages; release App borrows
        // before the OS call. Dragging is handled entirely by WindowMoveArea.
        if let Some(app) = weak.upgrade() {
            match verb.as_str() {
                "compact" => app.borrow_mut().set_compact(true),
                "close" => app.borrow_mut().hide_current_window(),
                action => if let Some(ui) = main_weak.upgrade() { native::caption_action(ui.window(), action); },
            }
        }
    });
    let weak = Rc::downgrade(controller);
    let mini_weak = controller.borrow().mini.as_weak();
    controller.borrow().mini.on_caption_action(move |verb| {
        if let Some(app) = weak.upgrade() {
            match verb.as_str() {
                "restore" => app.borrow_mut().set_compact(false),
                "close" => app.borrow_mut().hide_current_window(),
                action => if let Some(ui) = mini_weak.upgrade() { native::caption_action(ui.window(), action); },
            }
        }
    });
    let weak = Rc::downgrade(controller);
    controller.borrow().mini.window().on_close_requested(move || {
        if let Some(app) = weak.upgrade() { app.borrow_mut().hide_current_window(); }
        slint::CloseRequestResponse::HideWindow
    });
    let weak = Rc::downgrade(controller);
    ui.window().on_close_requested(move || { if let Some(app) = weak.upgrade() { app.borrow_mut().commit(); } slint::CloseRequestResponse::HideWindow });
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let startup = std::env::args().any(|s| s == "--startup");
    let Some(_instance) = native::Instance::claim(!startup).map_err(std::io::Error::other)? else { return Ok(()); };
    slint::BackendSelector::new().backend_name("winit".into()).select()?;
    let folders = Folders::system();
    let (mut document, message) = data::load(&folders);
    document.settings.startup = native::startup_enabled();
    if folders.custom().is_file() && !document.settings.custom_sounds.iter().any(|s| s.id == 7) {
        document.settings.custom_sounds.insert(0, data::CustomSound { id: 7, name: if document.settings.custom_name.is_empty() { "自定义提示音".into() } else { document.settings.custom_name.clone() } });
    }
    let ui = MainWindow::new()?;
    let mini = MiniWindow::new()?;
    i18n::select(document.settings.language);
    ui.window().set_size(slint::LogicalSize::new(document.settings.width, document.settings.height));
    ui.set_page(document.settings.selected_tab);
    let timer_rows = Rc::new(VecModel::<TimerRow>::default()); let alarm_rows = Rc::new(VecModel::<AlarmRow>::default());
    let day_rows = Rc::new(VecModel::<DayRow>::default()); let week_rows = Rc::new(VecModel::<AlarmRow>::default());
    let focus_bars = Rc::new(VecModel::<FocusBar>::default());
    let countdown_names = Rc::new(VecModel::<SharedString>::default());
    let alarm_names = Rc::new(VecModel::<SharedString>::default());
    ui.set_timers(timer_rows.clone().into()); ui.set_alarms(alarm_rows.clone().into());
    ui.set_week_days(day_rows.clone().into()); ui.set_week_items(week_rows.clone().into());
    ui.set_focus_bars(focus_bars.clone().into());
    ui.set_countdown_names(countdown_names.clone().into()); ui.set_alarm_names(alarm_names.clone().into());
    let app = Rc::new(RefCell::new(App { ui: ui.as_weak(), mini, compact: false, mini_positioned: false, engine: Engine::new(document, message), folders,
        player: audio::Player::default(), import: None, timer_rows, alarm_rows, day_rows, week_rows, focus_bars, countdown_names, alarm_names, week_offset: 0,
        week_day: Local::now().weekday().num_days_from_monday() as i32, editor: None, confirm: None, about: None,
        alerts: VecDeque::new(), alert: None, clock_alert: None, pending_clock: None, sound_owner: None, quit_pending: false, import_notice: String::new(), preview_index: -1, preview_deadline: None,
        message_seen: String::new(), message_deadline: None }));
    app.borrow().restore_focus_draft();
    app.borrow().refresh_sound_library(); ui.set_draft_sound(app.borrow().sound_index(app.borrow().engine.doc.settings.default_sound)); bind(&app, &ui); app.borrow().apply_theme(); app.borrow_mut().refresh();
    shortcuts::install(&app, &ui);
    // Persist migration to this profile only; the WPF profile remains readable by the previous version.
    app.borrow_mut().commit();
    let (native_sender, native_receiver) = mpsc::channel();
    let bridge = Rc::new(RefCell::new(native::Bridge::new(native_sender).map_err(std::io::Error::other)?));
    if !startup { ui.show()?; }
    for delay in [0, 60, 180] {
        let weak = Rc::downgrade(&app);
        slint::Timer::single_shot(Duration::from_millis(delay), move || { if let Some(app) = weak.upgrade() { app.borrow().apply_theme(); } });
    }
    let timer = slint::Timer::default();
    let weak = Rc::downgrade(&app); let bridge_tick = bridge.clone();
    let mut last_second = 0i64;
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(100), move || {
        let Some(controller) = weak.upgrade() else { return; };
        let mut app = controller.borrow_mut();
        while let Ok(event) = native_receiver.try_recv() {
            match event {
                native::Event::Show => app.show_current_window(),
                native::Event::Quit => app.request_quit(),
            }
        }
        let import_result = app.import.as_ref().and_then(|job| match job.receiver.try_recv() {
            Ok(result) => Some(result), Err(mpsc::TryRecvError::Disconnected) => Some(Err(i18n::t("音频转换任务意外结束。").into())), _ => None,
        });
        if let Some(result) = import_result {
            app.import = None;
            match result {
                Ok(result) => {
                    app.engine.doc.settings.custom_sounds.push(data::CustomSound { id: result.id, name: result.name });
                    app.refresh_sound_library();
                    app.import_notice = result.notice;
                    if let Some(ui) = app.ui.upgrade() { ui.set_sounds(app.sounds()); }
                    app.commit();
                }
                Err(error) => app.import_notice = error,
            }
            app.refresh();
            if app.quit_pending { app.request_quit(); }
        }
        if app.preview_deadline.is_some_and(|deadline| Instant::now() >= deadline) { app.stop_preview(); }
        app.player.release_finished();
        if app.message_deadline.is_some_and(|deadline| Instant::now() >= deadline) { app.engine.message.clear(); app.refresh(); }
        let (alerts, changed) = app.engine.tick();
        for id in app.engine.take_cycle_restarts() { app.discard_cycle_alerts(&id); }
        app.receive_alerts(alerts);
        if changed { app.commit(); }
        let second = Local::now().timestamp();
        if second != last_second || changed {
            last_second = second; app.refresh();
            let count = app.engine.doc.countdowns.iter().filter(|t| !t.finished).count();
            bridge_tick.borrow_mut().retry_hotkey();
            bridge_tick.borrow().tooltip(&i18n::message(&format!("Timoa · {count} 项计时 · {}", Local::now().format("%H:%M"))));
            if let Some(ui) = app.ui.upgrade() { ui.set_footer_text(if bridge_tick.borrow().hotkey {
                i18n::t("关闭窗口后，在托盘中继续计时").into()
            } else { i18n::t("快捷键被占用，可从托盘唤起").into() }); }
        }
        let show_alert = app.alert.is_none() && !app.alerts.is_empty();
        let show_clock = app.pending_clock.is_some(); drop(app);
        if show_alert { present_alert(&controller); }
        if show_clock { present_clock_alert(&controller); }
    });
    slint::run_event_loop_until_quit()?;
    app.borrow_mut().commit(); drop(timer); drop(bridge); Ok(())
}

fn main() { if let Err(error) = run() { native::error(&format!("Timoa 无法启动：{error}")); } }
