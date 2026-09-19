use crate::parser::{ChainCommand, CompleteHealCall};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ClericSlot {
    pub number: u32,
    pub player: String,
    pub target: Option<String>,
    pub last_shout_ms: Option<u64>,
    pub last_actual_ms: Option<u64>,
    pub last_offset_seconds: Option<f64>,
}

impl ClericSlot {
    fn new(number: u32, player: String) -> Self {
        Self {
            number,
            player,
            target: None,
            last_shout_ms: None,
            last_actual_ms: None,
            last_offset_seconds: None,
        }
    }
}

#[derive(Debug, Default)]
struct VacatedSlot {
    old_number: Option<u32>,
    last_shout_ms: Option<u64>,
    last_actual_ms: Option<u64>,
    last_offset_seconds: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SlotFormat {
    Number,
    Letter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum WarningKind {
    #[default]
    Other,
    SlotTaken,
    WrongTarget,
    AutoTake,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TankKey {
    Main,
    Named(String),
}

#[derive(Debug, Clone)]
struct TankAssignment {
    name: String,
    from: u32,
    to: u32,
    interval_seconds: Option<f64>,
    running: bool,
    started_at_ms: Option<u64>,
    current_number: Option<u32>,
    is_off: bool,
    shout_sync: bool,
    armed: bool,
}

#[derive(Debug, Clone)]
pub struct ChainState {
    pub tank: Option<String>,
    pub off_tank: Option<String>,
    pub interval_seconds: f64,
    pub cast_time_seconds: f64,
    pub your_name: Option<String>,
    pub slots: BTreeMap<u32, ClericSlot>,
    pub skipped: HashSet<u32>,
    pub current_number: Option<u32>,
    pub running: bool,
    pub started_at_ms: Option<u64>,
    tanks: Vec<TankAssignment>,
    pub warning: Option<String>,
    pub warning_at_ms: Option<u64>,
    pub warning_urgent: bool,
    pub warning_kind: WarningKind,
    pub warning_speech: Option<String>,
    slot_format: SlotFormat,
    shout_sync: bool,
    armed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotSnapshot {
    pub number: u32,
    pub player: String,
    pub target: Option<String>,
    pub skipped: bool,
    pub is_you: bool,
    pub is_current: bool,
    pub is_next: bool,
    pub remaining_seconds: f64,
    pub progress: f64,
    pub last_shout_ms: Option<u64>,
    pub last_cast_ms: Option<u64>,
    pub cast_remaining_seconds: f64,
    pub cast_progress: f64,
    pub offset_seconds: Option<f64>,
    pub tank: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TankSnapshot {
    pub name: String,
    pub from: Option<u32>,
    pub to: Option<u32>,
    pub interval_seconds: f64,
    pub running: bool,
    pub started_at_ms: Option<u64>,
    pub current_number: Option<u32>,
    pub next_number: Option<u32>,
    pub beat_tick: Option<u64>,
    pub is_you: bool,
    pub armed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainSnapshot {
    pub tank: Option<String>,
    pub your_tank: Option<String>,
    pub interval_seconds: f64,
    pub cast_time_seconds: f64,
    pub your_name: Option<String>,
    pub current_number: Option<u32>,
    pub next_number: Option<u32>,
    pub you_are_next_in: Option<f64>,
    pub you_cast_in: Option<f64>,
    pub you_last_offset: Option<f64>,
    pub running: bool,
    pub armed: bool,
    pub started_at_ms: Option<u64>,
    pub beat_tick: Option<u64>,
    pub warning: Option<String>,
    pub warning_urgent: bool,
    pub warning_kind: WarningKind,
    pub warning_speech: Option<String>,
    pub tanks: Vec<TankSnapshot>,
    pub slots: Vec<SlotSnapshot>,
    pub slot_format: SlotFormat,
    pub now_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct Beat {
    pub current: u32,
    pub next: u32,
    pub ticks: u64,
    pub rotation: Vec<u32>,
    pub interval_ms: u64,
    pub start: u64,
}

impl ChainState {
    pub fn new(interval_seconds: f64, cast_time_seconds: f64) -> Self {
        Self {
            tank: None,
            off_tank: None,
            interval_seconds,
            cast_time_seconds,
            your_name: None,
            slots: BTreeMap::new(),
            skipped: HashSet::new(),
            current_number: None,
            running: false,
            started_at_ms: None,
            tanks: Vec::new(),
            warning: None,
            warning_at_ms: None,
            warning_urgent: false,
            warning_kind: WarningKind::Other,
            warning_speech: None,
            slot_format: SlotFormat::Number,
            shout_sync: false,
            armed: false,
        }
    }

    pub fn new_rampage(interval_seconds: f64, cast_time_seconds: f64) -> Self {
        let mut chain = Self::new(interval_seconds, cast_time_seconds);
        chain.slot_format = SlotFormat::Letter;
        chain
    }

    fn fmt_slot(&self, number: u32) -> String {
        match self.slot_format {
            SlotFormat::Number => format!("{number:03}"),
            SlotFormat::Letter => {
                let idx = number.saturating_sub(1).min(25);
                let letter = char::from_u32(b'A' as u32 + idx).unwrap_or('A');
                letter.to_string().repeat(3)
            }
        }
    }

    pub fn set_your_name(&mut self, name: String) {
        if self.your_name.as_deref() == Some(name.as_str()) {
            return;
        }
        let prev = self.your_name.replace(name.clone());
        for slot in self.slots.values_mut() {
            if slot.player == "You"
                || prev
                    .as_ref()
                    .is_some_and(|old| old.eq_ignore_ascii_case(&slot.player))
            {
                slot.player = name.clone();
            }
        }
    }

    pub fn apply_heal(&mut self, call: CompleteHealCall) {
        self.apply_heal_at(call, now_ms());
    }

    pub fn apply_heal_at(&mut self, call: CompleteHealCall, now: u64) {
        self.clear_stale_warning_at(now);
        let player = call.speaker;
        let is_you = call.is_you || self.is_you(&player);
        if self.occupied_by_other(call.number, &player) {
            let msg = self.slot_taken_message(call.number, &player, is_you);
            self.set_warning_at(msg, now, is_you, WarningKind::SlotTaken);
            return;
        }
        let already_on = self.slot_number_for_player(&player).is_some();
        let key = self.tank_key_for(call.number);
        let was_new = !self.slots.contains_key(&call.number);
        let was_running = self.clock_running(&key);
        let prev_current = self.running_current(&key, now);
        self.vacate_player(&player, call.number);
        let mut slot = ClericSlot::new(call.number, player);
        slot.target = if call.target.is_empty() {
            None
        } else {
            Some(call.target)
        };
        slot.last_shout_ms = Some(now);
        let target_name = slot.target.clone().unwrap_or_default();
        self.slots.insert(call.number, slot);
        if self.clock_armed(&key) {
            self.set_armed(&key, false);
            self.reanchor(key.clone(), call.number, now, 0);
        } else if was_running && self.rotation_for(&key).len() <= 1 {
            self.reanchor(key.clone(), call.number, now, 0);
        } else if was_running && self.shout_sync(&key) {
            if let Some(seconds) = self.infer_interval_for(&key) {
                self.apply_inferred_interval(&key, seconds);
            }
            self.reanchor(key.clone(), call.number, now, 0);
        } else if !was_running {
            if let Some(seconds) = self.infer_interval_for(&key) {
                self.apply_inferred_interval(&key, seconds);
                self.reanchor(key.clone(), call.number, now, 0);
                self.set_shout_sync(&key, true);
            } else {
                self.set_clock_current(key, Some(call.number));
            }
        } else if was_new {
            self.preserve_beat(key, now, prev_current);
        }
        self.record_actual(call.number, now);
        if already_on && was_running {
            if let Some(msg) = self.target_mismatch(call.number, &target_name, is_you) {
                self.set_warning_at(msg, now, is_you, WarningKind::WrongTarget);
                return;
            }
        }
        self.clear_warning();
    }

    fn record_actual(&mut self, number: u32, actual: u64) {
        let expected = self.nearest_expected(number, actual);
        if let Some(slot) = self.slots.get_mut(&number) {
            slot.last_actual_ms = Some(actual);
            if let Some(expected) = expected {
                slot.last_offset_seconds = Some(offset_seconds(expected, actual));
            }
        }
    }

    pub fn apply_command(&mut self, cmd: ChainCommand, speaker: String) -> Option<String> {
        self.apply_command_at(cmd, speaker, now_ms())
    }

    pub fn apply_command_at(
        &mut self,
        cmd: ChainCommand,
        speaker: String,
        now: u64,
    ) -> Option<String> {
        self.clear_stale_warning_at(now);
        match cmd {
            ChainCommand::MainTank { tank } => {
                self.tank = Some(tank);
                None
            }
            ChainCommand::OffTank { tank } => self.set_off_tank(tank),
            ChainCommand::Split { number } => self.split_at(number),
            ChainCommand::TankRange { tank, from, to } => self.assign_range(tank, from, to),
            ChainCommand::Untank { tank } => self.remove_tank(&tank),
            ChainCommand::Skip { number } => {
                match self.resolve_slot_number(number, &speaker, "skip") {
                    Ok(number) if self.slots.contains_key(&number) => {
                        let key = self.tank_key_for(number);
                        let prev = self.running_current(&key, now);
                        self.skipped.insert(number);
                        if self.clock_running(&key) {
                            let keep = if prev == Some(number) {
                                self.next_after_in(&key, number)
                            } else {
                                prev
                            };
                            self.preserve_beat(key, now, keep);
                        }
                        None
                    }
                    Ok(number) => Some(format!(
                        "Cannot skip {}: that slot is not set.",
                        self.fmt_slot(number)
                    )),
                    Err(warning) => Some(warning),
                }
            }
            ChainCommand::Back { number } => match self.resolve_slot_number(number, &speaker, "back")
            {
                Ok(number) => {
                    let key = self.tank_key_for(number);
                    let prev = self.running_current(&key, now);
                    self.skipped.remove(&number);
                    if self.clock_running(&key) {
                        self.preserve_beat(key, now, prev);
                    }
                    None
                }
                Err(warning) => Some(warning),
            },
            ChainCommand::ResetChain => {
                self.slots.clear();
                self.skipped.clear();
                self.current_number = None;
                self.running = false;
                self.armed = false;
                self.started_at_ms = None;
                self.shout_sync = false;
                for tank in &mut self.tanks {
                    tank.running = false;
                    tank.armed = false;
                    tank.started_at_ms = None;
                    tank.current_number = None;
                    tank.shout_sync = false;
                }
                self.clear_warning();
                None
            }
            ChainCommand::Take { number, player } => {
                self.take_slot(Some(number), player, speaker, now)
            }
            ChainCommand::TakeNext { player } => self.take_slot(None, player, speaker, now),
            ChainCommand::Move { from, to } => {
                if from == to {
                    return Some("Move needs two different numbers.".into());
                }
                let key = self.tank_key_for(from);
                let prev = self.running_current(&key, now);
                let Some(a) = self.slots.remove(&from) else {
                    return Some(format!(
                        "Cannot move {}: that slot is not set.",
                        self.fmt_slot(from)
                    ));
                };
                let Some(mut b) = self.slots.remove(&to) else {
                    self.slots.insert(from, a);
                    return Some(format!(
                        "Cannot move to {}: that slot is not set.",
                        self.fmt_slot(to)
                    ));
                };
                let mut a = a;
                a.number = to;
                b.number = from;
                let skip_a = self.skipped.remove(&from);
                let skip_b = self.skipped.remove(&to);
                if skip_a {
                    self.skipped.insert(to);
                }
                if skip_b {
                    self.skipped.insert(from);
                }
                self.remap_current(from, to);
                self.slots.insert(to, a);
                self.slots.insert(from, b);
                if self.clock_running(&key) {
                    let keep = match prev {
                        Some(n) if n == from => Some(to),
                        Some(n) if n == to => Some(from),
                        other => other,
                    };
                    self.preserve_beat(key, now, keep);
                }
                None
            }
            ChainCommand::ChainInterval { seconds, tank } => self.set_interval(seconds, tank),
            ChainCommand::StartChain { tank } => self.start_chains(tank, now),
            ChainCommand::EndChain { tank } => self.stop_chains(tank),
        }
    }

    pub fn set_warning(&mut self, warning: String) {
        self.set_warning_at(warning, now_ms(), false, WarningKind::Other);
    }

    fn set_warning_at(&mut self, warning: String, now: u64, urgent: bool, kind: WarningKind) {
        self.warning = Some(warning);
        self.warning_at_ms = Some(now);
        self.warning_urgent = urgent;
        self.warning_kind = kind;
        self.warning_speech = None;
    }

    fn clear_warning(&mut self) {
        self.warning = None;
        self.warning_at_ms = None;
        self.warning_urgent = false;
        self.warning_kind = WarningKind::Other;
        self.warning_speech = None;
    }

    fn occupied_by_other(&self, number: u32, speaker: &str) -> bool {
        self.slots
            .get(&number)
            .is_some_and(|slot| !self.same_player(&slot.player, speaker))
    }

    fn slot_taken_message(&self, number: u32, speaker: &str, is_you: bool) -> String {
        let slot = self.fmt_slot(number);
        if is_you {
            format!("{slot} is already taken.")
        } else {
            format!("{speaker}: {slot} is already taken.")
        }
    }

    fn take_slot(
        &mut self,
        number: Option<u32>,
        player: Option<String>,
        speaker: String,
        now: u64,
    ) -> Option<String> {
        let player = player.unwrap_or(speaker);
        let is_you = self.is_you(&player);
        let auto = number.is_none();
        let number = match number {
            Some(number) => number,
            None => {
                if let Some(existing) = self.slot_number_for_player(&player) {
                    self.announce_auto_take(
                        existing,
                        &player,
                        is_you,
                        now,
                        true,
                    );
                    return None;
                }
                match self.next_free_slot() {
                    Some(number) => number,
                    None => {
                        let msg = if self.slot_format == SlotFormat::Letter {
                            "No free rampage letters left."
                        } else {
                            "No free numbers left."
                        };
                        self.set_warning_at(msg.into(), now, is_you, WarningKind::AutoTake);
                        return None;
                    }
                }
            }
        };
        if self.occupied_by_other(number, &player) {
            let msg = self.slot_taken_message(number, &player, is_you);
            self.set_warning_at(msg, now, is_you, WarningKind::SlotTaken);
            return None;
        }
        let key = self.tank_key_for(number);
        let was_running = self.clock_running(&key);
        let prev = self.running_current(&key, now);
        let vacated = self.vacate_player(&player, number);
        let mut slot = ClericSlot::new(number, player.clone());
        slot.last_actual_ms = vacated.last_actual_ms;
        slot.last_offset_seconds = vacated.last_offset_seconds;
        if vacated.old_number == Some(number) {
            slot.last_shout_ms = vacated.last_shout_ms;
        }
        self.slots.insert(number, slot);
        if was_running {
            self.preserve_beat(key, now, prev);
        }
        if auto {
            self.announce_auto_take(number, &player, is_you, now, false);
        } else {
            self.clear_warning();
        }
        None
    }

    fn next_free_slot(&self) -> Option<u32> {
        let max = match self.slot_format {
            SlotFormat::Number => 999,
            SlotFormat::Letter => 26,
        };
        (1..=max).find(|number| !self.slots.contains_key(number))
    }

    fn spoken_slot(&self, number: u32) -> String {
        match self.slot_format {
            SlotFormat::Number => number.to_string(),
            SlotFormat::Letter => self.fmt_slot(number),
        }
    }

    fn announce_auto_take(
        &mut self,
        number: u32,
        player: &str,
        is_you: bool,
        now: u64,
        already: bool,
    ) {
        let slot = self.fmt_slot(number);
        let spoken = self.spoken_slot(number);
        let banner = if already {
            if is_you {
                format!("You already have {slot}.")
            } else {
                format!("{player} already has {slot}.")
            }
        } else if is_you {
            format!("You got {slot}.")
        } else {
            format!("{player} got {slot}.")
        };
        self.set_warning_at(banner, now, is_you, WarningKind::AutoTake);
        if is_you {
            let speech = if already {
                format!("You already have {spoken}")
            } else {
                format!("You got {spoken}")
            };
            self.warning_speech = Some(speech);
        }
    }

    fn same_player(&self, a: &str, b: &str) -> bool {
        a.eq_ignore_ascii_case(b) || (self.is_you(a) && self.is_you(b))
    }

    fn vacate_player(&mut self, speaker: &str, keep: u32) -> VacatedSlot {
        let old_numbers: Vec<u32> = self
            .slots
            .iter()
            .filter(|(_, slot)| self.same_player(&slot.player, speaker))
            .map(|(number, _)| *number)
            .collect();
        let mut vacated = VacatedSlot::default();
        for number in old_numbers {
            if number == keep {
                if let Some(slot) = self.slots.get(&number) {
                    vacated.old_number = Some(number);
                    vacated.last_shout_ms = slot.last_shout_ms;
                    vacated.last_actual_ms = slot.last_actual_ms;
                    vacated.last_offset_seconds = slot.last_offset_seconds;
                }
                continue;
            }
            if let Some(slot) = self.slots.remove(&number) {
                vacated.old_number = Some(number);
                vacated.last_shout_ms = slot.last_shout_ms;
                vacated.last_actual_ms = slot.last_actual_ms;
                vacated.last_offset_seconds = slot.last_offset_seconds;
            }
            self.skipped.remove(&number);
            self.clear_current_if(number);
        }
        vacated
    }

    fn clear_stale_warning_at(&mut self, now: u64) {
        if let Some(at) = self.warning_at_ms {
            if now.saturating_sub(at) > 20_000 {
                self.clear_warning();
            }
        }
    }

    pub fn snapshot(&self) -> ChainSnapshot {
        self.snapshot_at(now_ms())
    }

    pub(crate) fn snapshot_at(&self, now: u64) -> ChainSnapshot {
        let view = self.view_key();
        let focused = self.your_slot_number().is_some();
        let beat = self.beat_for(&view, now);
        let (current_number, next_number, beat_tick) = if let Some(beat) = &beat {
            (Some(beat.current), Some(beat.next), Some(beat.ticks))
        } else {
            let current = self.clock_current(&view);
            (
                current,
                current.and_then(|cur| self.next_after_in(&view, cur)),
                None,
            )
        };

        let you_cast_in = self.you_cast_in(now, &beat);
        let you_are_next_in = next_number.and_then(|num| {
            let slot = self.slots.get(&num)?;
            if !self.is_you(&slot.player) {
                return None;
            }
            you_cast_in
        });
        let you_last_offset = self.your_slot_number().and_then(|n| {
            self.slots.get(&n).and_then(|s| s.last_offset_seconds)
        });

        let slots = self
            .slots
            .values()
            .filter(|slot| !focused || self.tank_key_for(slot.number) == view)
            .map(|slot| {
                let key = self.tank_key_for(slot.number);
                let skipped = self.skipped.contains(&slot.number);
                let slot_beat = if focused {
                    beat.clone()
                } else {
                    self.beat_for(&key, now)
                };
                let (remaining, progress) = if skipped {
                    (0.0, 0.0)
                } else if let Some(slot_beat) = &slot_beat {
                    if slot_beat.rotation.len() <= 1 {
                        let origin = countdown_origin(
                            slot.last_shout_ms,
                            slot.last_actual_ms,
                            slot_beat.start,
                        );
                        let remaining = remaining_cast(Some(origin), self.cast_time_seconds, now);
                        let progress = if self.cast_time_seconds > 0.0 {
                            (remaining / self.cast_time_seconds).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        (remaining, progress)
                    } else {
                        let cycle = self.interval_for(&key) * slot_beat.rotation.len() as f64;
                        remaining_until_slot(slot.number, slot_beat, now, cycle)
                    }
                } else {
                    let remaining = remaining_cast(slot.last_shout_ms, self.cast_time_seconds, now);
                    let progress = if self.cast_time_seconds > 0.0 {
                        (remaining / self.cast_time_seconds).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    (remaining, progress)
                };
                let (is_current, is_next) = if focused {
                    (
                        current_number == Some(slot.number),
                        next_number == Some(slot.number) && current_number != Some(slot.number),
                    )
                } else if let Some(slot_beat) = &slot_beat {
                    (
                        slot_beat.current == slot.number,
                        slot_beat.next == slot.number && slot_beat.current != slot.number,
                    )
                } else {
                    let cur = self.clock_current(&key);
                    (
                        cur == Some(slot.number),
                        cur.and_then(|c| self.next_after_in(&key, c)) == Some(slot.number)
                            && cur != Some(slot.number),
                    )
                };
                let last_cast_ms = cast_started_ms(slot);
                let (cast_remaining, cast_progress) = match last_cast_ms {
                    Some(at) => {
                        let remaining = remaining_cast(Some(at), self.cast_time_seconds, now);
                        let progress = if self.cast_time_seconds > 0.0 {
                            (remaining / self.cast_time_seconds).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        (remaining, progress)
                    }
                    None => (0.0, 0.0),
                };
                SlotSnapshot {
                    number: slot.number,
                    player: slot.player.clone(),
                    target: slot.target.clone(),
                    skipped,
                    is_you: self.is_you(&slot.player),
                    is_current,
                    is_next,
                    remaining_seconds: remaining,
                    progress,
                    last_shout_ms: slot.last_shout_ms,
                    last_cast_ms,
                    cast_remaining_seconds: cast_remaining,
                    cast_progress,
                    offset_seconds: slot.last_offset_seconds,
                    tank: Some(self.display_name(&key)),
                }
            })
            .collect();

        let warning = self.warning.clone().and_then(|w| {
            if now.saturating_sub(self.warning_at_ms.unwrap_or(now)) > 20_000 {
                None
            } else {
                Some(w)
            }
        });
        let warning_urgent = warning.is_some() && self.warning_urgent;
        let warning_kind = if warning.is_some() {
            self.warning_kind
        } else {
            WarningKind::Other
        };
        let warning_speech = if warning.is_some() {
            self.warning_speech.clone()
        } else {
            None
        };
        let tanks = self.tank_snapshots(now);
        let your_tank = self.your_slot_number().map(|_| self.display_name(&view));

        ChainSnapshot {
            tank: Some(self.display_name(&view)),
            your_tank,
            interval_seconds: self.interval_for(&view),
            cast_time_seconds: self.cast_time_seconds,
            your_name: self.your_name.clone(),
            current_number,
            next_number,
            you_are_next_in,
            you_cast_in,
            you_last_offset,
            running: self.clock_running(&view),
            armed: self.clock_armed(&view),
            started_at_ms: self.clock_started(&view),
            beat_tick,
            warning,
            warning_urgent,
            warning_kind,
            warning_speech,
            tanks,
            slots,
            slot_format: self.slot_format,
            now_ms: now,
        }
    }

    pub(crate) fn next_after(&self, current: u32) -> Option<u32> {
        self.next_after_in(&self.tank_key_for(current), current)
    }

    fn you_cast_in(&self, now: u64, beat: &Option<Beat>) -> Option<f64> {
        let number = self.your_slot_number()?;
        if self.skipped.contains(&number) {
            return None;
        }
        let key = self.tank_key_for(number);
        if let Some(beat) = beat {
            if beat.rotation.len() <= 1 {
                let slot = self.slots.get(&number)?;
                return Some(remaining_cast(
                    Some(countdown_origin(
                        slot.last_shout_ms,
                        slot.last_actual_ms,
                        beat.start,
                    )),
                    self.cast_time_seconds,
                    now,
                ));
            }
            if !beat.rotation.contains(&number) {
                return None;
            }
            let remaining = remaining_until_slot(number, beat, now, 1.0).0;
            return Some(remaining);
        }
        let current = self.clock_current(&key)?;
        let next = self.next_after_in(&key, current)?;
        if next != number {
            return None;
        }
        let current_slot = self.slots.get(&current)?;
        let shouted = current_slot.last_shout_ms?;
        let elapsed = now.saturating_sub(shouted) as f64 / 1000.0;
        Some((self.interval_for(&key) - elapsed).max(0.0))
    }

    fn your_slot_number(&self) -> Option<u32> {
        self.slot_number_for_player("You")
    }

    fn slot_number_for_player(&self, speaker: &str) -> Option<u32> {
        self.slots
            .values()
            .find(|slot| {
                slot.player.eq_ignore_ascii_case(speaker)
                    || (self.is_you(&slot.player) && self.is_you(speaker))
            })
            .map(|slot| slot.number)
    }

    fn resolve_slot_number(
        &self,
        number: Option<u32>,
        speaker: &str,
        verb: &str,
    ) -> Result<u32, String> {
        match number {
            Some(number) => Ok(number),
            None => self.slot_number_for_player(speaker).ok_or_else(|| {
                format!("Cannot {verb}: {speaker} is not in the chain.")
            }),
        }
    }

    fn nearest_expected(&self, number: u32, actual: u64) -> Option<u64> {
        let beat = self.beat_for(&self.tank_key_for(number), actual)?;
        let idx = beat.rotation.iter().position(|&n| n == number)? as u64;
        let n = beat.rotation.len() as u64;
        let current_idx = beat.ticks % n;
        let steps_forward = (idx + n - current_idx) % n;
        let next = beat.start + (beat.ticks + steps_forward) * beat.interval_ms;
        let prev = next.saturating_sub(n * beat.interval_ms);
        if prev >= beat.start && actual.abs_diff(prev) < actual.abs_diff(next) {
            Some(prev)
        } else {
            Some(next)
        }
    }

    fn is_you(&self, player: &str) -> bool {
        player == "You"
            || self
                .your_name
                .as_ref()
                .is_some_and(|n| n.eq_ignore_ascii_case(player))
    }

    fn view_key(&self) -> TankKey {
        self.your_slot_number()
            .map(|n| self.tank_key_for(n))
            .unwrap_or(TankKey::Main)
    }

    fn tank_key_for(&self, number: u32) -> TankKey {
        self.tanks
            .iter()
            .find(|tank| number >= tank.from && number <= tank.to)
            .map(|tank| TankKey::Named(tank.name.clone()))
            .unwrap_or(TankKey::Main)
    }

    fn display_name(&self, key: &TankKey) -> String {
        match key {
            TankKey::Main => self.tank.clone().unwrap_or_else(|| "MT".into()),
            TankKey::Named(name) => name.clone(),
        }
    }

    fn interval_for(&self, key: &TankKey) -> f64 {
        match key {
            TankKey::Main => self.interval_seconds,
            TankKey::Named(name) => self
                .tanks
                .iter()
                .find(|tank| tank.name.eq_ignore_ascii_case(name))
                .and_then(|tank| tank.interval_seconds)
                .unwrap_or(self.interval_seconds),
        }
    }

    fn shout_sync(&self, key: &TankKey) -> bool {
        match key {
            TankKey::Main => self.shout_sync,
            TankKey::Named(name) => self
                .tanks
                .iter()
                .find(|tank| tank.name.eq_ignore_ascii_case(name))
                .map(|tank| tank.shout_sync)
                .unwrap_or(self.shout_sync),
        }
    }

    fn clock_armed(&self, key: &TankKey) -> bool {
        match key {
            TankKey::Main => self.armed,
            TankKey::Named(name) => self
                .tanks
                .iter()
                .find(|tank| tank.name.eq_ignore_ascii_case(name))
                .map(|tank| tank.armed)
                .unwrap_or(self.armed),
        }
    }

    fn set_armed(&mut self, key: &TankKey, value: bool) {
        match key {
            TankKey::Main => self.armed = value,
            TankKey::Named(name) => {
                if let Some(tank) = self
                    .tanks
                    .iter_mut()
                    .find(|tank| tank.name.eq_ignore_ascii_case(name))
                {
                    tank.armed = value;
                } else {
                    self.armed = value;
                }
            }
        }
    }

    fn set_shout_sync(&mut self, key: &TankKey, value: bool) {
        match key {
            TankKey::Main => self.shout_sync = value,
            TankKey::Named(name) => {
                if let Some(tank) = self
                    .tanks
                    .iter_mut()
                    .find(|tank| tank.name.eq_ignore_ascii_case(name))
                {
                    tank.shout_sync = value;
                } else {
                    self.shout_sync = value;
                }
            }
        }
    }

    fn running_current(&self, key: &TankKey, now: u64) -> Option<u32> {
        self.beat_for(key, now)
            .map(|beat| beat.current)
            .or_else(|| self.clock_current(key))
    }

    fn current_phase_ms(&self, key: &TankKey, now: u64) -> u64 {
        let interval = interval_ms(self.interval_for(key));
        match self.clock_started(key) {
            Some(start) if self.clock_running(key) => now.saturating_sub(start) % interval,
            _ => 0,
        }
    }

    fn infer_interval_for(&self, key: &TankKey) -> Option<f64> {
        let mut times: Vec<u64> = self
            .slots
            .values()
            .filter(|slot| self.tank_key_for(slot.number) == *key)
            .filter_map(|slot| slot.last_shout_ms)
            .collect();
        times.sort_unstable();
        times.dedup();
        if times.len() < 2 {
            return None;
        }
        let mut gaps: Vec<f64> = times
            .windows(2)
            .map(|pair| (pair[1] - pair[0]) as f64 / 1000.0)
            .filter(|gap| *gap >= 0.5 && *gap <= 12.0)
            .collect();
        if gaps.is_empty() {
            return None;
        }
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(round_interval(gaps[gaps.len() / 2]))
    }

    fn apply_inferred_interval(&mut self, key: &TankKey, seconds: f64) {
        match key {
            TankKey::Main => self.interval_seconds = seconds,
            TankKey::Named(name) => {
                if let Some(tank) = self
                    .tanks
                    .iter_mut()
                    .find(|tank| tank.name.eq_ignore_ascii_case(name))
                {
                    tank.interval_seconds = Some(seconds);
                } else {
                    self.interval_seconds = seconds;
                }
            }
        }
    }

    fn reanchor(&mut self, key: TankKey, current: u32, now: u64, phase_ms: u64) {
        let rotation = self.rotation_for(&key);
        if rotation.is_empty() {
            return;
        }
        let idx = rotation
            .iter()
            .position(|&number| number == current)
            .unwrap_or(0) as u64;
        let interval = interval_ms(self.interval_for(&key));
        let phase = phase_ms.min(interval.saturating_sub(1));
        let start = now.saturating_sub(phase + idx * interval);
        self.set_clock(key.clone(), true, Some(start));
        self.set_clock_current(key, Some(current));
    }

    fn preserve_beat(&mut self, key: TankKey, now: u64, current: Option<u32>) {
        if !self.clock_running(&key) {
            return;
        }
        let phase = self.current_phase_ms(&key, now);
        let keep = current
            .filter(|number| {
                !self.skipped.contains(number)
                    && self.slots.contains_key(number)
                    && self.tank_key_for(*number) == key
            })
            .or_else(|| current.and_then(|number| self.next_after_in(&key, number)))
            .or_else(|| self.rotation_for(&key).first().copied());
        let Some(keep) = keep else {
            return;
        };
        self.reanchor(key, keep, now, phase);
    }

    fn clock_running(&self, key: &TankKey) -> bool {
        match key {
            TankKey::Main => self.running,
            TankKey::Named(name) => self
                .tanks
                .iter()
                .find(|tank| tank.name.eq_ignore_ascii_case(name))
                .map(|tank| tank.running)
                .unwrap_or(false),
        }
    }

    fn clock_started(&self, key: &TankKey) -> Option<u64> {
        match key {
            TankKey::Main => self.started_at_ms,
            TankKey::Named(name) => self
                .tanks
                .iter()
                .find(|tank| tank.name.eq_ignore_ascii_case(name))
                .and_then(|tank| tank.started_at_ms),
        }
    }

    fn clock_current(&self, key: &TankKey) -> Option<u32> {
        match key {
            TankKey::Main => self.current_number,
            TankKey::Named(name) => self
                .tanks
                .iter()
                .find(|tank| tank.name.eq_ignore_ascii_case(name))
                .and_then(|tank| tank.current_number),
        }
    }

    fn set_clock_current(&mut self, key: TankKey, number: Option<u32>) {
        match key {
            TankKey::Main => self.current_number = number,
            TankKey::Named(name) => {
                if let Some(tank) = self
                    .tanks
                    .iter_mut()
                    .find(|tank| tank.name.eq_ignore_ascii_case(&name))
                {
                    tank.current_number = number;
                }
            }
        }
    }

    fn set_clock(&mut self, key: TankKey, running: bool, started_at_ms: Option<u64>) {
        match key {
            TankKey::Main => {
                self.running = running;
                self.started_at_ms = started_at_ms;
            }
            TankKey::Named(name) => {
                if let Some(tank) = self
                    .tanks
                    .iter_mut()
                    .find(|tank| tank.name.eq_ignore_ascii_case(&name))
                {
                    tank.running = running;
                    tank.started_at_ms = started_at_ms;
                }
            }
        }
    }

    fn rotation_for(&self, key: &TankKey) -> Vec<u32> {
        self.slots
            .keys()
            .copied()
            .filter(|n| !self.skipped.contains(n) && self.tank_key_for(*n) == *key)
            .collect()
    }

    fn next_after_in(&self, key: &TankKey, current: u32) -> Option<u32> {
        let nums = self.rotation_for(key);
        if nums.is_empty() {
            return None;
        }
        nums.iter()
            .copied()
            .find(|n| *n > current)
            .or_else(|| nums.first().copied())
    }

    fn beat_for(&self, key: &TankKey, now: u64) -> Option<Beat> {
        if !self.clock_running(key) {
            return None;
        }
        let start = self.clock_started(key)?;
        let rotation = self.rotation_for(key);
        let interval = self.interval_for(key);
        if rotation.is_empty() || interval <= 0.0 {
            return None;
        }
        let interval_ms = interval_ms(interval);
        let elapsed = now.saturating_sub(start);
        let ticks = elapsed / interval_ms;
        let idx = (ticks as usize) % rotation.len();
        let current = rotation[idx];
        let next = rotation[(idx + 1) % rotation.len()];
        Some(Beat {
            current,
            next,
            ticks,
            rotation,
            interval_ms,
            start,
        })
    }

    fn tank_snapshots(&self, now: u64) -> Vec<TankSnapshot> {
        let mut out = Vec::new();
        let you = self.your_slot_number().map(|n| self.tank_key_for(n));
        let mut keys = vec![TankKey::Main];
        keys.extend(
            self.tanks
                .iter()
                .map(|tank| TankKey::Named(tank.name.clone())),
        );
        for key in keys {
            let beat = self.beat_for(&key, now);
            let current = beat
                .as_ref()
                .map(|b| b.current)
                .or_else(|| self.clock_current(&key));
            let next = beat
                .as_ref()
                .map(|b| b.next)
                .or_else(|| current.and_then(|cur| self.next_after_in(&key, cur)));
            let (from, to) = match &key {
                TankKey::Main => (None, None),
                TankKey::Named(name) => self
                    .tanks
                    .iter()
                    .find(|tank| tank.name.eq_ignore_ascii_case(name))
                    .map(|tank| (Some(tank.from), Some(tank.to)))
                    .unwrap_or((None, None)),
            };
            out.push(TankSnapshot {
                name: self.display_name(&key),
                from,
                to,
                interval_seconds: self.interval_for(&key),
                running: self.clock_running(&key),
                armed: self.clock_armed(&key),
                started_at_ms: self.clock_started(&key),
                current_number: current,
                next_number: next,
                beat_tick: beat.as_ref().map(|b| b.ticks),
                is_you: you.as_ref() == Some(&key),
            });
        }
        out
    }

    fn parse_tank_key(&self, name: &str) -> Option<TankKey> {
        if name.eq_ignore_ascii_case("mt") {
            return Some(TankKey::Main);
        }
        if name.eq_ignore_ascii_case("ot") {
            let ot = self.off_tank.as_deref()?;
            return self.parse_tank_key(ot);
        }
        if self
            .tank
            .as_ref()
            .is_some_and(|tank| tank.eq_ignore_ascii_case(name))
        {
            return Some(TankKey::Main);
        }
        if self
            .tanks
            .iter()
            .any(|tank| tank.name.eq_ignore_ascii_case(name))
        {
            return Some(TankKey::Named(name.to_string()));
        }
        None
    }

    fn resolve_tank_keys(&self, name: Option<&str>) -> Result<Vec<TankKey>, String> {
        match name {
            None => {
                let mut keys = vec![TankKey::Main];
                keys.extend(
                    self.tanks
                        .iter()
                        .map(|tank| TankKey::Named(tank.name.clone())),
                );
                Ok(keys)
            }
            Some(name) => self
                .parse_tank_key(name)
                .map(|key| vec![key])
                .ok_or_else(|| format!("Unknown tank {name}.")),
        }
    }

    fn start_chains(&mut self, tank: Option<String>, _now: u64) -> Option<String> {
        let keys = match self.resolve_tank_keys(tank.as_deref()) {
            Ok(keys) => keys,
            Err(warning) => return Some(warning),
        };
        let mut started = 0;
        for key in keys {
            if self.rotation_for(&key).is_empty() {
                continue;
            }
            self.set_shout_sync(&key, false);
            self.set_armed(&key, true);
            self.set_clock(key.clone(), false, None);
            self.set_clock_current(key, None);
            started += 1;
        }
        if started == 0 {
            Some("Cannot start an empty chain. Take or shout numbers first.".into())
        } else {
            None
        }
    }

    fn stop_chains(&mut self, tank: Option<String>) -> Option<String> {
        let keys = match self.resolve_tank_keys(tank.as_deref()) {
            Ok(keys) => keys,
            Err(warning) => return Some(warning),
        };
        for key in keys {
            self.set_shout_sync(&key, false);
            self.set_armed(&key, false);
            self.set_clock(key.clone(), false, None);
            self.set_clock_current(key, None);
        }
        None
    }

    fn set_interval(&mut self, seconds: f64, tank: Option<String>) -> Option<String> {
        let Some(name) = tank else {
            self.interval_seconds = seconds;
            return None;
        };
        match self.parse_tank_key(&name) {
            Some(TankKey::Main) => {
                self.interval_seconds = seconds;
                None
            }
            Some(TankKey::Named(found)) => {
                if let Some(tank) = self
                    .tanks
                    .iter_mut()
                    .find(|tank| tank.name.eq_ignore_ascii_case(&found))
                {
                    tank.interval_seconds = Some(seconds);
                }
                None
            }
            None => Some(format!("Unknown tank {name}.")),
        }
    }

    fn set_off_tank(&mut self, tank: String) -> Option<String> {
        if self
            .tank
            .as_ref()
            .is_some_and(|mt| mt.eq_ignore_ascii_case(&tank))
        {
            return Some(format!("{tank} is already the main tank."));
        }
        if let Some(existing) = self.tanks.iter_mut().find(|t| t.is_off) {
            existing.name = tank.clone();
        } else if let Some(existing) = self
            .tanks
            .iter_mut()
            .find(|t| t.name.eq_ignore_ascii_case(&tank))
        {
            existing.is_off = true;
        }
        self.off_tank = Some(tank);
        None
    }

    fn split_at(&mut self, number: u32) -> Option<String> {
        let ot = match &self.off_tank {
            Some(name) => name.clone(),
            None => return Some("Set the off tank first with !ot <tank>.".into()),
        };
        if self
            .tanks
            .iter()
            .any(|tank| !tank.is_off && !tank.name.eq_ignore_ascii_case(&ot))
        {
            return Some("Split is for two tanks. Use !tank <name> <from> <to> for more.".into());
        }
        let warning = self.assign_range(ot.clone(), number, 999);
        if let Some(tank) = self
            .tanks
            .iter_mut()
            .find(|tank| tank.name.eq_ignore_ascii_case(&ot))
        {
            tank.is_off = true;
        }
        warning
    }

    fn assign_range(&mut self, name: String, from: u32, to: u32) -> Option<String> {
        if from > to {
            return Some("Tank range needs the lower number first.".into());
        }
        if self
            .tank
            .as_ref()
            .is_some_and(|mt| mt.eq_ignore_ascii_case(&name))
        {
            return Some(format!("{name} is the main tank. Use !ot or a different name."));
        }
        for tank in &self.tanks {
            if tank.name.eq_ignore_ascii_case(&name) {
                continue;
            }
            if from <= tank.to && to >= tank.from {
                return Some(format!(
                    "Range {}-{} overlaps {} ({}-{}).",
                    self.fmt_slot(from),
                    self.fmt_slot(to),
                    tank.name,
                    self.fmt_slot(tank.from),
                    self.fmt_slot(tank.to)
                ));
            }
        }
        let is_off = self
            .off_tank
            .as_ref()
            .is_some_and(|ot| ot.eq_ignore_ascii_case(&name));
        if let Some(tank) = self
            .tanks
            .iter_mut()
            .find(|tank| tank.name.eq_ignore_ascii_case(&name))
        {
            tank.name = name;
            tank.from = from;
            tank.to = to;
            tank.is_off = tank.is_off || is_off;
        } else {
            self.tanks.push(TankAssignment {
                name,
                from,
                to,
                interval_seconds: None,
                running: false,
                started_at_ms: None,
                current_number: None,
                is_off,
                shout_sync: false,
                armed: false,
            });
        }
        None
    }

    fn remove_tank(&mut self, name: &str) -> Option<String> {
        let before = self.tanks.len();
        self.tanks
            .retain(|tank| !tank.name.eq_ignore_ascii_case(name));
        if self.tanks.len() == before {
            return Some(format!("No tank named {name}."));
        }
        if self
            .off_tank
            .as_ref()
            .is_some_and(|ot| ot.eq_ignore_ascii_case(name))
        {
            self.off_tank = None;
        }
        None
    }

    fn remap_current(&mut self, from: u32, to: u32) {
        let swap = |cur: &mut Option<u32>| {
            if *cur == Some(from) {
                *cur = Some(to);
            } else if *cur == Some(to) {
                *cur = Some(from);
            }
        };
        swap(&mut self.current_number);
        for tank in &mut self.tanks {
            swap(&mut tank.current_number);
        }
    }

    fn clear_current_if(&mut self, number: u32) {
        if self.current_number == Some(number) {
            self.current_number = None;
        }
        for tank in &mut self.tanks {
            if tank.current_number == Some(number) {
                tank.current_number = None;
            }
        }
    }

    fn target_mismatch(&self, number: u32, target: &str, is_you: bool) -> Option<String> {
        if target.is_empty() {
            return None;
        }
        let slot_key = self.tank_key_for(number);
        let expected = match &slot_key {
            TankKey::Main => self.tank.as_deref()?,
            TankKey::Named(name) => name.as_str(),
        };
        if target.eq_ignore_ascii_case(expected) {
            return None;
        }
        if self.parse_tank_key(target).as_ref() == Some(&slot_key) {
            return None;
        }
        let tank = self.display_name(&slot_key);
        Some(if is_you {
            format!("You CHed {target} instead of {tank}.")
        } else {
            let player = self
                .slots
                .get(&number)
                .map(|slot| slot.player.as_str())
                .unwrap_or("They");
            format!("{player} CHed {target} instead of {tank}.")
        })
    }
}

fn remaining_until_slot(number: u32, beat: &Beat, now: u64, cycle_seconds: f64) -> (f64, f64) {
    let Some(idx) = beat.rotation.iter().position(|&n| n == number) else {
        return (0.0, 0.0);
    };
    let n = beat.rotation.len() as u64;
    if n == 0 {
        return (0.0, 0.0);
    }
    let current_idx = beat.ticks % n;
    let mut steps = (idx as u64 + n - current_idx) % n;
    if steps == 0 {
        steps = n;
    }
    let next_beat = beat.start + (beat.ticks + steps) * beat.interval_ms;
    let remaining = (next_beat as i128 - now as i128).max(0) as f64 / 1000.0;
    let progress = if cycle_seconds > 0.0 {
        (remaining / cycle_seconds).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (remaining, progress)
}

fn interval_ms(seconds: f64) -> u64 {
    (seconds * 1000.0).round().max(1.0) as u64
}

fn round_interval(seconds: f64) -> f64 {
    seconds.round().max(1.0)
}

fn offset_seconds(expected: u64, actual: u64) -> f64 {
    (actual as i64 - expected as i64) as f64 / 1000.0
}

fn remaining_cast(last_shout_ms: Option<u64>, cast_time: f64, now: u64) -> f64 {
    let Some(at) = last_shout_ms else {
        return 0.0;
    };
    let elapsed = now.saturating_sub(at) as f64 / 1000.0;
    (cast_time - elapsed).max(0.0)
}

fn countdown_origin(last_shout_ms: Option<u64>, last_actual_ms: Option<u64>, start: u64) -> u64 {
    [last_shout_ms, last_actual_ms, Some(start)]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(start)
}

fn cast_started_ms(slot: &ClericSlot) -> Option<u64> {
    match (slot.last_shout_ms, slot.last_actual_ms) {
        (Some(shout), Some(actual)) => Some(shout.max(actual)),
        (Some(shout), None) => Some(shout),
        (None, Some(actual)) => Some(actual),
        _ => None,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::CompleteHealCall;

    fn call(speaker: &str, number: u32, you: bool) -> CompleteHealCall {
        CompleteHealCall {
            speaker: speaker.into(),
            is_you: you,
            number,
            target: "Mluian".into(),
            tag: Some("GG".into()),
            raw: format!("GG {number:03} CH -- Mluian"),
        }
    }

    fn filled() -> ChainState {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 1 }, "Clericone".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 2 }, "Two".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 3 }, "Three".into());
        chain
    }

    #[test]
    fn take_skip_move_and_next() {
        let mut chain = filled();
        chain.apply_heal(call("Clericone", 1, true));
        assert_eq!(chain.next_after(1), Some(2));
        chain.apply_command(ChainCommand::Skip { number: Some(2) }, "Lead".into());
        assert_eq!(chain.next_after(1), Some(3));
        chain.apply_command(ChainCommand::Back { number: Some(2) }, "Lead".into());
        chain.apply_command(ChainCommand::Move { from: 1, to: 2 }, "Lead".into());
        assert_eq!(chain.slots.get(&2).unwrap().player, "Clericone");
        assert_eq!(chain.slots.get(&1).unwrap().player, "Two");
        chain.apply_command(ChainCommand::ResetChain, "Lead".into());
        assert!(chain.slots.is_empty());
        assert_eq!(chain.current_number, None);
        assert!(!chain.running);
    }

    #[test]
    fn take_moves_the_player_off_their_old_number() {
        let mut chain = filled();
        chain.apply_command(ChainCommand::Skip { number: Some(1) }, "Lead".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 8 }, "You".into());
        assert!(chain.slots.get(&1).is_none());
        assert!(!chain.skipped.contains(&1));
        assert_eq!(chain.slots.get(&8).unwrap().player, "You");
        assert!(chain.slots.get(&8).unwrap().last_shout_ms.is_none());
        assert!(!chain.running);
        assert_eq!(chain.current_number, None);
    }

    #[test]
    fn take_does_not_start_a_timer() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command_at(ChainCommand::Take { player: None, number: 1 }, "You".into(), 1_000);
        let snap = chain.snapshot_at(1_500);
        assert!(!snap.running);
        assert_eq!(snap.current_number, None);
        assert_eq!(snap.you_are_next_in, None);
        assert_eq!(snap.you_cast_in, None);
        let slot = snap.slots.iter().find(|s| s.number == 1).unwrap();
        assert_eq!(slot.last_shout_ms, None);
        assert_eq!(slot.remaining_seconds, 0.0);
        assert!(!slot.is_current);
    }

    #[test]
    fn retaking_the_same_number_does_not_clear_its_skip() {
        let mut chain = filled();
        chain.apply_command(ChainCommand::Skip { number: Some(1) }, "Lead".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 1 }, "You".into());
        assert!(chain.skipped.contains(&1));
        assert_eq!(chain.slots.get(&1).unwrap().player, "You");
    }

    #[test]
    fn take_can_assign_another_player_by_name() {
        let mut chain = filled();
        chain.apply_command(
            ChainCommand::Take {
                number: 8,
                player: Some("Portlia".into()),
            },
            "Clericone".into(),
        );
        assert_eq!(chain.slots.get(&8).unwrap().player, "Portlia");
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        chain.apply_command(
            ChainCommand::Take {
                number: 9,
                player: Some("Two".into()),
            },
            "Lead".into(),
        );
        assert!(chain.slots.get(&2).is_none());
        assert_eq!(chain.slots.get(&9).unwrap().player, "Two");
    }

    #[test]
    fn take_occupied_does_not_replace_the_occupant() {
        let mut chain = filled();
        chain.apply_command(ChainCommand::Take { player: None, number: 2 }, "Clericone".into());
        assert_eq!(chain.slots.get(&2).unwrap().player, "Two");
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.warning.as_deref(), Some("002 is already taken."));
        assert!(chain.warning_urgent);
        assert_eq!(chain.warning_kind, WarningKind::SlotTaken);
    }

    #[test]
    fn take_without_a_number_assigns_the_next_free_slot() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(ChainCommand::TakeNext { player: None }, "Clericone".into());
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.warning.as_deref(), Some("You got 001."));
        assert_eq!(chain.warning_speech.as_deref(), Some("You got 1"));
        assert_eq!(chain.warning_kind, WarningKind::AutoTake);
        assert!(chain.warning_urgent);

        chain.apply_command(ChainCommand::TakeNext { player: None }, "Two".into());
        assert_eq!(chain.slots.get(&2).unwrap().player, "Two");
        assert_eq!(chain.warning.as_deref(), Some("Two got 002."));
        assert_eq!(chain.warning_speech, None);
        assert!(!chain.warning_urgent);

        chain.apply_command(ChainCommand::Take { player: None, number: 4 }, "Three".into());
        chain.apply_command(ChainCommand::TakeNext { player: None }, "Four".into());
        assert_eq!(chain.slots.get(&3).unwrap().player, "Four");
        assert_eq!(chain.warning.as_deref(), Some("Four got 003."));

        chain.apply_command(ChainCommand::TakeNext { player: None }, "Clericone".into());
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.warning.as_deref(), Some("You already have 001."));
        assert_eq!(chain.warning_speech.as_deref(), Some("You already have 1"));
    }

    #[test]
    fn rampage_take_without_a_letter_assigns_the_next_free_slot() {
        let mut chain = ChainState::new_rampage(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(ChainCommand::TakeNext { player: None }, "Clericone".into());
        chain.apply_command(ChainCommand::TakeNext { player: None }, "Two".into());
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.slots.get(&2).unwrap().player, "Two");
        assert_eq!(chain.warning.as_deref(), Some("Two got BBB."));
        assert_eq!(chain.warning_speech, None);
    }

    #[test]
    fn claiming_an_occupied_number_warns_and_does_not_add_the_taker() {
        let mut yours = filled();
        yours.apply_heal(call("Clericone", 1, true));
        assert!(yours.warning.is_none());

        yours.apply_heal(call("Two", 1, false));
        assert_eq!(yours.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(yours.slots.get(&2).unwrap().player, "Two");
        assert_eq!(yours.warning.as_deref(), Some("Two: 001 is already taken."));
        assert!(!yours.warning_urgent);
        assert_eq!(yours.warning_kind, WarningKind::SlotTaken);

        let mut yours_take = filled();
        yours_take.apply_heal(call("Clericone", 2, true));
        assert_eq!(yours_take.slots.get(&2).unwrap().player, "Two");
        assert_eq!(yours_take.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(
            yours_take.warning.as_deref(),
            Some("002 is already taken.")
        );
        assert!(yours_take.warning_urgent);

        let mut others = filled();
        others.apply_heal(call("Three", 2, false));
        assert_eq!(others.slots.get(&2).unwrap().player, "Two");
        assert_eq!(others.slots.get(&3).unwrap().player, "Three");
        assert_eq!(
            others.warning.as_deref(),
            Some("Three: 002 is already taken.")
        );
        assert!(!others.warning_urgent);
    }

    #[test]
    fn skip_unset_number_warns_and_back_is_idempotent() {
        let mut chain = ChainState::new(2.0, 10.0);
        let warning = chain.apply_command(ChainCommand::Skip { number: Some(4) }, "Lead".into());
        assert!(warning.unwrap().contains("004"));
        assert!(chain
            .apply_command(ChainCommand::Back { number: Some(4) }, "Lead".into())
            .is_none());
    }

    #[test]
    fn skip_and_back_without_number_use_the_speaker_slot() {
        let mut chain = filled();
        assert!(chain
            .apply_command(ChainCommand::Skip { number: None }, "Two".into())
            .is_none());
        assert!(chain.skipped.contains(&2));
        assert!(!chain.skipped.contains(&1));

        assert!(chain
            .apply_command(ChainCommand::Back { number: None }, "two".into())
            .is_none());
        assert!(!chain.skipped.contains(&2));

        assert!(chain
            .apply_command(ChainCommand::Skip { number: None }, "You".into())
            .is_none());
        assert!(chain.skipped.contains(&1));

        let warning = chain
            .apply_command(ChainCommand::Skip { number: None }, "Lead".into())
            .unwrap();
        assert!(warning.contains("Lead"));
    }

    #[test]
    fn move_requires_two_different_set_numbers() {
        let mut chain = filled();
        assert_eq!(
            chain.apply_command(ChainCommand::Move { from: 1, to: 1 }, "Lead".into()),
            Some("Move needs two different numbers.".into())
        );
        assert!(chain
            .apply_command(ChainCommand::Move { from: 9, to: 1 }, "Lead".into())
            .unwrap()
            .contains("009"));
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert!(chain
            .apply_command(ChainCommand::Move { from: 1, to: 9 }, "Lead".into())
            .unwrap()
            .contains("009"));
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
    }

    #[test]
    fn move_swaps_skip_flags_and_current() {
        let mut chain = filled();
        chain.apply_heal(call("Clericone", 1, true));
        chain.apply_command(ChainCommand::Skip { number: Some(1) }, "Lead".into());
        chain.apply_command(ChainCommand::Move { from: 1, to: 2 }, "Lead".into());
        assert!(chain.skipped.contains(&2));
        assert!(!chain.skipped.contains(&1));
        assert_eq!(chain.current_number, Some(2));
        assert_eq!(chain.slots.get(&2).unwrap().player, "Clericone");
        assert_eq!(chain.slots.get(&1).unwrap().player, "Two");
    }

    #[test]
    fn next_wraps_around_and_skips_gaps() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.apply_command(ChainCommand::Take { player: None, number: 1 }, "A".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 5 }, "B".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 9 }, "C".into());
        assert_eq!(chain.next_after(5), Some(9));
        assert_eq!(chain.next_after(9), Some(1));
        chain.apply_command(ChainCommand::Skip { number: Some(1) }, "Lead".into());
        assert_eq!(chain.next_after(9), Some(5));
    }

