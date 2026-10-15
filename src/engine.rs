//! Pure application state and scheduling; no UI toolkit types or window calls.
use crate::data::*;
use chrono::Local;
use std::{collections::{HashMap, HashSet}, time::{Duration, Instant}};

#[derive(Clone)]
pub struct Alert {
    pub id: String,
    pub title: String,
    pub label: String,
    pub sound: i32,
    pub warning: bool,
    pub countdown: bool,
    pub default_label: bool,
}

pub struct Engine {
    pub doc: Document,
    deadlines: HashMap<String, Instant>,
    warned: HashSet<(String, i64)>,
    pub message: String,
}

impl Engine {
    pub fn new(mut doc: Document, mut message: String) -> Self {
        if !doc.focus_stats_initialized {
            doc.total_focus_ms = doc.countdowns.iter().filter(|t| t.finished)
                .fold(0u64, |total, t| total.saturating_add(t.original_ms));
            doc.focus_stats_initialized = true;
        }
        let now = now_ms();
        let mut missed = 0;
        for timer in &mut doc.countdowns {
            if !timer.finished && !timer.paused && now.saturating_sub(timer.ends_at_ms) > 300_000 {
                timer.finished = true; timer.remaining_ms = 0; missed += 1;
                doc.total_focus_ms = doc.total_focus_ms.saturating_add(timer.original_ms);
            }
        }
        if missed > 0 { message = format!("离开期间有 {missed} 项倒计时已结束。"); }
        let deadlines = doc.countdowns.iter().filter(|t| !t.paused && !t.finished).map(|t| {
            (t.id.clone(), Instant::now() + Duration::from_millis((t.ends_at_ms.saturating_sub(now).max(0) as u64).min(t.original_ms)))
        }).collect();
        let warned = doc.alarms.iter().filter(|a| a.warn_before && now / 1000 >= a.fire_at - 300)
            .map(|a| (a.id.clone(), a.fire_at)).collect();
        Self { doc, deadlines, warned, message }
    }
    pub fn remaining(&self, timer: &Countdown) -> u64 {
        if timer.finished { return 0; }
        if timer.paused { return timer.remaining_ms; }
        self.deadlines.get(&timer.id).map(|d| d.saturating_duration_since(Instant::now()).as_millis() as u64).unwrap_or(0)
    }
    pub fn snapshot(&self) -> Document {
        let mut doc = self.doc.clone();
        for timer in &mut doc.countdowns {
            timer.remaining_ms = self.remaining(timer);
            if !timer.paused && !timer.finished { timer.ends_at_ms = now_ms() + timer.remaining_ms as i64; }
        }
        doc
    }
    pub fn start(&mut self, seconds: u64, label: &str, selected_sound: i32) -> Result<(), String> {
        if seconds == 0 || seconds > 360000 { return Err("请设置大于零、最多 99 小时的倒计时。".into()); }
        if self.doc.countdowns.len() >= 100 { return Err("最多同时保留 100 项倒计时，请先移除已完成项目。".into()); }
        let ms = seconds * 1000;
        let id = new_id();
        self.deadlines.insert(id.clone(), Instant::now() + Duration::from_millis(ms));
        self.doc.countdowns.push(Countdown { id, label: text(label), original_ms: ms, remaining_ms: ms,
            ends_at_ms: now_ms() + ms as i64, paused: false, finished: false, sound: sound(selected_sound) });
        Ok(())
    }
    pub fn timer_action(&mut self, id: &str, action: &str) {
        let Some(index) = self.doc.countdowns.iter().position(|t| t.id == id) else { return; };
        let remaining = self.remaining(&self.doc.countdowns[index]);
        let timer = &mut self.doc.countdowns[index];
        match action {
            "remove" => { self.doc.countdowns.remove(index); self.deadlines.remove(id); }
            "toggle" if !timer.finished => {
                timer.paused = !timer.paused;
                timer.remaining_ms = remaining;
                if timer.paused { self.deadlines.remove(id); }
                else { self.deadlines.insert(id.into(), Instant::now() + Duration::from_millis(remaining)); timer.ends_at_ms = now_ms() + remaining as i64; }
            }
            "reset" => {
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
        if alarm.warn_before && Local::now().timestamp() >= alarm.fire_at - 300 { self.warned.insert((alarm.id.clone(), alarm.fire_at)); }
        if let Some(index) = self.doc.alarms.iter().position(|a| a.id == alarm.id) { self.doc.alarms[index] = alarm; }
        else { self.doc.alarms.push(alarm); }
    }
    pub fn remove_alarm(&mut self, id: &str) { self.doc.alarms.retain(|a| a.id != id); }
    pub fn tick(&mut self) -> (Vec<Alert>, bool) {
        let mut alerts = vec![];
        let mut changed = false;
        let due: Vec<String> = self.doc.countdowns.iter().filter(|t| !t.paused && !t.finished && self.remaining(t) == 0)
            .map(|t| t.id.clone()).collect();
        for id in due {
            if let Some(timer) = self.doc.countdowns.iter_mut().find(|t| t.id == id) {
                self.doc.total_focus_ms = self.doc.total_focus_ms.saturating_add(timer.original_ms);
                timer.finished = true;
                timer.remaining_ms = 0;
                self.deadlines.remove(&id);
                alerts.push(Alert { id, title: "时间到了".into(), label: if timer.label.is_empty() { "这一轮专注已完成。".into() } else { timer.label.clone() },
                    sound: timer.sound, warning: false, countdown: true, default_label: timer.label.is_empty() });
                changed = true;
            }
        }
        let now = Local::now().timestamp();
        let mut remove = vec![];
        let mut missed = 0;
        for alarm in &mut self.doc.alarms {
            if !alarm.enabled { continue; }
            if alarm.warn_before && now >= alarm.fire_at - 300 && now < alarm.fire_at && self.warned.insert((alarm.id.clone(), alarm.fire_at)) {
                alerts.push(Alert { id: alarm.id.clone(), title: "还有 5 分钟".into(), label: alarm.label.clone(),
                    sound: if alarm.sound == 0 { 0 } else { 1 }, warning: true, countdown: false, default_label: false });
            }
            if now < alarm.fire_at { continue; }
            let occurrence = if alarm.days == 0 { alarm.fire_at }
                else { previous_occurrence(now, alarm.hour, alarm.minute, alarm.days).unwrap_or(alarm.fire_at) };
            if alarm.days == 0 { remove.push(alarm.id.clone()); }
            else if let Some(next) = next_occurrence(now, alarm.hour, alarm.minute, alarm.days) { alarm.fire_at = next; }
            if now - occurrence <= 300 {
                alerts.push(Alert { id: alarm.id.clone(), title: "闹钟提醒".into(), label: if alarm.label.is_empty() { "计划的时间到了。".into() } else { alarm.label.clone() },
                    sound: alarm.sound, warning: false, countdown: false, default_label: alarm.label.is_empty() });
            } else { missed += 1; }
            changed = true;
        }
        self.doc.alarms.retain(|a| !remove.contains(&a.id));
        self.warned.retain(|(id, fire)| self.doc.alarms.iter().any(|a| &a.id == id && a.fire_at == *fire));
        if missed > 0 { self.message = format!("离开期间错过了 {missed} 项闹钟，已更新到下一次计划。"); }
        (alerts, changed)
    }
}
