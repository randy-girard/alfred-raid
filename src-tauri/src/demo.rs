use crate::parser::TestChannel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A complete heal takes this long to cast, so a cleric cannot come back around
/// any sooner than that no matter how tight the chain is.
pub const CAST_SECONDS: f64 = 10.0;

/// However many clerics pile onto a demo chain, the beat never goes under a
/// second: a raid cannot call numbers faster than that and read them.
pub const MIN_INTERVAL_SECONDS: f64 = 1.0;

pub const MAX_CLERICS: usize = 24;
pub const MAX_MINUTES: u32 = 30;

/// How the raid scenario paces the things that happen mid-chain.
const JOIN_LEAD_MS: u64 = 8_000;
const SKIP_FIRST_MS: u64 = 25_000;
const SKIP_EVERY_MS: u64 = 35_000;
const BACK_AFTER_MS: u64 = 18_000;

/// A scripted line that Alfred feeds to itself as if it came from the game log.
#[derive(Debug, Clone)]
pub struct DemoStep {
    pub delay_ms: u64,
    pub note: String,
    pub speaker: String,
    pub channel: TestChannel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoScenario {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: usize,
    pub seconds: f64,
    /// Whether the roster, length, and interval on the demo page apply.
    pub configurable: bool,
    pub clerics: usize,
    pub interval_seconds: f64,
}

/// Roster and length for the scenarios that take them. The interval is not a
/// choice: it falls out of the cast time and however many clerics are on the
/// chain at the time.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoOptions {
    /// On the chain when the pull starts.
    pub clerics: usize,
    /// Where the roster ends up once the latecomers have taken numbers.
    pub max_clerics: usize,
    /// How long the CH chain itself runs, not counting taking numbers.
    pub minutes: u32,
}

impl Default for DemoOptions {
    fn default() -> Self {
        Self {
            clerics: 8,
            max_clerics: 20,
            minutes: 4,
        }
    }
}

impl DemoOptions {
    pub fn normalized(self) -> Self {
        let clerics = self.clerics.clamp(1, MAX_CLERICS);
        Self {
            clerics,
            max_clerics: self.max_clerics.clamp(clerics, MAX_CLERICS),
            minutes: self.minutes.clamp(1, MAX_MINUTES),
        }
    }
}

pub const SCENARIO_IDS: &[&str] = &[
    "chain",
    "mistakes",
    "laggy",
    "wrongtank",
    "tankswap",
    "offtank",
    "split",
    "rampage",
    "rampagetank",
    "collision",
    "latejoin",
    "thin",
    "collapse",
    "macros",
    "shuffle",
    "raid",
    "full",
];

/// The scripted pulls, in the order "Everything" replays them. The raid
/// scenario is left out: it is configurable and runs as long as you ask.
const SHORT_IDS: &[&str] = &[
    "chain",
    "mistakes",
    "laggy",
    "wrongtank",
    "tankswap",
    "offtank",
    "split",
    "rampage",
    "rampagetank",
    "collision",
    "latejoin",
    "thin",
    "collapse",
    "macros",
    "shuffle",
];

/// The tightest interval a chain of this many clerics can hold: the cast time
/// split between them, rounded up to a tenth so nobody is asked to cast before
/// their last complete heal has landed.
pub fn interval_seconds(clerics: usize) -> f64 {
    if clerics == 0 {
        return CAST_SECONDS;
    }
    let split = (CAST_SECONDS / clerics as f64 * 10.0).ceil() / 10.0;
    split.max(MIN_INTERVAL_SECONDS)
}

fn interval_ms(clerics: usize) -> u64 {
    (interval_seconds(clerics) * 1000.0).round() as u64
}

fn cycle_ms(clerics: usize) -> u64 {
    interval_ms(clerics) * clerics as u64
}

fn pace_note(clerics: usize) -> String {
    format!(
        "{clerics} clerics on a {CAST_SECONDS:.0}s CH, so {}s apart",
        interval_seconds(clerics)
    )
}

pub fn is_configurable(id: &str) -> bool {
    id == "raid"
}

pub fn describe(id: &str, options: DemoOptions) -> DemoScenario {
    let options = options.normalized();
    let steps = scenario_steps_with(id, options);
    let configurable = is_configurable(id);
    DemoScenario {
        id: id.to_string(),
        name: scenario_name(id).to_string(),
        description: scenario_description(id).to_string(),
        seconds: steps.iter().map(|step| step.delay_ms).sum::<u64>() as f64 / 1000.0,
        steps: steps.len(),
        clerics: if configurable { options.max_clerics } else { 0 },
        interval_seconds: if configurable {
            interval_seconds(options.max_clerics)
        } else {
            0.0
        },
        configurable,
    }
}

pub fn scenarios() -> Vec<DemoScenario> {
    SCENARIO_IDS
        .iter()
        .map(|id| describe(id, DemoOptions::default()))
        .collect()
}

fn scenario_name(id: &str) -> &'static str {
    match id {
        "chain" => "Cleric chain",
        "mistakes" => "Mistakes and alerts",
        "laggy" => "Laggy cleric",
        "wrongtank" => "Wrong tank",
        "tankswap" => "Main tank goes down",
        "offtank" => "Off tank joins mid-pull",
        "split" => "Two-tank split",
        "rampage" => "Rampage chain",
        "rampagetank" => "CH and rampage together",
        "collision" => "Number collisions",
        "latejoin" => "Joining a chain in progress",
        "thin" => "Short-handed chain",
        "collapse" => "Chain falls apart",
        "macros" => "Broken macros",
        "shuffle" => "Slot shuffle",
        "raid" => "Full raid chain",
        _ => "Everything",
    }
}

fn scenario_description(id: &str) -> &'static str {
    match id {
        "chain" => "Four clerics 2.5s apart, the chain starts, and the rotation runs clean.",
        "mistakes" => {
            "A taken number, a wrong target, a macro Alfred cannot read, and a skip that \
             re-paces the chain."
        }
        "laggy" => {
            "One cleric drifts later every round until he misses his turn, gets skipped, and \
             comes back on the beat."
        }
        "wrongtank" => {
            "A split chain where clerics keep CHing the tank they are not assigned to, on both \
             sides of the split."
        }
        "tankswap" => {
            "The main tank dies mid-pull and the lead names a new one. One cleric keeps healing \
             the old tank."
        }
        "offtank" => {
            "A single rotation splits in two mid-pull when the off tank picks up a add, then \
             folds back together."
        }
        "split" => "Main tank and off tank on separate rotations, two clerics on each.",
        "rampage" => "Three clerics on rampage letters, 3.4s apart.",
        "rampagetank" => {
            "A CH chain on the main tank and a rampage chain on another mob at the same time, \
             with one rampage heal on the wrong target."
        }
        "collision" => {
            "Four clerics fight over the same numbers: taken slots, auto-takes, a number taken \
             for someone else, and a CH on a slot that is not yours."
        }
        "latejoin" => {
            "You arrive after the pull started. Alfred infers the interval from the shouts it \
             can hear and syncs you behind the last cleric."
        }
        "thin" => {
            "Two clerics 5s apart, one drops to a solo chain on the cast time, then a third \
             arrives and the lead tightens the pace."
        }
        "collapse" => {
            "Five clerics thin out one at a time, with the lead re-pacing after every loss \
             until the chain cannot hold."
        }
        "macros" => {
            "Macros Alfred cannot read: no number, the number in the wrong place, and a rampage \
             macro with no letter."
        }
        "shuffle" => {
            "The lead rearranges a running chain with moves, skips, and backs, then resets it \
             and starts over."
        }
        "raid" => {
            "A long pull that starts short-handed and fills up. Set the starting roster, the \
             roster to build to, and how long the chain runs. Latecomers take numbers mid-pull \
             and the lead re-paces, clerics skip out and come back, one drifts late, one misses \
             turns, and one CHes the wrong tank."
        }
        _ => "Every scenario back to back, from an empty chain to a full raid.",
    }
}

/// Builds a scenario on an absolute clock and turns it into per-step delays.
/// Casts are held back until the caster's last complete heal has finished, so a
/// script can never ask a cleric to chain faster than the spell allows.
struct Script {
    now_ms: u64,
    last_cast: BTreeMap<String, u64>,
    steps: Vec<DemoStep>,
}

impl Script {
    /// Every scenario starts from a clean chain so it can be replayed any time.
    fn new() -> Self {
        Self {
            now_ms: 400,
            last_cast: BTreeMap::new(),
            steps: vec![DemoStep {
                delay_ms: 400,
                note: "Clearing whatever was on the panels".into(),
                speaker: "Alfredbot".into(),
                channel: TestChannel::Guild,
                message: "!reset-chain".into(),
            }],
        }
    }

