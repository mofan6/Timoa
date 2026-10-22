//! Pure application state and scheduling; no UI toolkit types or window calls.
use crate::data::*;
use chrono::{Local, TimeZone, Timelike};
use std::{collections::{HashMap, HashSet}, time::{Duration, Instant}};

#[derive(Clone)]
pub struct Alert {
    pub token: String,
    pub id: String,
    pub title: String,
    pub label: String,
    pub sound: i32,
    pub warning: bool,
    pub countdown: bool,
    pub clock_chime: bool,
    pub default_label: bool,
}

pub struct Engine {
    pub doc: Document,
    deadlines: HashMap<String, Instant>,
    warned: HashSet<(String, i64)>,
    pub message: String,
    stopwatch_started: Option<Instant>,
    cycle_waits: HashMap<String, Instant>,
    cycle_restarted: Vec<String>,
    sampled_remaining: HashMap<String, u64>,
    sampled_stopwatch_ms: u64,
    clock_checked_at: i64,
    last_chime_boundary: i64,
}

impl Engine {
    pub fn new(mut doc: Document, mut message: String) -> Self {
        initialize_name_history(&mut doc);
        if !doc.focus_stats_initialized {
            doc.total_focus_ms = doc.countdowns.iter().filter(|t| t.finished)
                .fold(0u64, |total, t| total.saturating_add(t.original_ms));
            doc.focus_stats_initialized = true;
        }
        let now = now_ms();
        let mut missed = 0;
        for timer in &mut doc.countdowns {
            // Countdowns continue across closing/reopening; their saved deadline
            // lets us locate only the known time since the last snapshot.
            if !timer.finished && !timer.paused {
                let current = (timer.ends_at_ms.saturating_sub(now).max(0) as u64).min(timer.remaining_ms);
                crate::stats::add_span(&mut timer.focus_slots, now.min(timer.ends_at_ms), timer.remaining_ms.saturating_sub(current));
                timer.remaining_ms = current;
            }
            if !timer.finished && !timer.paused && now.saturating_sub(timer.ends_at_ms) > 300_000 {
                timer.finished = true; timer.remaining_ms = 0; missed += 1;
                doc.total_focus_ms = doc.total_focus_ms.saturating_add(timer.original_ms);
                crate::stats::record_completion(&mut doc.focus_days, &std::mem::take(&mut timer.focus_slots), timer.ends_at_ms);
            }
        }
        if missed > 0 { message = format!("离开期间有 {missed} 项倒计时已结束。"); }
        let deadlines = doc.countdowns.iter().filter(|t| !t.paused && !t.finished).map(|t| {
            (t.id.clone(), Instant::now() + Duration::from_millis((t.ends_at_ms.saturating_sub(now).max(0) as u64).min(t.original_ms)))
        }).collect();
        let warned = doc.alarms.iter().filter(|a| a.warn_before && now / 1000 >= a.fire_at - 300)
            .map(|a| (a.id.clone(), a.fire_at)).collect();
        // A restored stopwatch resumes paused, so time with the app closed is excluded.
        doc.stopwatch.running = false;
        let sampled_remaining = doc.countdowns.iter().filter(|timer| !timer.finished && !timer.paused)
            .map(|timer| (timer.id.clone(), timer.remaining_ms)).collect();
        let sampled_stopwatch_ms = doc.stopwatch.elapsed_ms;
        Self { doc, deadlines, warned, message, stopwatch_started: None, cycle_waits: HashMap::new(), cycle_restarted: vec![], sampled_remaining, sampled_stopwatch_ms,
            clock_checked_at: now / 1000, last_chime_boundary: 0 }
    }
    pub fn remaining(&self, timer: &Countdown) -> u64 {
        if timer.finished { return 0; }
        if timer.paused { return timer.remaining_ms; }
        self.deadlines.get(&timer.id).map(|d| d.saturating_duration_since(Instant::now()).as_millis() as u64).unwrap_or(0)
    }
    pub fn snapshot(&self) -> Document {
        let mut doc = self.doc.clone();
        let wall = now_ms();
        doc.stopwatch.elapsed_ms = self.stopwatch_elapsed();
        if self.doc.stopwatch.running {
            crate::stats::add_span(&mut doc.stopwatch.focus_slots, wall, doc.stopwatch.elapsed_ms.saturating_sub(self.sampled_stopwatch_ms));
        }
        for timer in &mut doc.countdowns {
            let remaining = self.remaining(timer);
            if !timer.paused && !timer.finished {
                let previous = self.sampled_remaining.get(&timer.id).copied().unwrap_or(timer.remaining_ms);
                crate::stats::add_span(&mut timer.focus_slots, self.sample_end(&timer.id, wall), previous.saturating_sub(remaining));
            }
            timer.remaining_ms = remaining;
            if !timer.paused && !timer.finished { timer.ends_at_ms = self.sample_end(&timer.id, wall).saturating_add(remaining as i64); }
        }
        doc
    }
    fn sample_end(&self, id: &str, wall: i64) -> i64 {
        let overdue = self.deadlines.get(id).map(|deadline| Instant::now().saturating_duration_since(*deadline).as_millis() as i64).unwrap_or(0);
        wall.saturating_sub(overdue)
    }
    fn sample_focus(&mut self) {
        let wall = now_ms();
        let samples: Vec<_> = self.doc.countdowns.iter().filter(|timer| !timer.finished && !timer.paused).map(|timer| {
            let remaining = self.remaining(timer);
            let previous = self.sampled_remaining.get(&timer.id).copied().unwrap_or(timer.remaining_ms);
            (timer.id.clone(), remaining, previous.saturating_sub(remaining), self.sample_end(&timer.id, wall))
        }).collect();
        for (id, remaining, amount, end) in samples {
            if let Some(timer) = self.doc.countdowns.iter_mut().find(|timer| timer.id == id) {
                crate::stats::add_span(&mut timer.focus_slots, end, amount);
            }
            self.sampled_remaining.insert(id, remaining);
        }
        self.sampled_remaining.retain(|id, _| self.doc.countdowns.iter().any(|timer| &timer.id == id && !timer.finished && !timer.paused));
        let elapsed = self.stopwatch_elapsed();
        if self.doc.stopwatch.running {
            crate::stats::add_span(&mut self.doc.stopwatch.focus_slots, wall, elapsed.saturating_sub(self.sampled_stopwatch_ms));
        }
        self.sampled_stopwatch_ms = elapsed;
    }
    pub fn focus_timer(&self) -> Option<&Countdown> {
        self.doc.countdowns.iter().find(|timer| timer.id == self.doc.settings.focus_timer_id)
    }
    pub fn cycle_remaining(&self, id: &str) -> Option<u64> {
        self.cycle_waits.get(id).map(|deadline| deadline.saturating_duration_since(Instant::now()).as_millis() as u64)
    }
    pub fn take_cycle_restarts(&mut self) -> Vec<String> { std::mem::take(&mut self.cycle_restarted) }
    pub fn cancel_cycle(&mut self, id: &str) { self.cycle_waits.remove(id); }
    pub fn set_auto_cycle(&mut self, enabled: bool) {
        self.doc.settings.auto_cycle = enabled;
        if !enabled { self.cycle_waits.clear(); }
    }
    pub fn upcoming_alarms(&self, now: i64, seconds: i64) -> Vec<&Alarm> {
        let until = now.saturating_add(seconds.max(0));
        let mut alarms: Vec<_> = self.doc.alarms.iter().filter(|alarm|
            alarm.enabled && alarm.fire_at >= now && alarm.fire_at <= until).collect();
        alarms.sort_by_key(|alarm| alarm.fire_at);
        alarms
    }
    pub fn stopwatch_elapsed(&self) -> u64 {
        self.doc.stopwatch.elapsed_ms.saturating_add(self.stopwatch_started
            .map(|start| start.elapsed().as_millis().min(u64::MAX as u128) as u64).unwrap_or(0)).min(359_999_000)
    }
    pub fn has_running_countdown(&self) -> bool {
        self.doc.countdowns.iter().any(|timer| !timer.paused && !timer.finished && self.remaining(timer) > 0)
    }
    pub fn mini_uses_stopwatch(&self) -> bool {
        self.doc.settings.stopwatch_enabled && (self.doc.stopwatch.running
            || (!self.has_running_countdown() && self.doc.settings.active_focus_mode == 1))
    }
    fn pause_stopwatch(&mut self) {
        if self.doc.stopwatch.running {
            self.doc.stopwatch.elapsed_ms = self.stopwatch_elapsed();
            self.doc.stopwatch.running = false; self.stopwatch_started = None;
        }
    }
    fn pause_countdowns(&mut self) {
        // Starting a stopwatch also pauses a pending next round, preserving exclusivity.
        for (id, _) in self.cycle_waits.drain() {
            if let Some(timer) = self.doc.countdowns.iter_mut().find(|timer| timer.id == id) {
                timer.finished = false; timer.paused = true; timer.remaining_ms = timer.original_ms;
            }
        }
        let running: Vec<_> = self.doc.countdowns.iter().enumerate()
            .filter(|(_, timer)| !timer.paused && !timer.finished)
            .map(|(index, timer)| (index, self.remaining(timer))).collect();
        for (index, remaining) in running {
            // Timers already at zero stay due so tick() can deliver their alert.
            if remaining == 0 { continue; }
            let timer = &mut self.doc.countdowns[index];
            timer.paused = true; timer.remaining_ms = remaining;
            self.deadlines.remove(&timer.id);
        }
    }
    pub fn stopwatch_action(&mut self, action: &str) {
        self.sample_focus();
        let elapsed = self.stopwatch_elapsed();
        match action {
            "toggle" => {
                if self.doc.stopwatch.running {
                    self.doc.stopwatch.elapsed_ms = elapsed; self.stopwatch_started = None;
                    self.doc.stopwatch.running = false;
                } else if elapsed < 359_999_000 {
                    self.pause_countdowns(); self.doc.settings.active_focus_mode = 1;
                    self.stopwatch_started = Some(Instant::now()); self.doc.stopwatch.running = true;
                }
            }
            "reset" | "finish" => {
                if action == "finish" {
                    self.doc.total_focus_ms = self.doc.total_focus_ms.saturating_add(elapsed);
                    if elapsed > 0 { crate::stats::record_completion(&mut self.doc.focus_days, &self.doc.stopwatch.focus_slots, now_ms()); }
                }
                self.doc.stopwatch = Stopwatch::default(); self.stopwatch_started = None; self.sampled_stopwatch_ms = 0;
            }
            _ => {}
        }
    }
    pub fn start(&mut self, seconds: u64, label: &str, selected_sound: i32) -> Result<(), String> {
        if seconds == 0 || seconds > 360000 { return Err("请设置大于零、最多 99 小时的倒计时。".into()); }
        if self.doc.countdowns.len() >= 100 { return Err("最多同时保留 100 项倒计时，请先移除已完成项目。".into()); }
        self.sample_focus(); self.pause_stopwatch(); self.doc.settings.active_focus_mode = 0;
        let ms = seconds * 1000;
        let id = new_id();
        self.deadlines.insert(id.clone(), Instant::now() + Duration::from_millis(ms));
        self.sampled_remaining.insert(id.clone(), ms);
        self.doc.settings.focus_timer_id = id.clone();
        self.doc.settings.focus_duration_seconds = seconds;
        remember_name(&mut self.doc.countdown_names, label);
        self.doc.countdowns.push(Countdown { focus_slots: vec![], id, label: text(label), original_ms: ms, remaining_ms: ms,
            ends_at_ms: now_ms() + ms as i64, paused: false, finished: false, sound: sound(selected_sound) });
        Ok(())
    }
    pub fn timer_action(&mut self, id: &str, action: &str) {
        self.sample_focus();
        let Some(index) = self.doc.countdowns.iter().position(|t| t.id == id) else { return; };
        let waiting = self.cycle_waits.remove(id).is_some();
        if waiting && action == "toggle" {
            let timer = &mut self.doc.countdowns[index];
            timer.finished = false; timer.paused = true; timer.remaining_ms = timer.original_ms;
            return;
        }
        let timer = &self.doc.countdowns[index];
        let will_run = action == "toggle" && timer.paused && !timer.finished || action == "reset" && !timer.paused;
        if will_run {
            self.pause_stopwatch(); self.doc.settings.active_focus_mode = 0;
            self.doc.settings.focus_timer_id = id.into();
        }
        let remaining = self.remaining(&self.doc.countdowns[index]);
        let timer = &mut self.doc.countdowns[index];
        match action {
            "remove" => {
                self.doc.countdowns.remove(index); self.deadlines.remove(id); self.sampled_remaining.remove(id);
                if self.doc.settings.focus_timer_id == id { self.doc.settings.focus_timer_id.clear(); }
            }
            "toggle" if !timer.finished => {
                timer.paused = !timer.paused;
                timer.remaining_ms = remaining;
                if timer.paused { self.deadlines.remove(id); self.sampled_remaining.remove(id); }
                else { self.sampled_remaining.insert(id.into(), remaining); self.deadlines.insert(id.into(), Instant::now() + Duration::from_millis(remaining)); timer.ends_at_ms = now_ms() + remaining as i64; }
            }
            "reset" => {
                timer.focus_slots.clear(); self.sampled_remaining.insert(id.into(), timer.original_ms);
                timer.finished = false;
                timer.remaining_ms = timer.original_ms;
                if !timer.paused { self.deadlines.insert(id.into(), Instant::now() + Duration::from_millis(timer.original_ms)); }
                timer.ends_at_ms = now_ms() + timer.original_ms as i64;
            }
            _ => {}
        }
    }
    pub fn toggle_alarm(&mut self, id: &str) {
        if let Some(alarm) = self.doc.alarms.iter_mut().find(|a| a.id == id) {
            alarm.enabled = !alarm.enabled;
            if alarm.enabled && alarm.days != 0 && alarm.fire_at <= Local::now().timestamp() {
                if let Some(next) = next_occurrence(Local::now().timestamp(), alarm.hour, alarm.minute, alarm.days) { alarm.fire_at = next; }
            }
            if alarm.warn_before && Local::now().timestamp() >= alarm.fire_at - 300 { self.warned.insert((alarm.id.clone(), alarm.fire_at)); }
        }
    }
    pub fn put_alarm(&mut self, alarm: Alarm) {
        remember_name(&mut self.doc.alarm_names, &alarm.label);
        if alarm.warn_before && Local::now().timestamp() >= alarm.fire_at - 300 { self.warned.insert((alarm.id.clone(), alarm.fire_at)); }
        if let Some(index) = self.doc.alarms.iter().position(|a| a.id == alarm.id) { self.doc.alarms[index] = alarm; }
        else { self.doc.alarms.push(alarm); }
    }
    pub fn remove_alarm(&mut self, id: &str) { self.doc.alarms.retain(|a| a.id != id); }
    pub fn tick(&mut self) -> (Vec<Alert>, bool) {
        self.sample_focus();
        let mut alerts = vec![];
        let mut changed = false;
        if !self.doc.settings.auto_cycle { self.cycle_waits.clear(); }
        let ready: Vec<String> = self.cycle_waits.iter().filter(|(_, deadline)| Instant::now() >= **deadline).map(|(id, _)| id.clone()).collect();
        for id in ready {
            self.cycle_waits.remove(&id);
            if !self.doc.stopwatch.running && self.doc.countdowns.iter().any(|timer| timer.id == id && timer.finished && !timer.paused) {
                self.timer_action(&id, "reset"); self.cycle_restarted.push(id); changed = true;
            }
        }
        if self.doc.stopwatch.running && self.stopwatch_elapsed() >= 359_999_000 {
            self.stopwatch_action("toggle"); changed = true;
        }
        let due: Vec<String> = self.doc.countdowns.iter().filter(|t| !t.paused && !t.finished && self.remaining(t) == 0)
            .map(|t| t.id.clone()).collect();
        for id in due {
            if let Some(timer) = self.doc.countdowns.iter_mut().find(|t| t.id == id) {
                self.doc.total_focus_ms = self.doc.total_focus_ms.saturating_add(timer.original_ms);
                crate::stats::record_completion(&mut self.doc.focus_days, &std::mem::take(&mut timer.focus_slots), timer.ends_at_ms);
                self.sampled_remaining.remove(&id);
                timer.finished = true;
                timer.remaining_ms = 0;
                self.deadlines.remove(&id);
                if self.doc.settings.auto_cycle {
                    self.cycle_waits.insert(id.clone(), Instant::now() + Duration::from_secs(self.doc.settings.cycle_interval_seconds.min(3600).max(1)));
                }
                alerts.push(Alert { token: new_id(), id, title: "时间到了".into(), label: if timer.label.is_empty() { "这一轮专注已完成。".into() } else { timer.label.clone() },
                    sound: timer.sound, warning: false, countdown: true, clock_chime: false, default_label: timer.label.is_empty() });
                changed = true;
            }
        }
        let now = Local::now().timestamp();
        if let Some(alert) = self.clock_chime(now) { alerts.push(alert); }
        let mut remove = vec![];
        let mut missed = 0;
        for alarm in &mut self.doc.alarms {
            if !alarm.enabled { continue; }
            if alarm.warn_before && now >= alarm.fire_at - 300 && now < alarm.fire_at && self.warned.insert((alarm.id.clone(), alarm.fire_at)) {
                alerts.push(Alert { token: new_id(), id: alarm.id.clone(), title: "还有 5 分钟".into(), label: alarm.label.clone(),
                    sound: if alarm.sound == 0 { 0 } else { 1 }, warning: true, countdown: false, clock_chime: false, default_label: false });
            }
            if now < alarm.fire_at { continue; }
            let occurrence = if alarm.days == 0 { alarm.fire_at }
                else { previous_occurrence(now, alarm.hour, alarm.minute, alarm.days).unwrap_or(alarm.fire_at) };
            if alarm.days == 0 { remove.push(alarm.id.clone()); }
            else if let Some(next) = next_occurrence(now, alarm.hour, alarm.minute, alarm.days) { alarm.fire_at = next; }
            if now - occurrence <= 300 {
                alerts.push(Alert { token: new_id(), id: alarm.id.clone(), title: "闹钟提醒".into(), label: if alarm.label.is_empty() { "计划的时间到了。".into() } else { alarm.label.clone() },
                    sound: alarm.sound, warning: false, countdown: false, clock_chime: false, default_label: alarm.label.is_empty() });
            } else { missed += 1; }
            changed = true;
        }
        self.doc.alarms.retain(|a| !remove.contains(&a.id));
        self.warned.retain(|(id, fire)| self.doc.alarms.iter().any(|a| &a.id == id && a.fire_at == *fire));
        if missed > 0 { self.message = format!("离开期间错过了 {missed} 项闹钟，已更新到下一次计划。"); }
        (alerts, changed)
    }
    fn clock_chime(&mut self, now: i64) -> Option<Alert> {
        let previous = std::mem::replace(&mut self.clock_checked_at, now);
        // Do not replay reports from time spent closed, asleep, or after a clock jump.
        if now <= previous || now.saturating_sub(previous) > 120 { return None; }
        let local = Local.timestamp_opt(now, 0).single()?;
        let boundary = now - i64::from(local.second());
        if boundary <= previous || boundary == self.last_chime_boundary { return None; }
        let title = match local.minute() {
            0 if self.doc.settings.hourly_chime => "整点报时",
            30 if self.doc.settings.half_hour_chime => "半点报时",
            _ => return None,
        };
        self.last_chime_boundary = boundary;
        Some(Alert { token: new_id(), id: format!("clock-{boundary}"), title: title.into(),
            label: format!("{:02}:{:02}", local.hour(), local.minute()), sound: self.doc.settings.default_sound,
            warning: false, countdown: false, clock_chime: true, default_label: false })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine { Engine::new(Document::default(), String::new()) }

    fn boundary(hour: u32, minute: u32) -> i64 {
        Local::now().date_naive().and_hms_opt(hour, minute, 0).unwrap()
            .and_local_timezone(Local).earliest().unwrap().timestamp()
    }

    #[test]
    fn chime_boundary_is_once_only_and_uses_selected_default_sound() {
        let mut state = engine(); state.doc.settings.hourly_chime = true;
        state.doc.settings.default_sound = 3;
        let at = boundary(9, 0); state.clock_checked_at = at - 1;
        let alert = state.clock_chime(at).unwrap();
        assert_eq!(alert.title, "整点报时"); assert_eq!(alert.label, "09:00");
        assert_eq!(alert.sound, 3); assert!(!alert.countdown && !alert.warning && alert.clock_chime);
        for second in 0..60 { assert!(state.clock_chime(at + second).is_none()); }
        state.clock_checked_at = at - 1; assert!(state.clock_chime(at).is_none());
        state.clock_checked_at = at + 3600 - 1;
        assert!(state.clock_chime(at + 3600).is_some());
    }

    #[test]
    fn half_hour_and_full_hour_can_be_enabled_independently() {
        let mut state = engine(); let full = boundary(8, 0); let half = boundary(8, 30);
        state.clock_checked_at = full - 1; assert!(state.clock_chime(full).is_none());
        state.doc.settings.half_hour_chime = true;
        state.clock_checked_at = full - 1; assert!(state.clock_chime(full).is_none());
        state.clock_checked_at = half - 1;
        let alert = state.clock_chime(half + 2).unwrap();
        assert_eq!(alert.title, "半点报时"); assert_eq!(alert.sound, 0);
        state.doc.settings.half_hour_chime = false; state.doc.settings.hourly_chime = true;
        state.clock_checked_at = half + 3600 - 1;
        assert!(state.clock_chime(half + 3600).is_none());
        state.clock_checked_at = full + 3600 - 1;
        assert!(state.clock_chime(full + 3600).is_some());
    }

    #[test]
    fn chimes_do_not_replay_after_sleep_backward_clock_or_enabling_mid_minute() {
        let mut state = engine(); state.doc.settings.hourly_chime = true;
        let at = boundary(10, 0);
        state.clock_checked_at = at - 121; assert!(state.clock_chime(at).is_none());
        state.clock_checked_at = at + 5; assert!(state.clock_chime(at).is_none());
        state.clock_checked_at = at + 5; assert!(state.clock_chime(at + 6).is_none());
        state.clock_checked_at = at - 1; state.doc.settings.hourly_chime = false;
        assert!(state.clock_chime(at).is_none());
        state.doc.settings.hourly_chime = true; assert!(state.clock_chime(at + 1).is_none());
    }

    #[test]
    fn countdown_controls_keep_original_duration() {
        let mut engine = engine(); engine.start(60, "focus", 0).unwrap();
        let id = engine.doc.settings.focus_timer_id.clone();
        assert_eq!(engine.focus_timer().unwrap().original_ms, 60_000);
        engine.timer_action(&id, "toggle");
        assert!(engine.focus_timer().unwrap().paused);
        engine.timer_action(&id, "reset");
        assert_eq!(engine.remaining(engine.focus_timer().unwrap()), 60_000);
        engine.timer_action(&id, "toggle");
        assert!(!engine.focus_timer().unwrap().paused);
    }

    #[test]
    fn removing_single_focus_returns_idle_and_keeps_other_timers() {
        let mut engine = engine(); engine.start(30, "older", 0).unwrap();
        engine.start(60, "current", 0).unwrap(); engine.doc.settings.single_focus = true;
        let id = engine.doc.settings.focus_timer_id.clone(); engine.timer_action(&id, "remove");
        assert!(engine.focus_timer().is_none());
        assert_eq!(engine.doc.countdowns.len(), 1);
        assert_eq!(engine.doc.countdowns[0].label, "older");
        assert!(engine.remaining(&engine.doc.countdowns[0]) > 0);
    }

    #[test]
    fn completed_countdown_is_counted_once() {
        let mut engine = engine(); engine.start(1, "", 0).unwrap();
        engine.deadlines.insert(engine.doc.settings.focus_timer_id.clone(), Instant::now() - Duration::from_secs(1));
        let (alerts, changed) = engine.tick(); assert!(changed); assert_eq!(alerts.len(), 1);
        assert_eq!(engine.doc.total_focus_ms, 1000);
        assert!(engine.tick().0.is_empty()); assert_eq!(engine.doc.total_focus_ms, 1000);
    }

    #[test]
    fn stopwatch_pause_resume_finish_counts_only_active_time_once() {
        let mut engine = engine(); engine.doc.stopwatch.elapsed_ms = 2500;
        engine.stopwatch_action("toggle");
        engine.stopwatch_started = Some(Instant::now() - Duration::from_millis(7500));
        let snapshot = engine.snapshot(); assert!(snapshot.stopwatch.elapsed_ms >= 10_000);
        assert_eq!(engine.doc.total_focus_ms, 0);
        engine.stopwatch_action("toggle"); let paused = engine.stopwatch_elapsed();
        assert_eq!(paused, engine.stopwatch_elapsed()); assert!(!engine.doc.stopwatch.running);
        engine.stopwatch_action("finish"); assert_eq!(engine.doc.total_focus_ms, paused);
        engine.stopwatch_action("finish"); assert_eq!(engine.doc.total_focus_ms, paused);
        assert_eq!(engine.stopwatch_elapsed(), 0);
    }

    #[test]
    fn stopwatch_reset_discards_unfinished_time() {
        let mut engine = engine(); engine.doc.total_focus_ms = 42_000;
        engine.doc.stopwatch.elapsed_ms = 12_000; engine.stopwatch_action("toggle");
        engine.stopwatch_action("reset"); assert_eq!(engine.stopwatch_elapsed(), 0);
        assert!(!engine.doc.stopwatch.running); assert_eq!(engine.doc.total_focus_ms, 42_000);
    }

    #[test]
    fn restored_stopwatch_is_paused_and_new_settings_roundtrip() {
        let mut engine = engine(); engine.doc.settings.single_focus = true;
        engine.doc.settings.stopwatch_enabled = true; engine.doc.settings.selected_tab = 4;
        engine.doc.stopwatch.elapsed_ms = 17_000; engine.stopwatch_action("toggle");
        let bytes = serde_json::to_vec(&engine.snapshot()).unwrap();
        let restored = Engine::new(parse(&bytes).unwrap(), String::new());
        assert!(restored.doc.settings.single_focus); assert!(restored.doc.settings.stopwatch_enabled);
        assert_eq!(restored.doc.settings.selected_tab, 4);
        assert!(!restored.doc.stopwatch.running); assert!(restored.stopwatch_elapsed() >= 17_000);
    }

    #[test]
    fn older_backup_defaults_and_page_ids_stay_compatible() {
        let defaults = Settings::default();
        assert!(!defaults.single_focus); assert!(!defaults.stopwatch_enabled);
        assert_eq!(defaults.active_focus_mode, 0);
        let doc = parse(br#"{"format":"TIMO.Native.v1","settings":{"selectedTab":3},"alarms":[],"countdowns":[]}"#).unwrap();
        assert!(!doc.settings.single_focus); assert!(!doc.settings.stopwatch_enabled);
        assert_eq!(doc.settings.selected_tab, 3); assert_eq!(doc.stopwatch.elapsed_ms, 0);
        let mut doc = doc; doc.settings.selected_tab = 4;
        assert_eq!(sanitize(doc).settings.selected_tab, 0);
    }

    #[test]
    fn starting_stopwatch_pauses_all_running_countdowns_and_keeps_progress() {
        let mut engine = engine();
        engine.start(60, "first", 0).unwrap(); engine.start(120, "second", 0).unwrap();
        let first = engine.doc.countdowns[0].id.clone();
        engine.deadlines.insert(first.clone(), Instant::now() + Duration::from_secs(30));
        let second = engine.doc.countdowns[1].id.clone();
        engine.timer_action(&second, "toggle");
        let second_remaining = engine.remaining(&engine.doc.countdowns[1]);
        engine.stopwatch_action("toggle");
        assert!(engine.doc.stopwatch.running); assert!(!engine.has_running_countdown());
        assert!(engine.doc.countdowns.iter().all(|timer| timer.paused));
        assert!((29_000..=30_000).contains(&engine.remaining(&engine.doc.countdowns[0])));
        assert_eq!(engine.remaining(&engine.doc.countdowns[1]), second_remaining);
        assert!(engine.deadlines.is_empty()); assert_eq!(engine.doc.settings.active_focus_mode, 1);
    }

    #[test]
    fn starting_countdown_pauses_stopwatch_without_discarding_elapsed_time() {
        let mut engine = engine(); engine.doc.stopwatch.elapsed_ms = 12_000;
        engine.stopwatch_action("toggle");
        engine.stopwatch_started = Some(Instant::now() - Duration::from_secs(3));
        engine.start(60, "focus", 0).unwrap();
        assert!(!engine.doc.stopwatch.running); assert!(engine.stopwatch_started.is_none());
        assert!(engine.stopwatch_elapsed() >= 15_000); assert!(engine.has_running_countdown());
        assert_eq!(engine.doc.settings.active_focus_mode, 0); assert_eq!(engine.doc.total_focus_ms, 0);
    }

    #[test]
    fn resuming_or_restarting_countdown_pauses_stopwatch() {
        let mut engine = engine(); engine.start(60, "focus", 0).unwrap();
        let id = engine.doc.settings.focus_timer_id.clone();
        engine.stopwatch_action("toggle"); engine.timer_action(&id, "toggle");
        assert!(engine.has_running_countdown()); assert!(!engine.doc.stopwatch.running);
        engine.stopwatch_action("toggle");
        engine.timer_action(&id, "reset");
        assert!(engine.doc.stopwatch.running); assert!(engine.doc.countdowns[0].paused);
        assert_eq!(engine.remaining(&engine.doc.countdowns[0]), 60_000);
        // A completed timer restarts running when reset, and must also pause the stopwatch.
        engine.doc.countdowns[0].finished = true; engine.doc.countdowns[0].paused = false;
        engine.timer_action(&id, "reset");
        assert!(!engine.doc.stopwatch.running); assert!(engine.has_running_countdown());
        assert_eq!(engine.doc.settings.focus_timer_id, id);
    }

    #[test]
    fn mini_uses_selected_stopwatch_when_idle_paused_or_running() {
        let mut engine = engine(); engine.doc.settings.stopwatch_enabled = true;
        engine.doc.settings.active_focus_mode = 1;
        assert!(engine.mini_uses_stopwatch());
        engine.stopwatch_action("toggle"); assert!(engine.mini_uses_stopwatch());
        engine.doc.settings.active_focus_mode = 0;
        assert!(engine.mini_uses_stopwatch()); // A running timer takes priority over navigation.
        engine.doc.settings.active_focus_mode = 1;
        engine.stopwatch_action("toggle"); assert!(engine.mini_uses_stopwatch());
        engine.start(60, "focus", 0).unwrap(); assert!(!engine.mini_uses_stopwatch());
        engine.doc.settings.active_focus_mode = 1;
        assert!(!engine.mini_uses_stopwatch()); // A running countdown still takes priority.
        let id = engine.doc.settings.focus_timer_id.clone(); engine.timer_action(&id, "toggle");
        assert!(engine.mini_uses_stopwatch());
        engine.doc.settings.stopwatch_enabled = false; assert!(!engine.mini_uses_stopwatch());
    }

    #[test]
    fn switching_to_stopwatch_does_not_lose_already_due_countdown_alert() {
        let mut engine = engine(); engine.start(1, "focus", 0).unwrap();
        engine.deadlines.insert(engine.doc.settings.focus_timer_id.clone(), Instant::now() - Duration::from_secs(1));
        engine.stopwatch_action("toggle"); let (alerts, changed) = engine.tick();
        assert!(changed); assert_eq!(alerts.len(), 1);
        assert!(engine.doc.stopwatch.running); assert_eq!(engine.doc.total_focus_ms, 1000);
    }

    #[test]
    fn mini_alarms_include_only_enabled_next_thirty_minutes_in_order() {
        let mut engine = engine(); let now = 2_000_000_000;
        for (offset, enabled) in [(1801, true), (1800, true), (60, false), (-1, true), (0, true)] {
            engine.doc.alarms.push(Alarm { id: new_id(), fire_at: now + offset, enabled, ..Alarm::default() });
        }
        let alarms = engine.upcoming_alarms(now, 1800);
        assert_eq!(alarms.len(), 2); assert_eq!(alarms[0].fire_at, now); assert_eq!(alarms[1].fire_at, now + 1800);
    }

    #[test]
    fn automatic_cycle_waits_then_restarts_without_double_counting() {
        let mut engine = engine(); engine.set_auto_cycle(true);
        engine.doc.settings.cycle_interval_seconds = 0;
        engine.start(5, "cycle", 0).unwrap(); let id = engine.doc.settings.focus_timer_id.clone();
        engine.deadlines.insert(id.clone(), Instant::now() - Duration::from_secs(1));
        let (alerts, changed) = engine.tick();
        assert!(changed); assert_eq!(alerts.len(), 1); assert!(!alerts[0].token.is_empty());
        assert!(engine.focus_timer().unwrap().finished); assert_eq!(engine.doc.total_focus_ms, 5000);
        assert!(engine.cycle_remaining(&id).unwrap() > 800);
        engine.cycle_waits.insert(id.clone(), Instant::now() - Duration::from_millis(1));
        let (alerts, changed) = engine.tick();
        assert!(changed); assert!(alerts.is_empty()); assert_eq!(engine.take_cycle_restarts(), vec![id]);
        assert!(!engine.focus_timer().unwrap().finished); assert!(engine.has_running_countdown());
        assert_eq!(engine.doc.total_focus_ms, 5000);
        assert_eq!(engine.doc.focus_days.iter().map(|day| day.rounds).sum::<u64>(), 1);
        assert_eq!(engine.doc.focus_days.iter().flat_map(|day| day.hours_ms).sum::<u64>(), 5000);
    }

    #[test]
    fn pausing_or_disabling_cycle_cancels_pending_restart() {
        let mut engine = engine(); engine.set_auto_cycle(true);
        engine.start(1, "cycle", 0).unwrap(); let id = engine.doc.settings.focus_timer_id.clone();
        engine.deadlines.insert(id.clone(), Instant::now() - Duration::from_secs(1)); engine.tick();
        engine.timer_action(&id, "toggle");
        assert!(engine.cycle_remaining(&id).is_none());
        let timer = engine.focus_timer().unwrap(); assert!(timer.paused); assert!(!timer.finished); assert_eq!(timer.remaining_ms, 1000);
        engine.timer_action(&id, "toggle");
        engine.deadlines.insert(id.clone(), Instant::now() - Duration::from_secs(1)); engine.tick();
        engine.set_auto_cycle(false); assert!(engine.cycle_remaining(&id).is_none());
        assert!(engine.focus_timer().unwrap().finished);
    }

    #[test]
    fn stopwatch_also_pauses_a_countdown_between_rounds() {
        let mut engine = engine(); engine.set_auto_cycle(true);
        engine.start(1, "cycle", 0).unwrap(); let id = engine.doc.settings.focus_timer_id.clone();
        engine.deadlines.insert(id.clone(), Instant::now() - Duration::from_secs(1)); engine.tick();
        engine.stopwatch_action("toggle");
        assert!(engine.doc.stopwatch.running); assert!(engine.cycle_remaining(&id).is_none());
        let timer = engine.focus_timer().unwrap(); assert!(timer.paused); assert!(!timer.finished);
        assert_eq!(engine.doc.total_focus_ms, 1000);
    }

    #[test]
    fn reminder_and_cycle_options_roundtrip_in_backup_and_validate_limits() {
        let mut document = Document::default();
        document.settings.quick_snooze_seconds = vec![1, 600, 9999, 42];
        document.settings.auto_cycle = true; document.settings.cycle_interval_seconds = 4000;
        let restored = parse(&serde_json::to_vec(&document).unwrap()).unwrap();
        assert_eq!(restored.settings.quick_snooze_seconds, vec![1, 600, 5999]);
        assert!(restored.settings.auto_cycle); assert_eq!(restored.settings.cycle_interval_seconds, 3600);
        let legacy = parse(br#"{"format":"TIMO.Native.v1","settings":{},"alarms":[],"countdowns":[]}"#).unwrap();
        assert!(!legacy.settings.auto_cycle);
        assert_eq!(legacy.settings.quick_snooze_seconds, vec![300, 600, 900]);
    }

    #[test]
    fn daily_countdown_history_waits_for_completion_and_keeps_paused_progress() {
        let mut engine = engine(); engine.start(60, "focus", 0).unwrap();
        let id = engine.doc.settings.focus_timer_id.clone();
        engine.deadlines.insert(id.clone(), Instant::now() + Duration::from_secs(30));
        engine.timer_action(&id, "toggle");
        let pending = engine.focus_timer().unwrap().focus_slots.iter().map(|slot| slot.milliseconds).sum::<u64>();
        assert!((29_950..=30_050).contains(&pending));
        assert!(engine.doc.focus_days.is_empty());
        engine.tick();
        assert_eq!(engine.focus_timer().unwrap().focus_slots.iter().map(|slot| slot.milliseconds).sum::<u64>(), pending);
        engine.timer_action(&id, "toggle");
        engine.deadlines.insert(id, Instant::now() - Duration::from_secs(1));
        engine.tick();
        let recorded = engine.doc.focus_days.iter().flat_map(|day| day.hours_ms).sum::<u64>();
        assert!((59_950..=60_000).contains(&recorded));
        assert_eq!(engine.doc.focus_days.iter().map(|day| day.rounds).sum::<u64>(), 1);
        engine.tick();
        assert_eq!(engine.doc.focus_days.iter().flat_map(|day| day.hours_ms).sum::<u64>(), recorded);
    }

    #[test]
    fn pending_focus_snapshot_restores_without_double_counting() {
        let mut engine = engine(); engine.start(60, "focus", 0).unwrap();
        let id = engine.doc.settings.focus_timer_id.clone();
        engine.deadlines.insert(id.clone(), Instant::now() + Duration::from_secs(30));
        let snapshot = engine.snapshot();
        assert!(engine.doc.countdowns[0].focus_slots.is_empty()); // Snapshot is a copy.
        assert!(engine.doc.focus_days.is_empty());
        let mut restored = Engine::new(parse(&serde_json::to_vec(&snapshot).unwrap()).unwrap(), String::new());
        restored.deadlines.insert(id, Instant::now() - Duration::from_secs(1));
        restored.tick();
        let recorded = restored.doc.focus_days.iter().flat_map(|day| day.hours_ms).sum::<u64>();
        assert!((59_950..=60_050).contains(&recorded));
        assert_eq!(restored.doc.focus_days.iter().map(|day| day.rounds).sum::<u64>(), 1);
        let mut reopened = Engine::new(parse(&serde_json::to_vec(&restored.snapshot()).unwrap()).unwrap(), String::new());
        reopened.tick();
        assert_eq!(reopened.doc.focus_days.iter().flat_map(|day| day.hours_ms).sum::<u64>(), recorded);
    }

    #[test]
    fn stopwatch_history_is_added_only_when_finished_and_reset_discards_it() {
        let mut engine = engine(); engine.stopwatch_action("toggle");
        engine.stopwatch_started = Some(Instant::now() - Duration::from_millis(1500));
        engine.stopwatch_action("toggle");
        assert!(engine.doc.focus_days.is_empty());
        engine.stopwatch_action("finish");
        let recorded = engine.doc.focus_days.iter().flat_map(|day| day.hours_ms).sum::<u64>();
        assert!((1500..=1550).contains(&recorded));
        assert_eq!(engine.doc.focus_days.iter().map(|day| day.rounds).sum::<u64>(), 1);
        engine.stopwatch_action("finish");
        engine.stopwatch_action("toggle");
        engine.stopwatch_started = Some(Instant::now() - Duration::from_secs(10));
        engine.stopwatch_action("reset");
        assert_eq!(engine.doc.focus_days.iter().flat_map(|day| day.hours_ms).sum::<u64>(), recorded);
    }

    #[test]
    fn daily_history_and_three_presets_roundtrip_and_old_default_is_ignored() {
        let mut engine = engine(); engine.start(1, "focus", 0).unwrap();
        engine.deadlines.insert(engine.doc.settings.focus_timer_id.clone(), Instant::now() - Duration::from_secs(1));
        engine.tick();
        engine.doc.settings.quick_snooze_seconds = vec![6, 900, 1200];
        let bytes = serde_json::to_vec(&engine.snapshot()).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("\"snoozeSeconds\""));
        let restored = parse(&bytes).unwrap();
        assert_eq!(restored.settings.quick_snooze_seconds, vec![6, 900, 1200]);
        assert_eq!(restored.focus_days.iter().flat_map(|day| day.hours_ms).sum::<u64>(), 1000);
        let legacy = parse(br#"{"format":"TIMO.Native.v1","settings":{"snoozeSeconds":42,"quickSnoozeSeconds":[6,900,1200]},"alarms":[],"countdowns":[]}"#).unwrap();
        assert_eq!(legacy.settings.quick_snooze_seconds, vec![6, 900, 1200]);
        assert!(legacy.focus_days.is_empty());
    }
}