    #[test]
    fn empty_or_all_skipped_has_no_next() {
        let mut chain = ChainState::new(2.0, 10.0);
        assert_eq!(chain.next_after(1), None);
        chain.apply_command(ChainCommand::Take { player: None, number: 1 }, "A".into());
        chain.apply_command(ChainCommand::Skip { number: Some(1) }, "Lead".into());
        assert_eq!(chain.next_after(1), None);
    }

    #[test]
    fn snapshot_marks_current_next_you_and_progress() {
        let mut chain = filled();
        chain.apply_heal_at(call("Two", 2, false), 1_000);
        let snap = chain.snapshot_at(3_500);
        assert_eq!(snap.current_number, Some(2));
        assert_eq!(snap.next_number, Some(3));
        assert!(snap.you_are_next_in.is_none());
        let current = snap.slots.iter().find(|s| s.number == 2).unwrap();
        assert!(current.is_current);
        assert!((current.remaining_seconds - 7.5).abs() < 0.05);
        assert!((current.progress - 0.75).abs() < 0.05);
        let you = snap.slots.iter().find(|s| s.number == 1).unwrap();
        assert!(you.is_you);
        assert!(!you.is_next);
        let next = snap.slots.iter().find(|s| s.number == 3).unwrap();
        assert!(next.is_next);
    }