    /// Returns when the line actually goes out, which is never before the line
    /// ahead of it.
    fn say(
        &mut self,
        at_ms: u64,
        note: &str,
        speaker: &str,
        channel: TestChannel,
        message: String,
    ) -> u64 {
        let at_ms = at_ms.max(self.now_ms);
        self.steps.push(DemoStep {
            delay_ms: at_ms - self.now_ms,
            note: note.to_string(),
            speaker: speaker.to_string(),
            channel,
            message,
        });
        self.now_ms = at_ms;
        at_ms
    }

    fn command(&mut self, at_ms: u64, note: &str, speaker: &str, message: &str) {
        self.say(
            at_ms,
            note,
            speaker,
            TestChannel::Guild,
            message.to_string(),
        );
    }

    fn pace(&mut self, at_ms: u64, clerics: usize, tank: Option<&str>) {
        let seconds = interval_seconds(clerics);
        let message = match tank {
            Some(tank) => format!("!chain {seconds} {tank}"),
            None => format!("!chain {seconds}"),
        };
        let note = match tank {
            Some(tank) => format!("{tank}: {}", pace_note(clerics)),
            None => pace_note(clerics),
        };
        self.say(at_ms, &note, "Raidlead", TestChannel::Guild, message);
    }

    fn cast(
        &mut self,
        at_ms: u64,
        note: &str,
        speaker: &str,
        slot: &str,
        macro_name: &str,
        target: &str,
    ) -> u64 {
        let ready = self
            .last_cast
            .get(speaker)
            .map(|last| last + (CAST_SECONDS * 1000.0) as u64)
            .unwrap_or(0);
        // A line can also be held back by whatever is ahead of it in the
        // script, so the cooldown is tracked from when the cast really lands.
        let at_ms = self.say(
            at_ms.max(ready),
            note,
            speaker,
            TestChannel::Shout,
            format!("GG {slot} {macro_name} -- {target}"),
        );
        self.last_cast.insert(speaker.to_string(), at_ms);
        at_ms
    }

    fn finish(self) -> Vec<DemoStep> {
        self.steps
    }
}

fn ch_slot(number: u32) -> String {
    format!("{number:03}")
}

fn rch_slot(number: u32) -> String {
    let letter = char::from_u32(b'A' as u32 + number.saturating_sub(1).min(25)).unwrap_or('A');
    letter.to_string().repeat(3)
}

#[cfg(test)]
pub fn scenario_steps(id: &str) -> Vec<DemoStep> {
    scenario_steps_with(id, DemoOptions::default())
}

pub fn scenario_steps_with(id: &str, options: DemoOptions) -> Vec<DemoStep> {
    let options = options.normalized();
    match id {
        "chain" => chain_steps(),
        "mistakes" => mistakes_steps(),
        "laggy" => laggy_steps(),
        "wrongtank" => wrongtank_steps(),
        "tankswap" => tankswap_steps(),
        "offtank" => offtank_steps(),
        "split" => split_steps(),
        "rampage" => rampage_steps(),
        "rampagetank" => rampagetank_steps(),
        "collision" => collision_steps(),
        "latejoin" => latejoin_steps(),
        "thin" => thin_steps(),
        "collapse" => collapse_steps(),
        "macros" => macros_steps(),
        "shuffle" => shuffle_steps(),
        "raid" => raid_steps(options),
        "full" => {
            let mut steps = Vec::new();
            for id in SHORT_IDS {
                let next = scenario_steps_with(id, options);
                if steps.is_empty() {
                    steps = next;
                } else {
                    steps.extend(breather(next));
                }
            }
            steps
        }
        _ => Vec::new(),
    }
}

/// Enough names for a full raid roster. You are always on the chain too.
const CLERIC_NAMES: &[&str] = &[
    "Portlia",
    "Bellamere",
    "Curaja",
    "Dunkin",
    "Hanbox",
    "Mirelda",
    "Oswynne",
    "Faldric",
    "Genevive",
    "Tobrin",
    "Marisol",
    "Ellowyn",
    "Brannoc",
    "Sisterwin",
    "Alderic",
    "Quillon",
    "Verath",
    "Lumenna",
    "Corwyn",
    "Perrin",
    "Sableen",
    "Rhoswen",
    "Wendric",
];

/// You take a number in the middle of the pack so the "Cast in" countdown has
/// somewhere to run. `you_at` is a seat in the starting group, so you are never
/// one of the clerics who wanders in halfway through the pull.
fn raid_roster(clerics: usize, you_at: usize) -> Vec<String> {
    let mut names = Vec::with_capacity(clerics);
    let mut pool = CLERIC_NAMES.iter().cycle();
    for index in 0..clerics {
        if index == you_at {
            names.push("YOU".to_string());
        } else {
            names.push((*pool.next().expect("cleric names cycle")).to_string());
        }
    }
    names
}

fn you_seat(clerics: usize) -> usize {
    (clerics / 3).min(clerics.saturating_sub(1))
}

/// The next cleric to send off to med: anyone on the chain except you, so the
/// panels keep counting down to your own turn.
fn next_skip_seat(roster: &[String], active: &[bool], cursor: &mut usize) -> Option<usize> {
    for step in 0..roster.len() {
        let seat = (*cursor + step) % roster.len();
        if active[seat] && roster[seat] != "YOU" {
            *cursor = seat + 1;
            return Some(seat);
        }
    }
    None
}

/// Small deterministic wobble so the offsets on the report are not all zero.
fn jitter_ms(round: u64, index: usize) -> i64 {
    ((round * 31 + index as u64 * 17) % 7) as i64 * 45 - 135
}

fn raid_steps(options: DemoOptions) -> Vec<DemoStep> {
    let start = options.clerics;
    let max = options.max_clerics;
    let roster = raid_roster(max, you_seat(start));
    let budget = options.minutes as u64 * 60_000;
    let mut script = Script::new();

    // Roles only make sense once the roster is big enough to spare them.
    let quirks = max >= 6;
    let slow = 4 % max;
    let flaky = 7 % max;
    let sloppy = 11 % max;

    script.command(
        600,
        "The raid lead names the main tank",
        "Raidlead",
        "!mt Mluian",
    );
    script.pace(1_200, start, None);

    let mut at = 2_000;
    for (seat, player) in roster.iter().enumerate().take(start) {
        let number = seat as u32 + 1;
        let note = if player == "YOU" {
            format!("You take {}", ch_slot(number))
        } else {
            format!("{player} takes {}", ch_slot(number))
        };
        script.command(at, &note, player, &format!("!take {}", ch_slot(number)));
        at += 350;
    }

    script.command(at + 600, "The chain arms", "Raidlead", "!startchain");

    let first = at + 2_000;
    let mut active: Vec<bool> = (0..max).map(|seat| seat < start).collect();
    let mut live = start;
    let joiners = max - start;
    // Spread the latecomers over the first part of the pull so the last one in
    // still gets turns on a full chain.
    let join_every = if joiners == 0 {
        u64::MAX
    } else {
        ((budget * 7 / 10).saturating_sub(JOIN_LEAD_MS) / joiners as u64).max(4_000)
    };
    let mut next_join = start;
    let mut join_at = first + JOIN_LEAD_MS;
    let mut skip_at = first + SKIP_FIRST_MS;
    let mut back_at = u64::MAX;
    let mut resting: Option<usize> = None;
    let mut skip_cursor = 0usize;

    let mut round = 0u64;
    let mut clock = first;
    let mut last = first;
    while clock - first < budget {
        if next_join < max && clock >= join_at {
            let number = next_join as u32 + 1;
            let player = roster[next_join].clone();
            script.command(
                clock,
                &format!("{player} shows up late and takes {}", ch_slot(number)),
                &player,
                &format!("!take {}", ch_slot(number)),
            );
            active[next_join] = true;
            live += 1;
            next_join += 1;
            // Absolute schedule, so a slow round does not push the rest back.
            join_at += join_every;
            script.pace(clock + 500, live, None);
            clock += 1_000;
        }

        match resting {
            Some(seat) if clock >= back_at => {
                let player = roster[seat].clone();
                script.command(
                    clock,
                    &format!("{player} is back on the chain"),
                    &player,
                    &format!("!back {}", ch_slot(seat as u32 + 1)),
                );
                active[seat] = true;
                live += 1;
                resting = None;
                script.pace(clock + 500, live, None);
                clock += 1_000;
                skip_at = clock + SKIP_EVERY_MS;
            }
            // Nobody sits out unless the pull lasts long enough for them to come back.
            None if clock >= skip_at && live > 2 && clock + BACK_AFTER_MS < first + budget => {
                if let Some(seat) = next_skip_seat(&roster, &active, &mut skip_cursor) {
                    let player = roster[seat].clone();
                    script.command(
                        clock,
                        &format!("{player} drops out to med"),
                        &player,
                        &format!("!skip {}", ch_slot(seat as u32 + 1)),
                    );
                    active[seat] = false;
                    live -= 1;
                    resting = Some(seat);
                    back_at = clock + BACK_AFTER_MS;
                    script.pace(clock + 500, live, None);
                    clock += 1_000;
                }
            }
            _ => {}
        }

        let seats: Vec<usize> = (0..max).filter(|seat| active[*seat]).collect();
        if seats.is_empty() {
            break;
        }
        let interval_ms = interval_ms(seats.len());
        for (position, &seat) in seats.iter().enumerate() {
            let slot = ch_slot(seat as u32 + 1);
            let player = &roster[seat];
            if quirks && seat == flaky && round % 5 == 4 {
                continue;
            }
            let late = if quirks && seat == slow { 600 } else { 0 };
            let wobble = jitter_ms(round, seat) + late;
            let beat = clock + position as u64 * interval_ms;
            let at = (beat as i64 + wobble).max(0) as u64;
            let wrong = quirks && seat == sloppy && round % 7 == 3;
            let target = if wrong { "Beefwich" } else { "Mluian" };
            let note = if wrong {
                format!("{player} CHes the wrong tank on {slot}")
            } else if late > 0 {
                format!("{player} is late again on {slot}")
            } else {
                format!("{player} casts {slot} on round {}", round + 1)
            };
            last = script.cast(at, &note, player, &slot, "CH", target);
        }
        clock += interval_ms * seats.len() as u64;
        round += 1;
    }
    script.command(last + 2_000, "The mob is down", "Raidlead", "!stopchain");
    script.finish()
}

