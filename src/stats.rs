//! Hourly active-time buckets. Only completed focus rounds enter daily history.
use chrono::{Local, NaiveDate, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FocusSlot { pub hour_start_ms: i64, pub milliseconds: u64 }

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FocusDay { pub date: String, pub hours_ms: [u64; 24], pub rounds: u64 }

pub fn add_span(slots: &mut Vec<FocusSlot>, end_ms: i64, milliseconds: u64) {
    let mut cursor = end_ms.saturating_sub(milliseconds.min(360_000_000) as i64);
    while cursor < end_ms {
        let Some(local) = Local.timestamp_millis_opt(cursor).single() else { break; };
        let within_hour = (local.minute() as i64 * 60 + local.second() as i64) * 1000 + local.timestamp_subsec_millis() as i64;
        let hour = cursor - within_hour;
        let next = end_ms.min(hour.saturating_add(3_600_000));
        if next <= cursor { break; }
        let amount = (next - cursor) as u64;
        if let Some(slot) = slots.iter_mut().find(|slot| slot.hour_start_ms == hour) {
            slot.milliseconds = slot.milliseconds.saturating_add(amount);
        } else { slots.push(FocusSlot { hour_start_ms: hour, milliseconds: amount }); }
        cursor = next;
    }
}

pub fn record_completion(days: &mut Vec<FocusDay>, slots: &[FocusSlot], ended_ms: i64) {
    if !slots.iter().any(|slot| slot.milliseconds > 0) { return; }
    for slot in slots {
        let Some(local) = Local.timestamp_millis_opt(slot.hour_start_ms).single() else { continue; };
        let date = local.date_naive().format("%Y-%m-%d").to_string();
        let index = if let Some(index) = days.iter().position(|day| day.date == date) { index }
            else { days.push(FocusDay { date, ..FocusDay::default() }); days.len() - 1 };
        let hour = local.hour() as usize;
        days[index].hours_ms[hour] = days[index].hours_ms[hour].saturating_add(slot.milliseconds);
    }
    if let Some(end) = Local.timestamp_millis_opt(ended_ms).single() {
        let date = end.date_naive().format("%Y-%m-%d").to_string();
        let index = if let Some(index) = days.iter().position(|day| day.date == date) { index }
            else { days.push(FocusDay { date, ..FocusDay::default() }); days.len() - 1 };
        days[index].rounds = days[index].rounds.saturating_add(1);
    }
    days.sort_by(|a, b| a.date.cmp(&b.date));
    if days.len() > 3650 { days.drain(..days.len() - 3650); }
}

pub fn sanitize_days(days: Vec<FocusDay>) -> Vec<FocusDay> {
    let mut result = BTreeMap::<String, FocusDay>::new();
    for day in days.into_iter().take(10_000) {
        if NaiveDate::parse_from_str(&day.date, "%Y-%m-%d").is_err() { continue; }
        let target = result.entry(day.date.clone()).or_insert_with(|| FocusDay { date: day.date, ..FocusDay::default() });
        for (target, value) in target.hours_ms.iter_mut().zip(day.hours_ms) { *target = target.saturating_add(value); }
        target.rounds = target.rounds.saturating_add(day.rounds);
    }
    result.into_values().rev().take(3650).collect::<Vec<_>>().into_iter().rev().collect()
}

pub fn sanitize_slots(slots: &mut Vec<FocusSlot>) {
    slots.retain(|slot| slot.milliseconds > 0 && Local.timestamp_millis_opt(slot.hour_start_ms).single().is_some());
    slots.truncate(4096);
    for slot in slots { slot.milliseconds = slot.milliseconds.min(360_000_000); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_round_splits_at_midnight_without_inventing_pause_time() {
        let end = Local.with_ymd_and_hms(2026, 10, 17, 0, 10, 0).earliest().unwrap().timestamp_millis();
        let mut slots = vec![];
        add_span(&mut slots, end - 20 * 60_000, 10 * 60_000); // 23:40–23:50
        add_span(&mut slots, end, 15 * 60_000); // 23:55–00:10; paused gap excluded
        let mut days = vec![]; record_completion(&mut days, &slots, end);
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].hours_ms[23], 15 * 60_000);
        assert_eq!(days[1].hours_ms[0], 10 * 60_000);
        assert_eq!(days[1].rounds, 1);
    }
    #[test]
    fn daily_history_merges_duplicates_and_preserves_backup_values() {
        let mut day = FocusDay { date: "2026-10-17".into(), rounds: 1, ..FocusDay::default() };
        day.hours_ms[10] = 1500;
        let bytes = serde_json::to_vec(&vec![day.clone(), day]).unwrap();
        let merged = sanitize_days(serde_json::from_slice(&bytes).unwrap());
        assert_eq!(merged.len(), 1); assert_eq!(merged[0].hours_ms[10], 3000); assert_eq!(merged[0].rounds, 2);
    }
}