    #[test]
    fn single_cleric_chain_counts_down_to_the_next_beat() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(
            ChainCommand::Take {
                player: None,
                number: 1,
            },
            "Clericone".into(),
        );
        chain.apply_command_at(
            ChainCommand::StartChain { tank: None },
            "Lead".into(),
            10_000,
        );
        let waiting = chain.snapshot_at(10_000);
        assert!(waiting.armed);
        assert!(!waiting.running);

        chain.apply_heal_at(call("Clericone", 1, true), 10_000);
        let start = chain.snapshot_at(10_000);
        assert!(start.running);
        assert!(!start.armed);
        assert_eq!(start.current_number, Some(1));
        let you = start.slots.iter().find(|s| s.number == 1).unwrap();
        assert!(you.is_current);
        assert!(!you.is_next);
        assert!((you.remaining_seconds - 10.0).abs() < 0.05);
        assert!((start.you_cast_in.unwrap() - 10.0).abs() < 0.05);

        let mid = chain.snapshot_at(11_000);
        let you = mid.slots.iter().find(|s| s.number == 1).unwrap();
        assert!(you.is_current);
        assert!((you.remaining_seconds - 9.0).abs() < 0.05);
        assert!((mid.you_cast_in.unwrap() - 9.0).abs() < 0.05);