/// Back to back scenarios need a pause between them, or the clerics from the
/// last pull would be asked to cast again before their heal landed.
fn breather(mut steps: Vec<DemoStep>) -> Vec<DemoStep> {
    if let Some(first) = steps.first_mut() {
        first.delay_ms += 4_000;
        first.note = format!("Next pull. {}", first.note);
    }
    steps
}

fn chain_steps() -> Vec<DemoStep> {
    let roster = [
        ("YOU", 1u32),
        ("Portlia", 2),
        ("Bellamere", 3),
        ("Curaja", 4),
    ];
    let interval = interval_ms(roster.len());
    let cycle = cycle_ms(roster.len());
    let mut script = Script::new();

    script.command(
        600,
        "The raid lead names the main tank",
        "Raidlead",
        "!mt Mluian",
    );
    script.pace(1_200, roster.len(), None);
    script.command(2_000, "You take the first number", "YOU", "!take 001");
    script.command(
        2_600,
        "Portlia takes the next free number",
        "Portlia",
        "!take",
    );
    script.command(
        3_200,
        "Bellamere takes the next free number",
        "Bellamere",
        "!take",
    );
    script.command(3_800, "Curaja takes 004 by hand", "Curaja", "!take 004");
    script.command(
        4_600,
        "The chain arms and waits for the first CH",
        "Raidlead",
        "!startchain",
    );

    let first = 6_000;
    let mut last = first;
    for round in 0..3u64 {
        for (index, (player, number)) in roster.iter().enumerate() {
            let slack = if round == 1 && *player == "Bellamere" {
                900
            } else {
                0
            };
            let note = if slack > 0 {
                format!("{player} is 0.9s late on {}", ch_slot(*number))
            } else {
                format!("{player} casts {} on round {}", ch_slot(*number), round + 1)
            };
            last = script.cast(
                first + round * cycle + index as u64 * interval + slack,
                &note,
                player,
                &ch_slot(*number),
                "CH",
                "Mluian",
            );
        }
    }
    script.command(last + 1_500, "The mob is down", "Raidlead", "!stopchain");
    script.finish()
}

fn mistakes_steps() -> Vec<DemoStep> {
    let roster = [("YOU", 1u32), ("Portlia", 2), ("Dunkin", 3)];
    let interval = interval_ms(roster.len());
    let cycle = cycle_ms(roster.len());
    let mut script = Script::new();

    script.command(
        600,
        "Main tank again for a fresh pull",
        "Raidlead",
        "!mt Mluian",
    );
    script.pace(1_200, roster.len(), None);
    script.command(2_000, "You take 001", "YOU", "!take 001");
    script.command(2_600, "Portlia takes 002", "Portlia", "!take 002");
    script.command(
        3_400,
        "Dunkin grabs a number that is already yours — watch the alert",
        "Dunkin",
        "!take 001",
    );
    script.command(
        4_800,
        "Dunkin moves to a free number",
        "Dunkin",
        "!take 003",
    );
    script.command(5_600, "The chain arms", "Raidlead", "!startchain");

    let first = 7_000;
    let at = |round: u64, index: u64| first + round * cycle + index * interval;
    script.cast(at(0, 0), "You open the chain", "YOU", "001", "CH", "Mluian");
    script.cast(
        at(0, 1),
        "Portlia follows",
        "Portlia",
        "002",
        "CH",
        "Mluian",
    );
    script.cast(at(0, 2), "Dunkin follows", "Dunkin", "003", "CH", "Mluian");
    script.cast(
        at(1, 0),
        "You CH the wrong target — Alfred calls it out",
        "YOU",
        "001",
        "CH",
        "Beefwich",
    );
    script.cast(
        at(1, 1),
        "Portlia stays on the beat",
        "Portlia",
        "002",
        "CH",
        "Mluian",
    );
    script.say(
        at(1, 2),
        "Dunkin sends a macro Alfred cannot read, so his turn is missed",
        "Dunkin",
        TestChannel::Shout,
        "GG CH -- Mluian".into(),
    );
    script.cast(at(2, 0), "You keep going", "YOU", "001", "CH", "Mluian");
    script.cast(
        at(2, 1),
        "Portlia keeps going",
        "Portlia",
        "002",
        "CH",
        "Mluian",
    );
    script.cast(
        at(2, 2),
        "Dunkin fixes his macro and rejoins",
        "Dunkin",
        "003",
        "CH",
        "Mluian",
    );

    let short = at(2, 2) + 1_000;
    script.command(short, "Portlia steps out to med", "Portlia", "!skip 002");
    script.pace(short + 700, roster.len() - 1, None);

    // Two clerics left, so the survivors stretch to five seconds apart.
    let pair = interval_ms(2);
    let resume = short + 2_400;
    script.cast(resume, "You cover the gap", "YOU", "001", "CH", "Mluian");
    script.cast(
        resume + pair,
        "Dunkin covers the gap",
        "Dunkin",
        "003",
        "CH",
        "Mluian",
    );
    let back = script.cast(
        resume + 2 * pair,
        "Back to you",
        "YOU",
        "001",
        "CH",
        "Mluian",
    );

    script.command(back + 900, "Portlia is back", "Portlia", "!back 002");
    script.pace(back + 1_600, roster.len(), None);
    script.command(
        back + 2_400,
        "The lead swaps two numbers",
        "Raidlead",
        "!move 001 003",
    );
    script.command(back + 3_600, "Pull is over", "Raidlead", "!stopchain");
    script.finish()
}

fn rampage_steps() -> Vec<DemoStep> {
    let roster = [("YOU", 1u32), ("Portlia", 2), ("Bellamere", 3)];
    let interval = interval_ms(roster.len());
    let cycle = cycle_ms(roster.len());
    let mut script = Script::new();

    script.command(600, "Rampage tank", "Raidlead", "!rt Mluian");
    script.say(
        1_200,
        &format!("Rampage: {}", pace_note(roster.len())),
        "Raidlead",
        TestChannel::Guild,
        format!("!rchain {}", interval_seconds(roster.len())),
    );
    script.command(2_000, "You take the first letter", "YOU", "!take AAA");
    script.command(2_600, "Portlia takes BBB", "Portlia", "!take BBB");
    script.command(3_200, "Bellamere takes CCC", "Bellamere", "!take CCC");
    script.command(4_000, "Arm both chains", "Raidlead", "!startchain");

    let first = 5_400;
    let mut last = first;
    for round in 0..2u64 {
        for (index, (player, number)) in roster.iter().enumerate() {
            last = script.cast(
                first + round * cycle + index as u64 * interval,
                &format!(
                    "{player} casts {} on round {}",
                    rch_slot(*number),
                    round + 1
                ),
                player,
                &rch_slot(*number),
                "RCH",
                "Mluian",
            );
        }
    }
    script.command(last + 1_500, "Rampage is off", "Raidlead", "!stopchain");
    script.finish()
}

