use crate::chain::{ChainEvent, ChainEventKind};
use crate::config;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

/// A chain that goes quiet for this long is treated as finished.
pub const IDLE_CLOSE_MS: u64 = 180_000;
/// A heal this far from its beat still counts as on time.
pub const ON_TIME_SECONDS: f64 = 0.25;

const MAX_SESSIONS: usize = 25;
const MAX_EVENTS: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionKind {
    Ch,
    Rampage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: u64,
    pub kind: SessionKind,
    pub started_at_ms: u64,
    pub ended_at_ms: Option<u64>,
    pub last_event_ms: u64,
    pub tank: Option<String>,
    pub events: Vec<ChainEvent>,
}

impl Session {
    fn new(id: u64, kind: SessionKind, at_ms: u64) -> Self {
        Self {
            id,
            kind,
            started_at_ms: at_ms,
            ended_at_ms: None,
            last_event_ms: at_ms,
            tank: None,
            events: Vec::new(),
        }
    }

    fn heal_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.kind == ChainEventKind::Heal)
            .count()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClericReport {
    pub rank: u32,
    pub player: String,
    pub slots: Vec<u32>,
    pub heals: u32,
    pub on_time: u32,
    pub early: u32,
    pub late: u32,
    pub missed_turns: u32,
    pub wrong_target: u32,
    pub skips: u32,
    pub is_you: bool,
    pub avg_offset_seconds: Option<f64>,
    pub worst_late_seconds: Option<f64>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReport {
    pub id: u64,
    pub kind: SessionKind,
    pub live: bool,
    pub started_at_ms: u64,
    pub ended_at_ms: Option<u64>,
    pub duration_seconds: f64,
    pub tank: Option<String>,
    pub total_heals: u32,
    pub missed_turns: u32,
    pub wrong_target: u32,
    pub warnings: u32,
    pub avg_offset_seconds: Option<f64>,
    pub score: f64,
    pub clerics: Vec<ClericReport>,
    pub events: Vec<ChainEvent>,
}

#[derive(Debug, Default)]
pub struct Recorder {
    sessions: Vec<Session>,
    next_id: u64,
    needs_save: bool,
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Self {
        let Ok(text) = fs::read_to_string(config::sessions_file_path()) else {
            return Self::new();
        };
        let sessions: Vec<Session> = serde_json::from_str(&text).unwrap_or_default();
        let next_id = sessions.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        Self {
            sessions,
            next_id,
            needs_save: false,
        }
    }

    /// Only finished sessions go to disk, and only once they are finished, so a
    /// running chain does not rewrite the file on every cast.
    pub fn save_if_needed(&mut self) {
        if !self.needs_save {
            return;
        }
        self.needs_save = false;
        let path = config::sessions_file_path();
        let finished: Vec<&Session> = self
            .sessions
            .iter()
            .filter(|session| session.ended_at_ms.is_some())
            .collect();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string(&finished) {
            let _ = fs::write(path, text);
        }
    }

    /// Files a batch of chain events into the open session for that chain,
    /// opening and closing sessions as the chain starts and stops.
    pub fn record(&mut self, kind: SessionKind, events: Vec<ChainEvent>) -> bool {
        let mut changed = false;
        for event in events {
            changed |= self.close_idle_at(event.at_ms);
            if self.open_index(kind).is_none() {
                if !starts_a_session(event.kind) {
                    continue;
                }
                let id = self.next_id;
                self.next_id += 1;
                self.sessions.push(Session::new(id, kind, event.at_ms));
                while self.sessions.len() > MAX_SESSIONS {
                    self.sessions.remove(0);
                }
            }
            let Some(index) = self.open_index(kind) else {
                continue;
            };
            let ends = matches!(event.kind, ChainEventKind::Stop | ChainEventKind::Reset);
            let session = &mut self.sessions[index];
            session.last_event_ms = event.at_ms.max(session.last_event_ms);
            if session.tank.is_none() {
                session.tank = event.tank.clone();
            }
            if session.events.len() >= MAX_EVENTS {
                session.events.remove(0);
            }
            session.events.push(event);
            if ends {
                self.close(index, session_end_ms(&self.sessions[index]));
            }
            changed = true;
        }
        changed
    }

    /// Closes chains that stopped sending events without a !stopchain.
    pub fn close_idle_at(&mut self, now: u64) -> bool {
        let stale: Vec<usize> = self
            .sessions
            .iter()
            .enumerate()
            .filter(|(_, session)| {
                session.ended_at_ms.is_none()
                    && now.saturating_sub(session.last_event_ms) >= IDLE_CLOSE_MS
            })
            .map(|(index, _)| index)
            .collect();
        if stale.is_empty() {
            return false;
        }
        for index in stale.into_iter().rev() {
            let end = session_end_ms(&self.sessions[index]);
            self.close(index, end);
        }
        true
    }

    pub fn reports(&self) -> Vec<SessionReport> {
        let mut reports: Vec<SessionReport> = self.sessions.iter().map(build_report).collect();
        reports.sort_by(|a, b| b.started_at_ms.cmp(&a.started_at_ms));
        reports
    }

    pub fn clear(&mut self) {
        self.sessions.clear();
        self.needs_save = true;
    }

    fn open_index(&self, kind: SessionKind) -> Option<usize> {
        self.sessions
            .iter()
            .position(|session| session.kind == kind && session.ended_at_ms.is_none())
    }

    /// Sessions where nobody ever cast are dropped instead of kept as clutter.
    fn close(&mut self, index: usize, at_ms: u64) {
        self.needs_save = true;
        if self.sessions[index].heal_count() == 0 {
            self.sessions.remove(index);
            return;
        }
        self.sessions[index].ended_at_ms = Some(at_ms);
    }
}

fn session_end_ms(session: &Session) -> u64 {
    session.last_event_ms.max(session.started_at_ms)
}

fn starts_a_session(kind: ChainEventKind) -> bool {
    matches!(
        kind,
        ChainEventKind::Heal | ChainEventKind::Start | ChainEventKind::Claim
    )
}

#[derive(Default)]
struct ClericTally {
    slots: Vec<u32>,
    heal_times: Vec<u64>,
    offsets: Vec<f64>,
    on_time: u32,
    early: u32,
    late: u32,
    wrong_target: u32,
    skips: u32,
    is_you: bool,
}

pub fn build_report(session: &Session) -> SessionReport {
    let mut tallies: BTreeMap<String, ClericTally> = BTreeMap::new();
    let mut warnings = 0;
    let mut offsets: Vec<f64> = Vec::new();

    for event in &session.events {
        let player = event.player.clone().unwrap_or_default();
        if event.is_you && !player.is_empty() {
            tallies.entry(player.clone()).or_default().is_you = true;
        }
        match event.kind {
            ChainEventKind::Heal => {
                let tally = tallies.entry(player.clone()).or_default();
                if let Some(number) = event.number {
                    if !tally.slots.contains(&number) {
                        tally.slots.push(number);
                    }
                }
                tally.heal_times.push(event.at_ms);
                if let Some(offset) = event.offset_seconds {
                    tally.offsets.push(offset);
                    offsets.push(offset);
                    if offset > ON_TIME_SECONDS {
                        tally.late += 1;
                    } else if offset < -ON_TIME_SECONDS {
                        tally.early += 1;
                    } else {
                        tally.on_time += 1;
                    }
                }
            }
            ChainEventKind::WrongTarget => {
                tallies.entry(player).or_default().wrong_target += 1;
                warnings += 1;
            }
            ChainEventKind::Skip => {
                tallies.entry(player).or_default().skips += 1;
            }
            ChainEventKind::Claim => {
                let tally = tallies.entry(player).or_default();
                if let Some(number) = event.number {
                    if !tally.slots.contains(&number) {
                        tally.slots.push(number);
                    }
                }
            }
            ChainEventKind::Warning => warnings += 1,
            _ => {}
        }
    }
    tallies.remove("");

    let mut clerics: Vec<ClericReport> = tallies
        .into_iter()
        .map(|(player, tally)| {
            let heals = tally.heal_times.len() as u32;
            let missed_turns = missed_turns(&tally.heal_times);
            let avg_offset_seconds = mean(&tally.offsets);
            let worst_late_seconds = tally
                .offsets
                .iter()
                .copied()
                .fold(None::<f64>, |worst, offset| match worst {
                    Some(worst) if worst >= offset => Some(worst),
                    _ => Some(offset),
                })
                .filter(|worst| *worst > ON_TIME_SECONDS);
            let score = score(&ScoreInput {
                heals,
                late: tally.late,
                missed_turns,
                wrong_target: tally.wrong_target,
                avg_abs_offset: mean(&tally.offsets.iter().map(|o| o.abs()).collect::<Vec<f64>>()),
            });
            ClericReport {
                rank: 0,
                slots: tally.slots,
                heals,
                on_time: tally.on_time,
                early: tally.early,
                late: tally.late,
                missed_turns,
                wrong_target: tally.wrong_target,
                skips: tally.skips,
                is_you: tally.is_you,
                avg_offset_seconds,
                worst_late_seconds,
                score,
                player,
            }
        })
        .collect();

    clerics.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.heals.cmp(&a.heals))
            .then(a.player.cmp(&b.player))
    });
    for (index, cleric) in clerics.iter_mut().enumerate() {
        cleric.rank = index as u32 + 1;
    }

    let end = session.ended_at_ms.unwrap_or(session.last_event_ms);
    SessionReport {
        id: session.id,
        kind: session.kind,
        live: session.ended_at_ms.is_none(),
        started_at_ms: session.started_at_ms,
        ended_at_ms: session.ended_at_ms,
        duration_seconds: end.saturating_sub(session.started_at_ms) as f64 / 1000.0,
        tank: session.tank.clone(),
        total_heals: clerics.iter().map(|c| c.heals).sum(),
        missed_turns: clerics.iter().map(|c| c.missed_turns).sum(),
        wrong_target: clerics.iter().map(|c| c.wrong_target).sum(),
        warnings,
        avg_offset_seconds: mean(&offsets),
        score: session_score(&clerics),
        clerics,
        events: session.events.clone(),
    }
}