        chain.apply_heal_at(call("Clericone", 1, true), 12_000);
        let after_shout = chain.snapshot_at(14_000);
        let you = after_shout.slots.iter().find(|s| s.number == 1).unwrap();
        assert!((you.remaining_seconds - 8.0).abs() < 0.05);
        assert!((after_shout.you_cast_in.unwrap() - 8.0).abs() < 0.05);
        assert_eq!(after_shout.started_at_ms, Some(12_000));
    }

    #[test]
    fn solo_chain_ignores_a_stale_shout_when_starting() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(
            ChainCommand::Take {
                player: None,
                number: 1,
            },
            "Clericone".into(),
        );
        chain.apply_heal_at(call("Clericone", 1, true), 1_000);
        chain.apply_command_at(
            ChainCommand::StartChain { tank: None },
            "Lead".into(),
            20_000,
        );
        let start = chain.snapshot_at(21_000);
        assert!(start.armed);
        assert!(!start.running);
        let you = start.slots.iter().find(|s| s.number == 1).unwrap();
        assert_eq!(you.remaining_seconds, 0.0);
        assert_eq!(start.you_cast_in, None);
    }

    #[test]
    fn solo_chain_refills_the_bar_after_a_late_ch() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(
            ChainCommand::Take {
                player: None,
                number: 1,
            },
            "Clericone".into(),
        );
        chain.apply_command_at(
            ChainCommand::StartChain { tank: None },
            "Lead".into(),
            10_000,
        );
        let empty = chain.snapshot_at(21_000);
        let you = empty.slots.iter().find(|s| s.number == 1).unwrap();
        assert_eq!(you.remaining_seconds, 0.0);

        chain.apply_heal_at(call("Clericone", 1, true), 21_000);
        let reset = chain.snapshot_at(21_000);
        let you = reset.slots.iter().find(|s| s.number == 1).unwrap();
        assert!((you.remaining_seconds - 10.0).abs() < 0.05);
        assert!((you.progress - 1.0).abs() < 0.05);
        assert!((reset.you_cast_in.unwrap() - 10.0).abs() < 0.05);
        assert_eq!(reset.started_at_ms, Some(21_000));

        let later = chain.snapshot_at(23_000);
        let you = later.slots.iter().find(|s| s.number == 1).unwrap();
        assert!((you.remaining_seconds - 8.0).abs() < 0.05);
    }

    #[test]
    fn running_chain_keeps_ch_cast_progress() {
        let mut chain = filled();
        chain.apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 10_000);
        chain.apply_heal_at(call("Two", 2, false), 12_000);
        let snap = chain.snapshot_at(14_000);
        let two = snap.slots.iter().find(|s| s.number == 2).unwrap();
        assert_eq!(two.last_cast_ms, Some(12_000));
        assert!((two.cast_remaining_seconds - 8.0).abs() < 0.05);
        assert!((two.cast_progress - 0.8).abs() < 0.05);
        assert!((two.remaining_seconds - two.cast_remaining_seconds).abs() > 0.5);
    }

    #[test]
    fn you_are_next_counts_down_from_the_last_shout() {
        let mut chain = filled();
        chain.apply_heal_at(call("Three", 3, false), 1_000);
        let snap = chain.snapshot_at(1_500);
        assert_eq!(snap.next_number, Some(1));
        let eta = snap.you_are_next_in.expect("you are next");
        assert!((eta - 1.5).abs() < 0.05);
        let later = chain.snapshot_at(6_000);
        assert_eq!(later.you_are_next_in, Some(0.0));
    }

    #[test]
    fn empty_heal_target_is_stored_as_none() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.apply_heal(CompleteHealCall {
            speaker: "Two".into(),
            is_you: false,
            number: 2,
            target: String::new(),
            tag: Some("GG".into()),
            raw: "GG 002 CH".into(),
        });
        assert_eq!(chain.slots.get(&2).unwrap().target, None);
    }

    #[test]
    fn set_your_name_rewrites_you_and_previous_name() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.apply_command(ChainCommand::Take { player: None, number: 1 }, "You".into());
        chain.set_your_name("Clericone".into());
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        chain.set_your_name("Clericone".into());
        chain.set_your_name("Newname".into());
        assert_eq!(chain.slots.get(&1).unwrap().player, "Newname");
    }

    #[test]
    fn remaining_cast_clamps_and_handles_missing_shout() {
        assert_eq!(remaining_cast(None, 10.0, 1000), 0.0);
        assert_eq!(remaining_cast(Some(1000), 10.0, 1000), 10.0);
        assert_eq!(remaining_cast(Some(1000), 10.0, 14_000), 0.0);
        assert!((remaining_cast(Some(1000), 10.0, 3500) - 7.5).abs() < f64::EPSILON);
    }

    #[test]
    fn stale_warning_is_hidden_in_snapshot() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.warning = Some("old".into());
        chain.warning_at_ms = Some(1_000);
        let snap = chain.snapshot_at(22_000);
        assert_eq!(snap.warning, None);
        let fresh = chain.snapshot_at(5_000);
        assert_eq!(fresh.warning.as_deref(), Some("old"));
    }

    #[test]
    fn interval_and_tank_commands() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.apply_command(
            ChainCommand::MainTank {
                tank: "Mluian".into(),
            },
            "Lead".into(),
        );
        chain.apply_command(
            ChainCommand::ChainInterval {
                seconds: 3.0,
                tank: None,
            },
            "Lead".into(),
        );
        assert_eq!(chain.tank.as_deref(), Some("Mluian"));
        assert_eq!(chain.interval_seconds, 3.0);
    }

    #[test]
    fn start_and_end_iterate_on_interval_and_update_on_skip() {
        let mut chain = filled();
        assert!(chain
            .apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 10_000)
            .is_none());
        let waiting = chain.snapshot_at(10_000);
        assert!(waiting.armed);
        assert!(!waiting.running);
        chain.apply_heal_at(call("Clericone", 1, true), 10_000);
        let start = chain.snapshot_at(10_100);
        assert!(start.running);
        assert_eq!(start.current_number, Some(1));
        assert_eq!(start.next_number, Some(2));
        let you_now = start.you_cast_in.expect("you next cycle");
        assert!((you_now - 5.9).abs() < 0.15);
        let current = start.slots.iter().find(|s| s.number == 1).unwrap();
        assert!((current.remaining_seconds - 5.9).abs() < 0.15);
        assert!((current.progress - (5.9 / 6.0)).abs() < 0.05);

        let second = chain.snapshot_at(12_100);
        assert_eq!(second.current_number, Some(2));
        assert_eq!(second.next_number, Some(3));
        let you_eta = second.you_cast_in.expect("you next cycle");
        assert!((you_eta - 3.9).abs() < 0.15);

        chain.apply_command_at(ChainCommand::Skip { number: Some(2) }, "Lead".into(), 12_200);
        let skipped = chain.snapshot_at(12_200);
        assert_eq!(skipped.current_number, Some(3));
        assert_eq!(skipped.next_number, Some(1));

        chain.apply_command_at(ChainCommand::Back { number: Some(2) }, "Lead".into(), 12_300);
        chain.apply_command_at(ChainCommand::EndChain { tank: None }, "Lead".into(), 12_400);
        assert!(!chain.running);
        assert_eq!(chain.slots.len(), 3);
    }

    #[test]
    fn cannot_start_empty_chain() {
        let mut chain = ChainState::new(2.0, 10.0);
        assert!(chain
            .apply_command(ChainCommand::StartChain { tank: None }, "Lead".into())
            .unwrap()
            .contains("empty"));
    }

    #[test]
    fn timing_offset_from_expected_beat() {
        let mut chain = filled();
        chain.apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 10_000);
        chain.apply_heal_at(call("Clericone", 1, true), 10_000);
        chain.apply_heal_at(call("Two", 2, false), 12_250);
        let snap = chain.snapshot_at(12_250);
        let two = snap.slots.iter().find(|s| s.number == 2).unwrap();
        assert!((two.offset_seconds.unwrap() - 0.25).abs() < 0.05);
        let offset = chain.slots.get(&2).unwrap().last_offset_seconds.unwrap();
        assert!((offset - 0.25).abs() < 0.05);

        chain.apply_heal_at(call("Clericone", 1, true), 10_400);
        let you = chain.slots.get(&1).unwrap().last_offset_seconds.unwrap();
        assert!((you - 0.4).abs() < 0.05);
        let you_snap = chain.snapshot_at(10_400);
        assert!((you_snap.you_last_offset.unwrap() - 0.4).abs() < 0.05);
    }

    #[test]
    fn startchain_waits_for_the_first_cleric_to_go() {
        let mut chain = filled();
        chain.apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 10_000);
        let waiting = chain.snapshot_at(10_100);
        assert!(waiting.armed);
        assert!(!waiting.running);
        assert_eq!(waiting.current_number, None);
        assert_eq!(waiting.started_at_ms, None);

        chain.apply_heal_at(call("Two", 2, false), 12_000);
        let started = chain.snapshot_at(12_000);
        assert!(started.running);
        assert!(!started.armed);
        assert_eq!(started.current_number, Some(2));
        assert_eq!(started.next_number, Some(3));
        assert_eq!(started.slots.iter().map(|s| s.number).collect::<Vec<_>>(), vec![1, 2, 3]);
        let two = started.slots.iter().find(|s| s.number == 2).unwrap();
        assert!(two.is_current);
        let three = started.slots.iter().find(|s| s.number == 3).unwrap();
        assert!(three.is_next);

        let later = chain.snapshot_at(14_100);
        assert_eq!(later.current_number, Some(3));
        assert_eq!(later.next_number, Some(1));
    }

    #[test]
    fn rampage_startchain_starts_from_the_first_letter_to_go() {
        let mut chain = ChainState::new_rampage(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 1 }, "Clericone".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 2 }, "Two".into());
        chain.apply_command(ChainCommand::Take { player: None, number: 3 }, "Three".into());
        chain.apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 10_000);
        let waiting = chain.snapshot_at(10_000);
        assert!(waiting.armed);
        assert!(!waiting.running);

        chain.apply_heal_at(call("Two", 2, false), 12_000);
        let started = chain.snapshot_at(12_000);
        assert!(started.running);
        assert_eq!(started.current_number, Some(2));
        assert_eq!(started.next_number, Some(3));
        assert_eq!(started.slot_format, SlotFormat::Letter);
    }

    #[test]
    fn startchain_mid_fight_waits_for_the_next_ch() {
        let mut chain = filled();
        chain.apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 10_000);
        chain.apply_heal_at(call("Clericone", 1, true), 10_000);
        assert!(chain.snapshot_at(10_100).running);
        chain.apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 15_000);
        let waiting = chain.snapshot_at(15_100);
        assert!(waiting.armed);
        assert!(!waiting.running);
        assert_eq!(waiting.started_at_ms, None);
        chain.apply_heal_at(call("Three", 3, false), 16_000);
        let snap = chain.snapshot_at(16_000);
        assert!(snap.running);
        assert_eq!(snap.current_number, Some(3));
        assert_eq!(snap.next_number, Some(1));
    }

    #[test]
    fn split_and_tank_ranges_are_independent_rotations() {
        let mut chain = filled();
        chain.apply_command(ChainCommand::MainTank { tank: "Mluian".into() }, "Lead".into());
        chain.apply_command(ChainCommand::OffTank { tank: "Beefwich".into() }, "Lead".into());
        assert!(chain
            .apply_command(ChainCommand::Split { number: 3 }, "Lead".into())
            .is_none());
        assert_eq!(chain.next_after(1), Some(2));
        assert_eq!(chain.next_after(2), Some(1));
        assert_eq!(chain.next_after(3), Some(3));

        chain.apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 10_000);
        chain.apply_heal_at(call("Clericone", 1, true), 10_000);
        chain.apply_heal_at(call("Three", 3, false), 10_000);
        let you = chain.snapshot_at(10_100);
        assert_eq!(you.your_tank.as_deref(), Some("Mluian"));
        assert!(you.running);
        assert_eq!(you.current_number, Some(1));
        assert!(you.slots.iter().all(|slot| slot.number < 3));
        assert!(you.slots.iter().all(|slot| slot.tank.as_deref() == Some("Mluian")));

        let ot_running = chain
            .tanks
            .iter()
            .find(|tank| tank.name == "Beefwich")
            .unwrap()
            .running;
        assert!(ot_running);

        chain.apply_command_at(
            ChainCommand::EndChain {
                tank: Some("Beefwich".into()),
            },
            "Lead".into(),
            10_200,
        );
        assert!(!chain.tanks.iter().find(|t| t.name == "Beefwich").unwrap().running);
        assert!(chain.running);

        chain.apply_command(
            ChainCommand::ChainInterval {
                seconds: 3.0,
                tank: Some("ot".into()),
            },
            "Lead".into(),
        );
        assert_eq!(chain.tanks[0].interval_seconds, Some(3.0));
        assert_eq!(chain.interval_seconds, 2.0);
    }

    #[test]
    fn tank_range_command_hides_the_other_chain_from_you() {
        let mut chain = filled();
        chain.apply_command(
            ChainCommand::TankRange {
                tank: "Beefwich".into(),
                from: 3,
                to: 8,
            },
            "Lead".into(),
        );
        chain.apply_command(ChainCommand::Take { player: None, number: 4 }, "You".into());
        let snap = chain.snapshot_at(1_000);
        assert_eq!(snap.your_tank.as_deref(), Some("Beefwich"));
        assert!(snap.slots.iter().all(|slot| slot.number >= 3));
        assert!(snap.slots.iter().all(|slot| slot.tank.as_deref() == Some("Beefwich")));
    }

    #[test]
    fn shouting_the_wrong_tank_warns() {
        let mut chain = filled();
        chain.apply_command(ChainCommand::MainTank { tank: "Mluian".into() }, "Lead".into());
        chain.apply_command(ChainCommand::OffTank { tank: "Beefwich".into() }, "Lead".into());
        chain.apply_command(ChainCommand::Split { number: 3 }, "Lead".into());
        chain.apply_heal(call("Three", 3, false));
        chain.apply_command(ChainCommand::StartChain { tank: None }, "Lead".into());
        chain.apply_heal(call("Two", 2, false));
        chain.apply_heal(CompleteHealCall {
            speaker: "Clericone".into(),
            is_you: true,
            number: 1,
            target: "Beefwich".into(),
            tag: Some("GG".into()),
            raw: "GG 001 CH -- Beefwich".into(),
        });
        assert_eq!(
            chain.warning.as_deref(),
            Some("You CHed Beefwich instead of Mluian.")
        );
        assert!(chain.warning_urgent);
        assert_eq!(chain.warning_kind, WarningKind::WrongTarget);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
    }

    #[test]
    fn shouting_a_target_that_is_not_the_set_tank_warns() {
        let mut chain = filled();
        chain.apply_command(ChainCommand::MainTank { tank: "Mluian".into() }, "Lead".into());
        chain.apply_command(ChainCommand::StartChain { tank: None }, "Lead".into());
        chain.apply_heal(call("Clericone", 1, true));
        chain.apply_heal(CompleteHealCall {
            speaker: "Clericone".into(),
            is_you: true,
            number: 1,
            target: "a goblin".into(),
            tag: Some("GG".into()),
            raw: "GG 001 CH -- a goblin".into(),
        });
        assert_eq!(
            chain.warning.as_deref(),
            Some("You CHed a goblin instead of Mluian.")
        );
        assert!(chain.warning_urgent);
        assert_eq!(chain.warning_kind, WarningKind::WrongTarget);
        chain.apply_heal(call("Clericone", 1, true));
        assert_eq!(chain.warning, None);
    }

    #[test]
    fn other_clerics_wrong_target_warns_without_urgency() {
        let mut chain = filled();
        chain.apply_command(ChainCommand::MainTank { tank: "Mluian".into() }, "Lead".into());
        chain.apply_command(ChainCommand::StartChain { tank: None }, "Lead".into());
        chain.apply_heal(call("Clericone", 1, true));
        chain.apply_heal(CompleteHealCall {
            speaker: "Two".into(),
            is_you: false,
            number: 2,
            target: "an orc".into(),
            tag: Some("GG".into()),
            raw: "GG 002 CH -- an orc".into(),
        });
        assert_eq!(
            chain.warning.as_deref(),
            Some("Two CHed an orc instead of Mluian.")
        );
        assert!(!chain.warning_urgent);
        assert_eq!(chain.warning_kind, WarningKind::WrongTarget);
        assert_eq!(chain.slots.get(&2).unwrap().player, "Two");
    }

    #[test]
    fn wrong_target_does_not_warn_before_the_chain_starts() {
        let mut chain = filled();
        chain.apply_command(ChainCommand::MainTank { tank: "Mluian".into() }, "Lead".into());
        chain.apply_heal(CompleteHealCall {
            speaker: "Clericone".into(),
            is_you: true,
            number: 1,
            target: "a goblin".into(),
            tag: Some("GG".into()),
            raw: "GG 001 CH -- a goblin".into(),
        });
        assert_eq!(chain.warning, None);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
    }

    #[test]
    fn first_join_wrong_target_does_not_warn() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(ChainCommand::MainTank { tank: "Mluian".into() }, "Lead".into());
        chain.apply_heal(CompleteHealCall {
            speaker: "Clericone".into(),
            is_you: true,
            number: 1,
            target: "a goblin".into(),
            tag: Some("GG".into()),
            raw: "GG 001 CH -- a goblin".into(),
        });
        assert_eq!(chain.warning, None);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
    }

    #[test]
    fn no_warning_when_tank_is_not_set() {
        let mut chain = filled();
        chain.apply_heal(CompleteHealCall {
            speaker: "Clericone".into(),
            is_you: true,
            number: 1,
            target: "a goblin".into(),
            tag: Some("GG".into()),
            raw: "GG 001 CH -- a goblin".into(),
        });
        assert_eq!(chain.warning, None);
    }

    #[test]
    fn rampage_wrong_target_warns() {
        let mut chain = ChainState::new_rampage(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(ChainCommand::MainTank { tank: "Mluian".into() }, "Lead".into());
        chain.apply_heal(call("Clericone", 1, true));
        chain.apply_command(ChainCommand::StartChain { tank: None }, "Lead".into());
        chain.apply_heal(call("Clericone", 1, true));
        chain.apply_heal(CompleteHealCall {
            speaker: "Clericone".into(),
            is_you: true,
            number: 1,
            target: "Beefwich".into(),
            tag: Some("GG".into()),
            raw: "GG AAA RCH -- Beefwich".into(),
        });
        assert_eq!(
            chain.warning.as_deref(),
            Some("You CHed Beefwich instead of Mluian.")
        );
        assert!(chain.warning_urgent);
        assert_eq!(chain.warning_kind, WarningKind::WrongTarget);
    }

    #[test]
    fn rampage_chain_formats_slots_as_letters() {
        let mut chain = ChainState::new_rampage(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_heal(call("Clericone", 1, true));
        chain.apply_heal(call("Two", 1, false));
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert!(chain
            .warning
            .as_deref()
            .unwrap()
            .contains("AAA is already taken"));
        let snap = chain.snapshot_at(1_000);
        assert_eq!(snap.slot_format, SlotFormat::Letter);
        assert_eq!(snap.slots[0].number, 1);
        assert_eq!(snap.warning_kind, WarningKind::SlotTaken);
    }

    #[test]
    fn late_join_infers_interval_and_syncs_you_after_the_live_shout() {
        let mut chain = ChainState::new(3.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_heal_at(call("Two", 2, false), 10_000);
        chain.apply_heal_at(call("Three", 3, false), 12_100);
        assert!(chain.running);
        assert!((chain.interval_seconds - 2.0).abs() < 0.01);
        chain.apply_command_at(ChainCommand::Take { player: None, number: 4 }, "You".into(), 12_400);
        let snap = chain.snapshot_at(12_400);
        assert_eq!(snap.current_number, Some(3));
        assert_eq!(snap.next_number, Some(4));
        let eta = snap.you_cast_in.expect("you are next");
        assert!((eta - 1.7).abs() < 0.2);
    }

    #[test]
    fn late_join_rounds_shout_gaps_to_the_nearest_second() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.apply_heal_at(call("Two", 2, false), 10_000);
        chain.apply_heal_at(call("Three", 3, false), 12_600);
        assert!((chain.interval_seconds - 3.0).abs() < 0.01);
    }

    #[test]
    fn adding_a_late_take_does_not_jump_a_started_chain() {
        let mut chain = filled();
        chain.apply_command_at(ChainCommand::StartChain { tank: None }, "Lead".into(), 10_000);
        chain.apply_heal_at(call("Clericone", 1, true), 10_000);
        assert_eq!(chain.snapshot_at(12_100).current_number, Some(2));
        chain.apply_command_at(ChainCommand::Take { player: None, number: 9 }, "Four".into(), 12_100);
        let snap = chain.snapshot_at(12_100);
        assert_eq!(snap.current_number, Some(2));
        assert_eq!(snap.next_number, Some(3));
        assert!(snap.slots.iter().any(|slot| slot.number == 9));
    }

    #[test]
    fn late_join_syncs_only_the_tank_you_took() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        chain.apply_command(
            ChainCommand::MainTank {
                tank: "Mluian".into(),
            },
            "Lead".into(),
        );
        chain.apply_command(
            ChainCommand::OffTank {
                tank: "Beefwich".into(),
            },
            "Lead".into(),
        );
        chain.apply_command(ChainCommand::Split { number: 9 }, "Lead".into());
        chain.apply_heal_at(call("A", 1, false), 10_000);
        chain.apply_heal_at(call("B", 2, false), 12_000);
        chain.apply_command_at(ChainCommand::Take { player: None, number: 9 }, "You".into(), 12_500);
        assert!(chain.running);
        assert!(!chain
            .tanks
            .iter()
            .find(|tank| tank.name == "Beefwich")
            .unwrap()
            .running);
        chain.apply_heal_at(call("C", 10, false), 13_000);
        chain.apply_heal_at(call("D", 11, false), 15_000);
        assert!(chain
            .tanks
            .iter()
            .find(|tank| tank.name == "Beefwich")
            .unwrap()
            .running);
        let snap = chain.snapshot_at(15_000);
        assert_eq!(snap.your_tank.as_deref(), Some("Beefwich"));
        assert_eq!(snap.current_number, Some(11));
        assert_eq!(snap.next_number, Some(9));
    }
}