fn split_steps() -> Vec<DemoStep> {
    let main = [("YOU", 1u32), ("Portlia", 2)];
    let off = [("Bellamere", 5u32), ("Curaja", 6)];
    let interval = interval_ms(main.len());
    let cycle = cycle_ms(main.len());
    let mut script = Script::new();

    script.command(600, "Main tank", "Raidlead", "!mt Mluian");
    script.command(1_200, "Off tank", "Raidlead", "!ot Beefwich");
    script.command(
        1_900,
        "Numbers from 005 up heal the off tank",
        "Raidlead",
        "!split 005",
    );
    script.command(2_600, "You stay on the main tank", "YOU", "!take 001");
    script.command(3_200, "Portlia is on the main tank", "Portlia", "!take 002");
    script.command(
        3_800,
        "Bellamere covers the off tank",
        "Bellamere",
        "!take 005",
    );
    script.command(4_400, "Curaja covers the off tank", "Curaja", "!take 006");
    script.pace(5_200, main.len(), None);
    script.pace(5_900, off.len(), Some("ot"));
    script.command(6_600, "Arm both tanks", "Raidlead", "!startchain");

    // Each tank keeps its own clock, so the two pairs interleave.
    let first = 8_000;
    let stagger = 900;
    let mut last = first;
    for round in 0..2u64 {
        for (index, (player, number)) in main.iter().enumerate() {
            let at = first + round * cycle + index as u64 * interval;
            script.cast(
                at,
                &format!("{player} heals Mluian on round {}", round + 1),
                player,
                &ch_slot(*number),
                "CH",
                "Mluian",
            );
            let (off_player, off_number) = off[index];
            let wrong = round == 1 && index == 0;
            let target = if wrong { "Mluian" } else { "Beefwich" };
            let note = if wrong {
                format!("{off_player} CHes the main tank by mistake")
            } else {
                format!("{off_player} heals Beefwich on round {}", round + 1)
            };
            last = script.cast(
                at + stagger,
                &note,
                off_player,
                &ch_slot(off_number),
                "CH",
                target,
            );
        }
    }
    script.command(
        last + 1_800,
        "Both tanks are safe",
        "Raidlead",
        "!stopchain",
    );
    script.finish()
}

/// One cleric who is always a little later than the last time, until he is
/// late enough to lose his turn entirely.
fn laggy_steps() -> Vec<DemoStep> {
    let roster = [("YOU", 1u32), ("Portlia", 2), ("Dunkin", 3), ("Curaja", 4)];
    let interval = interval_ms(roster.len());
    let cycle = cycle_ms(roster.len());
    let mut script = Script::new();

    script.command(
        600,
        "The raid lead names the main tank",
        "Raidlead",
        "!mt Mluian",
    );
    script.pace(1_200, roster.len(), None);
    for (index, (player, number)) in roster.iter().enumerate() {
        script.command(
            2_000 + index as u64 * 600,
            &format!("{player} takes {}", ch_slot(*number)),
            player,
            &format!("!take {}", ch_slot(*number)),
        );
    }
    script.command(4_800, "The chain arms", "Raidlead", "!startchain");

    let first = 6_200;
    // Dunkin's lag grows every round: on the beat, then late, then very late.
    let drift = [0u64, 1_200, 2_600, 3_900];
    let mut last = first;
    for round in 0..4u64 {
        for (index, (player, number)) in roster.iter().enumerate() {
            let laggy = *player == "Dunkin";
            // The fourth round is the one he misses outright.
            if laggy && round == 3 {
                continue;
            }
            let slack = if laggy { drift[round as usize] } else { 0 };
            let note = if laggy && slack > 0 {
                format!(
                    "Dunkin is {:.1}s late on {}",
                    slack as f64 / 1000.0,
                    ch_slot(*number)
                )
            } else {
                format!("{player} casts {} on round {}", ch_slot(*number), round + 1)
            };
            last = script.cast(
                first + round * cycle + index as u64 * interval + slack,
                &note,
                player,
                &ch_slot(*number),
                "CH",
                "Mluian",
            );
        }
    }

    script.command(
        last + 1_200,
        "Dunkin missed his turn, so the lead pulls him out",
        "Raidlead",
        "!skip 003",
    );
    script.pace(last + 1_900, roster.len() - 1, None);

    // Three left, so the survivors stretch out to 3.4s apart.
    let thinner = interval_ms(3);
    let resume = last + 3_400;
    let short = [("YOU", 1u32), ("Portlia", 2), ("Curaja", 4)];
    let mut last = resume;
    for round in 0..2u64 {
        for (index, (player, number)) in short.iter().enumerate() {
            last = script.cast(
                resume + round * (thinner * 3) + index as u64 * thinner,
                &format!("{player} covers the gap on {}", ch_slot(*number)),
                player,
                &ch_slot(*number),
                "CH",
                "Mluian",
            );
        }
    }

    script.command(
        last + 1_000,
        "Dunkin says he is ready again",
        "Dunkin",
        "!back 003",
    );
    script.pace(last + 1_700, roster.len(), None);
    script.cast(
        last + 3_000,
        "Dunkin is back on the beat",
        "Dunkin",
        "003",
        "CH",
        "Mluian",
    );
    script.command(last + 4_600, "The mob is down", "Raidlead", "!stopchain");
    script.finish()
}

/// A split chain where clerics keep healing across the split.
fn wrongtank_steps() -> Vec<DemoStep> {
    let main = [("YOU", 1u32), ("Portlia", 2)];
    let off = [("Bellamere", 3u32), ("Curaja", 4)];
    let interval = interval_ms(2);
    let cycle = cycle_ms(2);
    let mut script = Script::new();

    script.command(600, "Main tank", "Raidlead", "!mt Mluian");
    script.command(1_200, "Off tank", "Raidlead", "!ot Beefwich");
    script.command(
        1_800,
        "003 and up are on the off tank",
        "Raidlead",
        "!split 003",
    );
    for (index, (player, number)) in main.iter().chain(off.iter()).enumerate() {
        script.command(
            2_400 + index as u64 * 600,
            &format!("{player} takes {}", ch_slot(*number)),
            player,
            &format!("!take {}", ch_slot(*number)),
        );
    }
    script.pace(5_000, main.len(), None);
    script.pace(5_600, off.len(), Some("ot"));
    script.command(6_200, "Arm both tanks", "Raidlead", "!startchain");

    let first = 7_600;
    for round in 0..3u64 {
        let base = first + round * cycle;
        // Round two is where both sides heal the wrong side of the split.
        let swap = round == 1;
        for (index, (player, number)) in main.iter().enumerate() {
            let target = if swap && index == 0 {
                "Beefwich"
            } else {
                "Mluian"
            };
            let note = if target == "Beefwich" {
                format!("{player} CHes the off tank from a main tank number")
            } else {
                format!("{player} heals Mluian on round {}", round + 1)
            };
            script.cast(
                base + index as u64 * interval,
                &note,
                player,
                &ch_slot(*number),
                "CH",
                target,
            );
        }
        for (index, (player, number)) in off.iter().enumerate() {
            let target = if swap && index == 1 {
                "Mluian"
            } else {
                "Beefwich"
            };
            let note = if target == "Mluian" {
                format!("{player} CHes the main tank from an off tank number")
            } else {
                format!("{player} heals Beefwich on round {}", round + 1)
            };
            script.cast(
                base + index as u64 * interval + 900,
                &note,
                player,
                &ch_slot(*number),
                "CH",
                target,
            );
        }
    }
    script.command(
        first + 3 * cycle,
        "The lead calls the tanks out again",
        "Raidlead",
        "!ot Beefwich",
    );
    script.command(
        first + 3 * cycle + 1_500,
        "Both tanks live",
        "Raidlead",
        "!stopchain",
    );
    script.finish()
}