/// How the chain did as a whole: each cleric's score weighted by how much of
/// the chain they carried, so one cleric who cast twice cannot sink a good pull.
pub fn session_score(clerics: &[ClericReport]) -> f64 {
    let casts: u32 = clerics.iter().map(|cleric| cleric.heals).sum();
    if casts == 0 {
        return 0.0;
    }
    clerics
        .iter()
        .map(|cleric| cleric.score * cleric.heals as f64)
        .sum::<f64>()
        / casts as f64
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

/// Turns a cleric skipped are inferred from gaps in their own cast times: a
/// gap near twice their usual spacing means one turn came and went unhealed.
pub fn missed_turns(times: &[u64]) -> u32 {
    if times.len() < 3 {
        return 0;
    }
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    let gaps: Vec<u64> = sorted.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let mut ordered = gaps.clone();
    ordered.sort_unstable();
    let cycle = ordered[ordered.len() / 2];
    if cycle == 0 {
        return 0;
    }
    gaps.iter()
        .map(|gap| {
            let ratio = *gap as f64 / cycle as f64;
            if ratio < 1.75 {
                0
            } else {
                ratio.round() as u32 - 1
            }
        })
        .sum()
}

pub struct ScoreInput {
    pub heals: u32,
    pub late: u32,
    pub missed_turns: u32,
    pub wrong_target: u32,
    pub avg_abs_offset: Option<f64>,
}

/// 100 is a cleric who never missed a turn and hit every beat on the tick.
pub fn score(input: &ScoreInput) -> f64 {
    if input.heals == 0 {
        return 0.0;
    }
    let mut score = 100.0;
    if let Some(offset) = input.avg_abs_offset {
        score -= (offset * 40.0).min(40.0);
    }
    score -= input.missed_turns as f64 * 8.0;
    score -= input.wrong_target as f64 * 5.0;
    score -= (input.late as f64 / input.heals as f64) * 10.0;
    score.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainState;
    use crate::parser::{ChainCommand, CompleteHealCall};

    fn heal(player: &str, number: u32, at_ms: u64, offset: Option<f64>) -> ChainEvent {
        ChainEvent {
            at_ms,
            kind: ChainEventKind::Heal,
            player: Some(player.into()),
            is_you: player == "You",
            number: Some(number),
            target: Some("Mluian".into()),
            tank: Some("Mluian".into()),
            offset_seconds: offset,
            text: format!("{player} cast {number:03}"),
        }
    }

    fn plain(kind: ChainEventKind, at_ms: u64) -> ChainEvent {
        ChainEvent {
            at_ms,
            kind,
            player: Some("Lead".into()),
            is_you: false,
            number: None,
            target: None,
            tank: Some("Mluian".into()),
            offset_seconds: None,
            text: "command".into(),
        }
    }

    #[test]
    fn a_session_opens_on_start_and_closes_on_stop() {
        let mut recorder = Recorder::new();
        assert!(!recorder.record(SessionKind::Ch, vec![plain(ChainEventKind::Tank, 1_000)]));
        assert!(recorder.reports().is_empty());

        recorder.record(
            SessionKind::Ch,
            vec![
                plain(ChainEventKind::Start, 2_000),
                heal("One", 1, 2_000, Some(0.0)),
                heal("Two", 2, 4_000, Some(0.1)),
                plain(ChainEventKind::Stop, 6_000),
            ],
        );
        let reports = recorder.reports();
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].live);
        assert_eq!(reports[0].total_heals, 2);
        assert_eq!(reports[0].duration_seconds, 4.0);
        assert_eq!(reports[0].tank.as_deref(), Some("Mluian"));
    }

    #[test]
    fn a_session_without_a_heal_is_dropped() {
        let mut recorder = Recorder::new();
        recorder.record(
            SessionKind::Ch,
            vec![
                plain(ChainEventKind::Start, 1_000),
                plain(ChainEventKind::Stop, 2_000),
            ],
        );
        assert!(recorder.reports().is_empty());
    }

    #[test]
    fn a_quiet_chain_closes_itself_before_the_next_one_opens() {
        let mut recorder = Recorder::new();
        recorder.record(
            SessionKind::Ch,
            vec![
                plain(ChainEventKind::Start, 1_000),
                heal("One", 1, 2_000, Some(0.0)),
            ],
        );
        assert!(recorder.reports()[0].live);
        recorder.record(
            SessionKind::Ch,
            vec![heal("One", 1, 2_000 + IDLE_CLOSE_MS, Some(0.0))],
        );
        let reports = recorder.reports();
        assert_eq!(reports.len(), 2);
        assert!(reports[0].live);
        assert_eq!(reports[1].ended_at_ms, Some(2_000));
    }

    #[test]
    fn ch_and_rampage_sessions_run_side_by_side() {
        let mut recorder = Recorder::new();
        recorder.record(SessionKind::Ch, vec![heal("One", 1, 1_000, Some(0.0))]);
        recorder.record(SessionKind::Rampage, vec![heal("Two", 1, 1_100, Some(0.0))]);
        let reports = recorder.reports();
        assert_eq!(reports.len(), 2);
        assert!(reports.iter().any(|r| r.kind == SessionKind::Rampage));
        assert!(reports.iter().any(|r| r.kind == SessionKind::Ch));
    }

    #[test]
    fn clerics_are_ranked_by_accuracy_and_misses() {
        let mut recorder = Recorder::new();
        let mut events = vec![plain(ChainEventKind::Start, 0)];
        for round in 0..6u64 {
            let at = round * 6_000;
            events.push(heal("Steady", 1, at, Some(0.02)));
            events.push(heal("Sloppy", 2, at + 2_000, Some(0.9)));
            // Laggy sits out one turn in the middle of the fight.
            if round != 2 {
                events.push(heal("Laggy", 3, at + 4_000, Some(0.3)));
            }
        }
        events.push(plain(ChainEventKind::Stop, 40_000));
        recorder.record(SessionKind::Ch, events);

        let report = recorder.reports().remove(0);
        assert_eq!(report.clerics[0].player, "Steady");
        assert_eq!(report.clerics[0].rank, 1);
        assert_eq!(report.clerics[0].on_time, 6);
        assert!(report.clerics[0].score > 95.0);
        let laggy = report
            .clerics
            .iter()
            .find(|c| c.player == "Laggy")
            .expect("laggy");
        assert_eq!(laggy.missed_turns, 1);
        assert!(laggy.score < report.clerics[0].score);
        let sloppy = report
            .clerics
            .iter()
            .find(|c| c.player == "Sloppy")
            .expect("sloppy");
        assert_eq!(sloppy.late, 6);
        assert!((sloppy.worst_late_seconds.unwrap() - 0.9).abs() < 0.001);
        assert_eq!(report.missed_turns, 1);
    }

    #[test]
    fn the_session_score_leans_on_whoever_cast_the_most() {
        let mut recorder = Recorder::new();
        let mut events = vec![plain(ChainEventKind::Start, 0)];
        for round in 0..8u64 {
            events.push(heal("Steady", 1, round * 6_000, Some(0.0)));
        }
        // One cleric who took two turns badly should barely move the chain score.
        events.push(heal("Cameo", 2, 2_000, Some(2.0)));
        events.push(heal("Cameo", 2, 14_000, Some(2.0)));
        events.push(plain(ChainEventKind::Stop, 50_000));
        recorder.record(SessionKind::Ch, events);

        let report = recorder.reports().remove(0);
        let cameo = report
            .clerics
            .iter()
            .find(|c| c.player == "Cameo")
            .expect("cameo");
        assert!(cameo.score < 60.0, "{}", cameo.score);
        assert!(report.score > 85.0, "{}", report.score);
        assert!(report.score < 100.0);
        assert_eq!(session_score(&[]), 0.0);
    }

    #[test]
    fn wrong_targets_and_warnings_are_counted() {
        let mut recorder = Recorder::new();
        let mut wrong = heal("Two", 2, 3_000, None);
        wrong.kind = ChainEventKind::WrongTarget;
        recorder.record(
            SessionKind::Ch,
            vec![heal("Two", 2, 1_000, Some(0.0)), wrong],
        );
        let report = recorder.reports().remove(0);
        assert_eq!(report.wrong_target, 1);
        assert_eq!(report.warnings, 1);
        assert_eq!(report.clerics[0].wrong_target, 1);
    }

    #[test]
    fn missed_turns_needs_a_rhythm_to_compare_against() {
        assert_eq!(missed_turns(&[]), 0);
        assert_eq!(missed_turns(&[1_000, 7_000]), 0);
        assert_eq!(missed_turns(&[0, 6_000, 12_000, 18_000]), 0);
        assert_eq!(missed_turns(&[0, 6_000, 18_000, 24_000]), 1);
        assert_eq!(missed_turns(&[0, 6_000, 24_000, 30_000]), 2);
        assert_eq!(missed_turns(&[5, 5, 5, 5]), 0);
    }

    #[test]
    fn score_punishes_misses_more_than_a_little_drift() {
        let perfect = score(&ScoreInput {
            heals: 10,
            late: 0,
            missed_turns: 0,
            wrong_target: 0,
            avg_abs_offset: Some(0.0),
        });
        assert_eq!(perfect, 100.0);
        assert_eq!(
            score(&ScoreInput {
                heals: 0,
                late: 0,
                missed_turns: 0,
                wrong_target: 0,
                avg_abs_offset: None,
            }),
            0.0
        );
        let drifty = score(&ScoreInput {
            heals: 10,
            late: 2,
            missed_turns: 0,
            wrong_target: 0,
            avg_abs_offset: Some(0.2),
        });
        let missing = score(&ScoreInput {
            heals: 10,
            late: 0,
            missed_turns: 3,
            wrong_target: 0,
            avg_abs_offset: Some(0.0),
        });
        assert!(missing < drifty);
        assert!(drifty < perfect);
    }

    #[test]
    fn only_the_newest_sessions_are_kept() {
        let mut recorder = Recorder::new();
        for round in 0..(MAX_SESSIONS as u64 + 5) {
            let at = round * 10_000;
            recorder.record(
                SessionKind::Ch,
                vec![
                    plain(ChainEventKind::Start, at),
                    heal("One", 1, at, Some(0.0)),
                    plain(ChainEventKind::Stop, at + 1_000),
                ],
            );
        }
        assert_eq!(recorder.reports().len(), MAX_SESSIONS);
    }

    #[test]
    fn clearing_drops_every_session() {
        let mut recorder = Recorder::new();
        recorder.record(SessionKind::Ch, vec![heal("One", 1, 1_000, Some(0.0))]);
        recorder.clear();
        assert!(recorder.reports().is_empty());
    }

    /// The log calls you "You" on your commands and your character name on the
    /// lines Alfred reads back, so both have to land on the same row.
    #[test]
    fn your_own_lines_are_one_cleric_however_the_log_named_you() {
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Alfredicus".into());
        chain.apply_command_at(
            ChainCommand::Take {
                number: 1,
                player: None,
            },
            "You".into(),
            1_000,
        );
        chain.apply_command_at(
            ChainCommand::StartChain { tank: None },
            "Raidlead".into(),
            1_500,
        );
        for (round, at) in [2_000u64, 12_000, 22_000].into_iter().enumerate() {
            chain.apply_heal_at(
                CompleteHealCall {
                    speaker: "Alfredicus".into(),
                    is_you: false,
                    number: 1,
                    target: "Mluian".into(),
                    tag: Some("GG".into()),
                    raw: format!("GG 001 CH -- Mluian round {round}"),
                },
                at,
            );
        }

        let mut recorder = Recorder::new();
        recorder.record(SessionKind::Ch, chain.take_events());
        let reports = recorder.reports();
        let report = reports.first().expect("a session");
        assert_eq!(report.clerics.len(), 1, "{:?}", report.clerics);
        let you = &report.clerics[0];
        assert_eq!(you.player, "You");
        assert!(you.is_you);
        assert_eq!(you.heals, 3);
        assert_eq!(you.slots, vec![1]);
    }
}