/// The tank dies and the chain has to follow a new one.
fn tankswap_steps() -> Vec<DemoStep> {
    let roster = [("YOU", 1u32), ("Portlia", 2), ("Bellamere", 3)];
    let interval = interval_ms(roster.len());
    let cycle = cycle_ms(roster.len());
    let mut script = Script::new();

    script.command(600, "Mluian takes the mob", "Raidlead", "!mt Mluian");
    script.pace(1_200, roster.len(), None);
    for (index, (player, number)) in roster.iter().enumerate() {
        script.command(
            2_000 + index as u64 * 600,
            &format!("{player} takes {}", ch_slot(*number)),
            player,
            &format!("!take {}", ch_slot(*number)),
        );
    }
    script.command(4_200, "The chain arms", "Raidlead", "!startchain");

    let first = 5_600;
    for (index, (player, number)) in roster.iter().enumerate() {
        script.cast(
            first + index as u64 * interval,
            &format!("{player} heals Mluian"),
            player,
            &ch_slot(*number),
            "CH",
            "Mluian",
        );
    }

    let swap = first + cycle - 1_200;
    script.command(
        swap,
        "Mluian goes down, so Beefwich is the new main tank",
        "Raidlead",
        "!mt Beefwich",
    );

    let after = first + cycle;
    for round in 0..2u64 {
        for (index, (player, number)) in roster.iter().enumerate() {
            // Bellamere's macro still points at the tank that died.
            let stale = *player == "Bellamere" && round == 0;
            let target = if stale { "Mluian" } else { "Beefwich" };
            let note = if stale {
                format!(
                    "{player} is still healing the old tank on {}",
                    ch_slot(*number)
                )
            } else {
                format!("{player} heals Beefwich on {}", ch_slot(*number))
            };
            script.cast(
                after + round * cycle + index as u64 * interval,
                &note,
                player,
                &ch_slot(*number),
                "CH",
                target,
            );
        }
    }
    script.command(
        after + 2 * cycle,
        "Beefwich holds it to the end",
        "Raidlead",
        "!stopchain",
    );
    script.finish()
}

/// One rotation becomes two in the middle of a pull, then goes back to one.
fn offtank_steps() -> Vec<DemoStep> {
    let roster = [
        ("YOU", 1u32),
        ("Portlia", 2),
        ("Bellamere", 3),
        ("Curaja", 4),
    ];
    let interval = interval_ms(roster.len());
    let cycle = cycle_ms(roster.len());
    let mut script = Script::new();

    script.command(
        600,
        "Everyone is on the main tank",
        "Raidlead",
        "!mt Mluian",
    );
    script.pace(1_200, roster.len(), None);
    for (index, (player, number)) in roster.iter().enumerate() {
        script.command(
            2_000 + index as u64 * 500,
            &format!("{player} takes {}", ch_slot(*number)),
            player,
            &format!("!take {}", ch_slot(*number)),
        );
    }
    script.command(4_400, "The chain arms", "Raidlead", "!startchain");

    let first = 5_800;
    for (index, (player, number)) in roster.iter().enumerate() {
        script.cast(
            first + index as u64 * interval,
            &format!("{player} heals Mluian on round 1"),
            player,
            &ch_slot(*number),
            "CH",
            "Mluian",
        );
    }

    let split_at = first + cycle - 1_400;
    script.command(
        split_at,
        "An add lands on Beefwich",
        "Raidlead",
        "!ot Beefwich",
    );
    script.command(
        split_at + 600,
        "003 and 004 move over to the off tank",
        "Raidlead",
        "!split 003",
    );
    script.pace(split_at + 1_200, 2, None);
    script.pace(split_at + 1_800, 2, Some("ot"));

    // Two clerics a side, so both rotations run 5s apart on their own clocks.
    let pair = interval_ms(2);
    let after = first + cycle + 1_000;
    for round in 0..2u64 {
        let base = after + round * cycle_ms(2);
        script.cast(base, "You hold the main tank", "YOU", "001", "CH", "Mluian");
        script.cast(
            base + 400,
            "Bellamere picks up the off tank",
            "Bellamere",
            "003",
            "CH",
            "Beefwich",
        );
        script.cast(
            base + pair,
            "Portlia holds the main tank",
            "Portlia",
            "002",
            "CH",
            "Mluian",
        );
        script.cast(
            base + pair + 400,
            "Curaja holds the off tank",
            "Curaja",
            "004",
            "CH",
            "Beefwich",
        );
    }

    let fold = after + 2 * cycle_ms(2);
    script.command(fold, "The add is dead", "Raidlead", "!untank Beefwich");
    script.pace(fold + 700, roster.len(), None);
    script.command(
        fold + 2_000,
        "One tank again, then done",
        "Raidlead",
        "!stopchain",
    );
    script.finish()
}

/// A CH chain and a rampage chain running side by side.
fn rampagetank_steps() -> Vec<DemoStep> {
    let heals = [("YOU", 1u32), ("Portlia", 2), ("Bellamere", 3)];
    let rampage = [("Curaja", 1u32), ("Dunkin", 2), ("Hanbox", 3)];
    let interval = interval_ms(3);
    let cycle = cycle_ms(3);
    let mut script = Script::new();

    script.command(600, "Main tank on the boss", "Raidlead", "!mt Mluian");
    script.command(1_200, "Rampage tank on the adds", "Raidlead", "!rt Grendel");
    script.pace(1_800, heals.len(), None);
    script.say(
        2_400,
        &format!("Rampage: {}", pace_note(rampage.len())),
        "Raidlead",
        TestChannel::Guild,
        format!("!rchain {}", interval_seconds(rampage.len())),
    );
    for (index, (player, number)) in heals.iter().enumerate() {
        script.command(
            3_000 + index as u64 * 500,
            &format!("{player} takes {} on the boss", ch_slot(*number)),
            player,
            &format!("!take {}", ch_slot(*number)),
        );
    }
    for (index, (player, number)) in rampage.iter().enumerate() {
        script.command(
            4_600 + index as u64 * 500,
            &format!("{player} takes {} on rampage", rch_slot(*number)),
            player,
            &format!("!take {}", rch_slot(*number)),
        );
    }
    script.command(6_400, "Arm both chains", "Raidlead", "!startchain");

    let first = 7_800;
    let mut last = first;
    for round in 0..2u64 {
        for (index, (player, number)) in heals.iter().enumerate() {
            script.cast(
                first + round * cycle + index as u64 * interval,
                &format!("{player} CHes the boss tank on round {}", round + 1),
                player,
                &ch_slot(*number),
                "CH",
                "Mluian",
            );
        }
        for (index, (player, number)) in rampage.iter().enumerate() {
            // One rampage heal lands on the boss tank instead of the rampage tank.
            let wrong = round == 1 && index == 2;
            let target = if wrong { "Mluian" } else { "Grendel" };
            let note = if wrong {
                format!("{player} RCHes the boss tank by mistake")
            } else {
                format!("{player} RCHes Grendel on round {}", round + 1)
            };
            last = script.cast(
                first + round * cycle + index as u64 * interval + 700,
                &note,
                player,
                &rch_slot(*number),
                "RCH",
                target,
            );
        }
    }
    script.command(
        last + 1_600,
        "Adds and boss are done",
        "Raidlead",
        "!stopchain",
    );
    script.finish()
}

/// Everyone wants the same numbers.
fn collision_steps() -> Vec<DemoStep> {
    let mut script = Script::new();

    script.command(600, "Main tank", "Raidlead", "!mt Mluian");
    script.pace(1_200, 4, None);
    script.command(2_000, "You take 001", "YOU", "!take 001");
    script.command(
        2_700,
        "Portlia asks for 001 as well — watch the alert",
        "Portlia",
        "!take 001",
    );
    script.command(
        3_600,
        "Portlia takes whatever is free instead",
        "Portlia",
        "!take",
    );
    script.command(
        4_400,
        "Bellamere asks for 002, which Portlia just took",
        "Bellamere",
        "!take 002",
    );
    script.command(5_200, "Bellamere settles on 003", "Bellamere", "!take 003");
    script.command(
        6_000,
        "The lead signs Curaja up for 004 himself",
        "Raidlead",
        "!take 004 Curaja",
    );
    script.command(6_800, "The chain arms", "Raidlead", "!startchain");

    let interval = interval_ms(4);
    let cycle = cycle_ms(4);
    let first = 8_200;
    let roster = [
        ("YOU", 1u32),
        ("Portlia", 2),
        ("Bellamere", 3),
        ("Curaja", 4),
    ];
    for (index, (player, number)) in roster.iter().enumerate() {
        script.cast(
            first + index as u64 * interval,
            &format!("{player} casts {}", ch_slot(*number)),
            player,
            &ch_slot(*number),
            "CH",
            "Mluian",
        );
    }

    let second = first + cycle;
    script.cast(
        second,
        "Bellamere shouts your number instead of his own",
        "Bellamere",
        "001",
        "CH",
        "Mluian",
    );
    script.cast(
        second + interval,
        "You cast your own number",
        "YOU",
        "001",
        "CH",
        "Mluian",
    );
    script.cast(
        second + 2 * interval,
        "Portlia carries on",
        "Portlia",
        "002",
        "CH",
        "Mluian",
    );
    script.cast(
        second + 3 * interval,
        "Curaja carries on",
        "Curaja",
        "004",
        "CH",
        "Mluian",
    );
    script.command(
        second + cycle,
        "Sorted out, mob is down",
        "Raidlead",
        "!stopchain",
    );
    script.finish()
}

/// The pull is already going when you get there, so Alfred has to work the
/// pace out from the shouts it can hear.
fn latejoin_steps() -> Vec<DemoStep> {
    let interval = interval_ms(3);
    let mut script = Script::new();

    script.command(
        800,
        "You load in mid-pull and all Alfred knows is who the tank is",
        "Raidlead",
        "!mt Mluian",
    );

    // Two rounds of shouts with no !chain and no !startchain to go on.
    let first = 2_000;
    let heard = [("Portlia", 2u32), ("Bellamere", 3), ("Curaja", 4)];
    for round in 0..2u64 {
        for (index, (player, number)) in heard.iter().enumerate() {
            script.cast(
                first + round * (interval * 3) + index as u64 * interval,
                &format!(
                    "{player} shouts {} — Alfred measures the gap",
                    ch_slot(*number)
                ),
                player,
                &ch_slot(*number),
                "CH",
                "Mluian",
            );
        }
    }

    let join = first + 2 * interval * 3;
    script.command(join, "You take the open number", "YOU", "!take 001");
    script.cast(
        join + 1_200,
        "Your first CH syncs you behind the last shouter",
        "YOU",
        "001",
        "CH",
        "Mluian",
    );

    let settled = join + 2_000;
    for (index, (player, number)) in heard.iter().enumerate() {
        script.cast(
            settled + index as u64 * interval,
            &format!("{player} keeps the rotation going"),
            player,
            &ch_slot(*number),
            "CH",
            "Mluian",
        );
    }

    let formal = settled + 4 * interval;
    script.pace(formal, 4, None);
    script.command(
        formal + 700,
        "The lead finally calls the start, which re-anchors everyone",
        "Raidlead",
        "!startchain",
    );
    script.cast(
        formal + 2_000,
        "You open the fresh clock",
        "YOU",
        "001",
        "CH",
        "Mluian",
    );
    script.command(formal + 4_000, "Mob is down", "Raidlead", "!stopchain");
    script.finish()
}

/// Not enough clerics for a comfortable chain.
fn thin_steps() -> Vec<DemoStep> {
    let mut script = Script::new();
    let pair = interval_ms(2);

    script.command(600, "Main tank", "Raidlead", "!mt Mluian");
    script.pace(1_200, 2, None);
    script.command(1_900, "You take 001", "YOU", "!take 001");
    script.command(2_500, "Portlia takes 002", "Portlia", "!take 002");
    script.command(3_200, "Two clerics, 5s apart", "Raidlead", "!startchain");

    let first = 4_400;
    for round in 0..2u64 {
        script.cast(
            first + round * pair * 2,
            "You cast on the 5s beat",
            "YOU",
            "001",
            "CH",
            "Mluian",
        );
        script.cast(
            first + round * pair * 2 + pair,
            "Portlia answers",
            "Portlia",
            "002",
            "CH",
            "Mluian",
        );
    }

    let alone = first + 2 * pair * 2;
    script.command(alone, "Portlia goes down", "Portlia", "!skip 002");
    script.pace(alone + 700, 1, None);
    // A solo chain can only go as fast as the spell, so the bar runs on cast time.
    for round in 0..2u64 {
        script.cast(
            alone + 2_000 + round * (CAST_SECONDS * 1000.0) as u64,
            "You are the whole chain now",
            "YOU",
            "001",
            "CH",
            "Mluian",
        );
    }

    let help = alone + 2_000 + 2 * (CAST_SECONDS * 1000.0) as u64;
    script.command(help, "Bellamere runs in", "Bellamere", "!take 003");
    script.command(help + 700, "Portlia is back up", "Portlia", "!back 002");
    script.pace(help + 1_400, 3, None);

    let three = interval_ms(3);
    script.cast(
        help + 2_600,
        "You lead the new rotation",
        "YOU",
        "001",
        "CH",
        "Mluian",
    );
    script.cast(
        help + 2_600 + three,
        "Portlia follows",
        "Portlia",
        "002",
        "CH",
        "Mluian",
    );
    script.cast(
        help + 2_600 + 2 * three,
        "Bellamere closes the round",
        "Bellamere",
        "003",
        "CH",
        "Mluian",
    );
    script.command(
        help + 2_600 + 4 * three,
        "It holds, mob down",
        "Raidlead",
        "!stopchain",
    );
    script.finish()
}

/// Clerics drop out until there is nothing left to pace.
fn collapse_steps() -> Vec<DemoStep> {
    let roster = [
        ("YOU", 1u32),
        ("Portlia", 2),
        ("Bellamere", 3),
        ("Curaja", 4),
        ("Dunkin", 5),
    ];
    let interval = interval_ms(roster.len());
    let mut script = Script::new();

    script.command(600, "Main tank", "Raidlead", "!mt Mluian");
    script.pace(1_200, roster.len(), None);
    for (index, (player, number)) in roster.iter().enumerate() {
        script.command(
            1_900 + index as u64 * 500,
            &format!("{player} takes {}", ch_slot(*number)),
            player,
            &format!("!take {}", ch_slot(*number)),
        );
    }
    script.command(4_600, "Five clerics, 2s apart", "Raidlead", "!startchain");

    let first = 5_800;
    for (index, (player, number)) in roster.iter().enumerate() {
        script.cast(
            first + index as u64 * interval,
            &format!("{player} casts {} on the full chain", ch_slot(*number)),
            player,
            &ch_slot(*number),
            "CH",
            "Mluian",
        );
    }

    // Each loss stretches the interval for whoever is left.
    let mut at = first + cycle_ms(roster.len());
    let mut alive: Vec<(&str, u32)> = roster.to_vec();
    for gone in ["Dunkin", "Curaja", "Bellamere"] {
        let number = alive
            .iter()
            .find(|(player, _)| *player == gone)
            .map(|(_, number)| *number)
            .expect("still on the chain");
        script.command(
            at,
            &format!("{gone} goes down"),
            gone,
            &format!("!skip {}", ch_slot(number)),
        );
        alive.retain(|(player, _)| *player != gone);
        script.pace(at + 600, alive.len(), None);
        let spacing = interval_ms(alive.len());
        for (index, (player, number)) in alive.iter().enumerate() {
            script.cast(
                at + 1_800 + index as u64 * spacing,
                &format!(
                    "{player} holds {} with {} left",
                    ch_slot(*number),
                    alive.len()
                ),
                player,
                &ch_slot(*number),
                "CH",
                "Mluian",
            );
        }
        at += 1_800 + cycle_ms(alive.len());
    }

    script.command(
        at,
        "Two clerics cannot hold it, so the lead calls the chain off",
        "Raidlead",
        "!stopchain",
    );
    script.finish()
}

/// Macros Alfred cannot make sense of. Every one of these is deliberate.
fn macros_steps() -> Vec<DemoStep> {
    let roster = [("YOU", 1u32), ("Portlia", 2), ("Bellamere", 3)];
    let interval = interval_ms(roster.len());
    let cycle = cycle_ms(roster.len());
    let mut script = Script::new();

    script.command(600, "Main tank", "Raidlead", "!mt Mluian");
    script.command(1_100, "Rampage tank too", "Raidlead", "!rt Grendel");
    script.pace(1_700, roster.len(), None);
    for (index, (player, number)) in roster.iter().enumerate() {
        script.command(
            2_400 + index as u64 * 600,
            &format!("{player} takes {}", ch_slot(*number)),
            player,
            &format!("!take {}", ch_slot(*number)),
        );
    }
    script.command(4_600, "The chain arms", "Raidlead", "!startchain");

    let first = 6_000;
    for (index, (player, number)) in roster.iter().enumerate() {
        script.cast(
            first + index as u64 * interval,
            &format!("{player} casts {} cleanly", ch_slot(*number)),
            player,
            &ch_slot(*number),
            "CH",
            "Mluian",
        );
    }

    // The second round comes around and two of the three macros are wrong.
    // Each of these clerics really did cast, so the lines still sit a full
    // cast apart from their last one.
    let broken = first + cycle;
    script.cast(broken, "You cast normally", "YOU", "001", "CH", "Mluian");
    script.say(
        broken + interval,
        "Portlia's macro has no number, so Alfred cannot read it",
        "Portlia",
        TestChannel::Shout,
        "GG CH -- Mluian".into(),
    );
    script.say(
        broken + 2 * interval,
        "Bellamere put the number after the spell, so Alfred cannot read it",
        "Bellamere",
        TestChannel::Shout,
        "GG CH 003 -- Mluian".into(),
    );
    script.say(
        broken + 2 * interval + 900,
        "Curaja sends a rampage macro with no letter, so Alfred cannot read it",
        "Curaja",
        TestChannel::Shout,
        "GG RCH -- Grendel".into(),
    );

    let fixed = broken + cycle;
    script.cast(
        fixed,
        "You keep the chain alive",
        "YOU",
        "001",
        "CH",
        "Mluian",
    );
    script.cast(
        fixed + interval,
        "Portlia fixes her macro",
        "Portlia",
        "002",
        "CH",
        "Mluian",
    );
    script.cast(
        fixed + 2 * interval,
        "Bellamere fixes his macro",
        "Bellamere",
        "003",
        "CH",
        "Mluian",
    );
    script.command(fixed + cycle, "Mob is down", "Raidlead", "!stopchain");
    script.finish()
}

/// The lead rearranges the chain while it is running, then starts over.
fn shuffle_steps() -> Vec<DemoStep> {
    let roster = [
        ("YOU", 1u32),
        ("Portlia", 2),
        ("Bellamere", 3),
        ("Curaja", 4),
    ];
    let interval = interval_ms(roster.len());
    let cycle = cycle_ms(roster.len());
    let mut script = Script::new();

    script.command(600, "Main tank", "Raidlead", "!mt Mluian");
    script.pace(1_200, roster.len(), None);
    for (index, (player, number)) in roster.iter().enumerate() {
        script.command(
            1_900 + index as u64 * 500,
            &format!("{player} takes {}", ch_slot(*number)),
            player,
            &format!("!take {}", ch_slot(*number)),
        );
    }
    script.command(4_200, "The chain arms", "Raidlead", "!startchain");

    let first = 5_600;
    for (index, (player, number)) in roster.iter().enumerate() {
        script.cast(
            first + index as u64 * interval,
            &format!("{player} casts {}", ch_slot(*number)),
            player,
            &ch_slot(*number),
            "CH",
            "Mluian",
        );
    }

    let moved = first + cycle;
    script.command(
        moved,
        "The lead swaps you with Curaja",
        "Raidlead",
        "!move 001 004",
    );
    script.command(
        moved + 800,
        "Bellamere is asked to sit out for a heal",
        "Raidlead",
        "!skip 003",
    );
    script.pace(moved + 1_500, 3, None);

    let three = interval_ms(3);
    let after = moved + 2_800;
    script.cast(
        after,
        "Curaja now leads on 001",
        "Curaja",
        "001",
        "CH",
        "Mluian",
    );
    script.cast(
        after + three,
        "Portlia keeps 002",
        "Portlia",
        "002",
        "CH",
        "Mluian",
    );
    script.cast(
        after + 2 * three,
        "You cast from 004 now",
        "YOU",
        "004",
        "CH",
        "Mluian",
    );

    let back = after + 3 * three;
    script.command(back, "Bellamere comes back", "Bellamere", "!back 003");
    script.pace(back + 700, roster.len(), None);
    script.command(
        back + 2_000,
        "The lead wipes the board and starts the next pull clean",
        "Raidlead",
        "!reset-chain",
    );
    script.pace(back + 3_000, 2, None);
    script.command(back + 3_600, "You take 001 again", "YOU", "!take 001");
    script.command(
        back + 4_200,
        "Portlia takes 002 again",
        "Portlia",
        "!take 002",
    );
    script.command(back + 4_800, "Fresh start", "Raidlead", "!startchain");
    let pair = interval_ms(2);
    script.cast(
        back + 6_000,
        "You open the new chain",
        "YOU",
        "001",
        "CH",
        "Mluian",
    );
    script.cast(
        back + 6_000 + pair,
        "Portlia answers",
        "Portlia",
        "002",
        "CH",
        "Mluian",
    );
    script.command(back + 6_000 + 3 * pair, "Done", "Raidlead", "!stopchain");
    script.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{format_test_log_line, ParseResult, Parser};

    #[test]
    fn the_interval_is_the_cast_time_split_between_the_clerics() {
        assert_eq!(interval_seconds(1), 10.0);
        assert_eq!(interval_seconds(2), 5.0);
        assert_eq!(interval_seconds(3), 3.4);
        assert_eq!(interval_seconds(4), 2.5);
        assert_eq!(interval_seconds(6), 1.7);
        assert_eq!(interval_seconds(0), CAST_SECONDS);
        // Rounding up keeps a full cast between two turns of the same cleric.
        for clerics in 1..=10 {
            assert!(
                interval_seconds(clerics) * clerics as f64 >= CAST_SECONDS,
                "{clerics} clerics chain faster than a CH takes"
            );
        }
    }

    #[test]
    fn no_cleric_is_asked_to_cast_before_their_heal_lands() {
        for id in SCENARIO_IDS {
            let mut at = 0u64;
            let mut last_cast: BTreeMap<String, u64> = BTreeMap::new();
            for step in scenario_steps(id) {
                at += step.delay_ms;
                if !step.message.contains(" CH ") && !step.message.contains(" RCH ") {
                    continue;
                }
                if let Some(previous) = last_cast.insert(step.speaker.clone(), at) {
                    let gap = (at - previous) as f64 / 1000.0;
                    assert!(
                        gap >= CAST_SECONDS,
                        "{id}: {} casts again after {gap}s",
                        step.speaker
                    );
                }
            }
        }
    }

    #[test]
    fn demo_options_are_clamped_to_something_runnable() {
        let wild = DemoOptions {
            clerics: 400,
            max_clerics: 400,
            minutes: 9_000,
        }
        .normalized();
        assert_eq!(wild.clerics, MAX_CLERICS);
        assert_eq!(wild.max_clerics, MAX_CLERICS);
        assert_eq!(wild.minutes, MAX_MINUTES);

        let tiny = DemoOptions {
            clerics: 0,
            max_clerics: 0,
            minutes: 0,
        }
        .normalized();
        assert_eq!(tiny.clerics, 1);
        assert_eq!(tiny.max_clerics, 1);
        assert_eq!(tiny.minutes, 1);

        // A roster cannot shrink below the clerics who are already standing there.
        let backwards = DemoOptions {
            clerics: 12,
            max_clerics: 4,
            minutes: 2,
        }
        .normalized();
        assert_eq!(backwards.clerics, 12);
        assert_eq!(backwards.max_clerics, 12);
    }

    #[test]
    fn the_raid_scenario_follows_the_roster_and_length_you_ask_for() {
        let options = DemoOptions {
            clerics: 8,
            max_clerics: 20,
            minutes: 4,
        };
        let steps = scenario_steps_with("raid", options);
        let takes = steps
            .iter()
            .filter(|s| s.message.starts_with("!take"))
            .count();
        assert_eq!(takes, 20, "everyone takes a number eventually");
        assert_eq!(steps.iter().filter(|s| s.message == "!take 001").count(), 1);
        // Eight take numbers at the pull; the rest wander in after it starts.
        let started = steps
            .iter()
            .position(|s| s.message == "!startchain")
            .expect("the chain arms");
        let early = steps[..started]
            .iter()
            .filter(|s| s.message.starts_with("!take"))
            .count();
        assert_eq!(early, 8);

        let yours: Vec<&DemoStep> = steps.iter().filter(|s| s.speaker == "YOU").collect();
        assert!(!yours.is_empty(), "you are on the chain");
        assert!(
            yours.iter().any(|s| s.message.contains(" CH ")),
            "you cast during the pull"
        );

        let described = describe("raid", options);
        assert!(described.configurable);
        assert_eq!(described.clerics, 20);
        // Twenty clerics would split the 10s cast to half a second, but a demo
        // chain never beats faster than a second.
        assert_eq!(described.interval_seconds, MIN_INTERVAL_SECONDS);
        // Four minutes of chain, plus taking numbers and the kill.
        assert!(described.seconds > 240.0, "{} seconds", described.seconds);
        assert!(described.seconds < 300.0, "{} seconds", described.seconds);

        let short = describe(
            "raid",
            DemoOptions {
                minutes: 1,
                ..options
            },
        );
        assert!(short.seconds < described.seconds);
        assert!(!describe("chain", options).configurable);
    }

    #[test]
    fn clerics_join_and_the_lead_re_paces_the_chain_as_they_do() {
        let steps = scenario_steps_with(
            "raid",
            DemoOptions {
                clerics: 4,
                max_clerics: 12,
                minutes: 5,
            },
        );
        let started = steps
            .iter()
            .position(|s| s.message == "!startchain")
            .expect("the chain arms");
        let late_takes = steps[started..]
            .iter()
            .filter(|s| s.message.starts_with("!take"))
            .count();
        assert_eq!(late_takes, 8, "the other eight join mid-pull");
        // Each roster change is re-called, so the interval tightens as they arrive.
        let paces: Vec<&String> = steps
            .iter()
            .filter(|s| s.message.starts_with("!chain "))
            .map(|s| &s.message)
            .collect();
        assert!(paces.len() > 8, "{} pace calls", paces.len());
        assert_eq!(paces[0], "!chain 2.5", "four clerics start 2.5s apart");
        assert!(
            paces.iter().any(|m| *m == "!chain 1"),
            "twelve clerics hold the 1s floor: {paces:?}"
        );
    }

    #[test]
    fn a_crowded_chain_stops_tightening_at_a_second() {
        assert_eq!(interval_seconds(10), 1.0);
        // Past ten clerics the cast would split finer than a second, and does not.
        assert_eq!(interval_seconds(11), MIN_INTERVAL_SECONDS);
        assert_eq!(interval_seconds(MAX_CLERICS), MIN_INTERVAL_SECONDS);
        // A thinner chain still paces off the cast time.
        assert_eq!(interval_seconds(4), 2.5);
        assert_eq!(interval_seconds(1), CAST_SECONDS);

        // No scenario, configurable or scripted, calls a pace under a second.
        let mut ids: Vec<&str> = SCENARIO_IDS.to_vec();
        ids.push("raid");
        for id in ids {
            let steps = scenario_steps_with(
                id,
                DemoOptions {
                    clerics: 24,
                    max_clerics: 24,
                    minutes: 1,
                },
            );
            for step in steps {
                let Some(rest) = step
                    .message
                    .strip_prefix("!chain ")
                    .or_else(|| step.message.strip_prefix("!rchain "))
                else {
                    continue;
                };
                let seconds: f64 = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| panic!("{id}: cannot read pace from {}", step.message));
                assert!(
                    seconds >= MIN_INTERVAL_SECONDS,
                    "{id} calls {seconds}s, under the floor"
                );
            }
        }
    }

    #[test]
    fn clerics_skip_out_and_come_back_during_the_pull() {
        let steps = scenario_steps_with(
            "raid",
            DemoOptions {
                clerics: 12,
                max_clerics: 12,
                minutes: 5,
            },
        );
        let skips: Vec<&DemoStep> = steps
            .iter()
            .filter(|s| s.message.starts_with("!skip"))
            .collect();
        let backs: Vec<&DemoStep> = steps
            .iter()
            .filter(|s| s.message.starts_with("!back"))
            .collect();
        assert!(skips.len() >= 3, "{} skips in five minutes", skips.len());
        assert_eq!(skips.len(), backs.len(), "everyone who sits out comes back");
        // Different clerics take the breaks, and never you.
        let who: BTreeMap<&str, ()> = skips.iter().map(|s| (s.speaker.as_str(), ())).collect();
        assert!(who.len() > 1, "the same cleric skipped every time");
        assert!(!who.contains_key("YOU"), "you were skipped out");
    }

    #[test]
    fn a_small_raid_roster_still_runs() {
        for clerics in 1..=6 {
            let steps = scenario_steps_with(
                "raid",
                DemoOptions {
                    clerics,
                    max_clerics: clerics,
                    minutes: 1,
                },
            );
            assert!(
                steps.iter().any(|s| s.message.contains(" CH ")),
                "{clerics} clerics never cast"
            );
            let roster = raid_roster(clerics, you_seat(clerics));
            assert_eq!(roster.len(), clerics);
            assert_eq!(roster.iter().filter(|name| *name == "YOU").count(), 1);
        }
    }

    #[test]
    fn you_are_never_one_of_the_latecomers() {
        for clerics in 1..=6 {
            let roster = raid_roster(20, you_seat(clerics));
            let seat = roster.iter().position(|name| name == "YOU").expect("you");
            assert!(seat < clerics, "{clerics} starting: you are seat {seat}");
        }
    }

    #[test]
    fn every_scenario_paces_the_chain_before_it_starts() {
        for id in SCENARIO_IDS {
            let steps = scenario_steps(id);
            let paced = steps
                .iter()
                .position(|step| {
                    step.message.starts_with("!chain") || step.message.starts_with("!rchain")
                })
                .unwrap_or_else(|| panic!("{id} never sets an interval"));
            let started = steps
                .iter()
                .position(|step| step.message.starts_with("!startchain"))
                .unwrap_or_else(|| panic!("{id} never starts the chain"));
            assert!(paced < started, "{id} starts before it sets an interval");
        }
    }

    #[test]
    fn every_scenario_names_a_tank_and_cleans_up_after_itself() {
        for id in SCENARIO_IDS {
            let steps = scenario_steps(id);
            assert_eq!(
                steps.first().map(|step| step.message.as_str()),
                Some("!reset-chain"),
                "{id} does not start from a clean chain"
            );
            assert!(
                steps
                    .iter()
                    .any(|step| step.message.starts_with("!mt") || step.message.starts_with("!rt")),
                "{id} never names a tank"
            );
            assert!(
                steps
                    .iter()
                    .any(|step| step.message.starts_with("!stopchain")),
                "{id} leaves the chain running"
            );
        }
    }

    #[test]
    fn the_scenarios_between_them_cover_the_things_that_go_wrong() {
        let all: Vec<DemoStep> = SHORT_IDS.iter().flat_map(|id| scenario_steps(id)).collect();
        let has = |needle: &str| all.iter().any(|step| step.message.starts_with(needle));
        assert!(has("!ot"), "no off tank anywhere");
        assert!(has("!rt"), "no rampage tank anywhere");
        assert!(has("!split"), "nothing splits the chain");
        assert!(has("!untank"), "a split is never folded back");
        assert!(has("!move"), "numbers are never moved");
        assert!(has("!skip") && has("!back"), "nobody sits out and returns");
        assert!(
            all.iter().any(|step| step.message.contains(" RCH ")),
            "nobody casts a rampage heal"
        );
        assert!(
            all.iter().any(|step| step.note.contains("wrong tank")
                || step.note.contains("off tank from")
                || step.note.contains("main tank from")),
            "nobody heals the wrong tank"
        );
        assert!(
            all.iter().any(|step| step.note.contains("late")),
            "nobody is ever late"
        );
        assert!(
            all.iter().any(|step| step.note.contains("cannot read")),
            "no broken macros"
        );
    }

    #[test]
    fn every_scenario_has_steps_and_a_runtime() {
        for scenario in scenarios() {
            assert!(!scenario.name.is_empty(), "{} has no name", scenario.id);
            assert!(
                !scenario.description.is_empty(),
                "{} has no description",
                scenario.id
            );
            assert!(scenario.steps > 0, "{} has no steps", scenario.id);
            assert!(scenario.seconds > 1.0, "{} is too short", scenario.id);
        }
        assert!(scenario_steps("nope").is_empty());
    }

    /// The short pulls are meant to be watched start to finish, so none of them
    /// should turn into a long sit.
    #[test]
    fn the_short_scenarios_stay_short() {
        for id in SHORT_IDS {
            let scenario = describe(id, DemoOptions::default());
            assert!(
                scenario.seconds > 15.0,
                "{id} is only {:.0}s",
                scenario.seconds
            );
            assert!(
                scenario.seconds < 120.0,
                "{id} runs {:.0}s",
                scenario.seconds
            );
            assert!(!scenario.configurable, "{id} should not take options");
        }
    }

    #[test]
    fn everything_is_the_other_scenarios_end_to_end() {
        let parts: usize = SHORT_IDS.iter().map(|id| scenario_steps(id).len()).sum();
        assert_eq!(scenario_steps("full").len(), parts);
        assert!(!SHORT_IDS.contains(&"raid"), "the raid runs on its own");
        assert!(!SHORT_IDS.contains(&"full"));
        for id in SHORT_IDS {
            assert!(SCENARIO_IDS.contains(id), "{id} is not on the demo page");
        }
    }

    #[test]
    fn every_step_parses_as_a_log_line() {
        let parser = Parser::new();
        for id in SCENARIO_IDS {
            for step in scenario_steps(id) {
                assert!(!step.note.is_empty(), "{id} has a step without a note");
                let line = format_test_log_line(&step.speaker, step.channel, &step.message)
                    .unwrap_or_else(|err| panic!("{id}: {err}"));
                let parsed = parser.parse_line(&line, Some("Clericone"));
                assert!(
                    !matches!(parsed, ParseResult::Ignored),
                    "{id}: Alfred ignores {line}"
                );
                // Unreadable macros are in the scripts on purpose, and the note
                // next to them has to say so.
                if matches!(parsed, ParseResult::MalformedHeal { .. }) {
                    assert!(
                        step.note.contains("cannot read"),
                        "{id}: {line} is unreadable but the note does not say so"
                    );
                }
            }
        }
    }
}
