use regex::Regex;
use serde::{Deserialize, Serialize};

// Optional guild tag, then slot, CH/RCH, target. The configured tag (default GG) is required.
const CH_PATTERNS: &[&str] = &[
    r"(?i)^\s*(?:[A-Za-z]{2,4}\s+)?(\d{1,3})\s+CH\b\s*(?:--+|—|-|:)?\s*(.*?)\s*$",
    r"(?i)^\s*(?:[A-Za-z]{2,4}\s+)?CH\s+(\d{1,3})\s*(?:--+|—|-|:)?\s*(.*?)\s*$",
];

const RCH_PATTERNS: &[&str] = &[
    r"(?i)^\s*(?:[A-Za-z]{2,4}\s+)?([A-Za-z]{1,3})\s+RCH\b\s*(?:--+|—|-|:)?\s*(.*?)\s*$",
    r"(?i)^\s*(?:[A-Za-z]{2,4}\s+)?RCH\s+([A-Za-z]{1,3})\s*(?:--+|—|-|:)?\s*(.*?)\s*$",
    r"(?i)^\s*(?:[A-Za-z]{2,4}\s+)?([A-Za-z]{1,3})\s+CH\b\s*(?:--+|—|-|:)?\s*(.*?)\s*$",
    r"(?i)^\s*([A-Za-z]{1,3})\s*-+\s*CH\b\s*-+\s*(.*?)\s*$",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Shout,
    Ooc,
    Group,
    Guild,
    Auction,
    Raid,
    Say,
    Tell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatLine {
    pub speaker: String,
    pub is_you: bool,
    pub channel: Channel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChainCommand {
    MainTank { tank: String },
    OffTank { tank: String },
    Split { number: u32 },
    TankRange { tank: String, from: u32, to: u32 },
    Untank { tank: String },
    Skip { number: Option<u32> },
    Back { number: Option<u32> },
    ResetChain,
    Take { number: u32, player: Option<String> },
    TakeNext { player: Option<String> },
    Move { from: u32, to: u32 },
    ChainInterval { seconds: f64, tank: Option<String> },
    StartChain { tank: Option<String> },
    EndChain { tank: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteHealCall {
    pub speaker: String,
    pub is_you: bool,
    pub number: u32,
    pub target: String,
    pub tag: Option<String>,
    pub raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainKind {
    Cleric,
    Rampage,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseResult {
    CompleteHeal(CompleteHealCall),
    RampageHeal(CompleteHealCall),
    Command(ChainCommand, String, bool, ChainKind),
    MalformedHeal {
        speaker: String,
        message: String,
        rampage: bool,
    },
    Ignored,
}

pub struct Parser {
    ch_patterns: Vec<Regex>,
    rch_patterns: Vec<Regex>,
    chain_tag: String,
}

impl Parser {
    pub fn new() -> Self {
        Self::with_chain_tag("GG")
    }

    pub fn with_chain_tag(tag: impl AsRef<str>) -> Self {
        Self {
            ch_patterns: CH_PATTERNS
                .iter()
                .map(|pat| Regex::new(pat).expect("built-in CH pattern"))
                .collect(),
            rch_patterns: RCH_PATTERNS
                .iter()
                .map(|pat| Regex::new(pat).expect("built-in RCH pattern"))
                .collect(),
            chain_tag: crate::config::normalize_chain_tag(tag.as_ref()),
        }
    }

    pub fn set_chain_tag(&mut self, tag: &str) {
        self.chain_tag = crate::config::normalize_chain_tag(tag);
    }

    fn accepts_tag(&self, message: &str) -> bool {
        detect_tag(message).is_some_and(|tag| tag == self.chain_tag)
    }

    pub fn parse_line(&self, line: &str, your_name: Option<&str>) -> ParseResult {
        let Some(chat) = parse_chat_line(line, your_name) else {
            return ParseResult::Ignored;
        };
        if let Some((cmd, kind)) = parse_chain_command(&chat.message) {
            return ParseResult::Command(cmd, chat.speaker, chat.is_you, kind);
        }
        if let Some(call) = self.parse_rch_message(&chat.message, &chat.speaker, chat.is_you) {
            return ParseResult::RampageHeal(call);
        }
        if let Some(call) = self.parse_ch_message(&chat.message, &chat.speaker, chat.is_you) {
            return ParseResult::CompleteHeal(call);
        }
        if looks_like_rch_macro(&chat.message, &self.chain_tag) {
            ParseResult::MalformedHeal {
                speaker: chat.speaker,
                message: chat.message,
                rampage: true,
            }
        } else if looks_like_ch_macro(&chat.message, &self.chain_tag) {
            ParseResult::MalformedHeal {
                speaker: chat.speaker,
                message: chat.message,
                rampage: false,
            }
        } else {
            ParseResult::Ignored
        }
    }

    fn parse_ch_message(
        &self,
        message: &str,
        speaker: &str,
        is_you: bool,
    ) -> Option<CompleteHealCall> {
        for re in &self.ch_patterns {
            if let Some(caps) = re.captures(message.trim()) {
                if !self.accepts_tag(message) {
                    return None;
                }
                let number = caps.get(1)?.as_str().parse().ok()?;
                if !(1..=999).contains(&number) {
                    continue;
                }
                let target = caps
                    .get(2)
                    .map(|m| sanitize_target(m.as_str()))
                    .unwrap_or_default();
                let tag = detect_tag(message);
                return Some(CompleteHealCall {
                    speaker: speaker.to_string(),
                    is_you,
                    number,
                    target,
                    tag,
                    raw: message.to_string(),
                });
            }
        }
        None
    }

    fn parse_rch_message(
        &self,
        message: &str,
        speaker: &str,
        is_you: bool,
    ) -> Option<CompleteHealCall> {
        for re in &self.rch_patterns {
            if let Some(caps) = re.captures(message.trim()) {
                if !self.accepts_tag(message) {
                    return None;
                }
                let Some(number) = parse_letter_slot(caps.get(1)?.as_str()) else {
                    continue;
                };
                let target = caps
                    .get(2)
                    .map(|m| sanitize_target(m.as_str()))
                    .unwrap_or_default();
                let tag = detect_tag(message);
                return Some(CompleteHealCall {
                    speaker: speaker.to_string(),
                    is_you,
                    number,
                    target,
                    tag,
                    raw: message.to_string(),
                });
            }
        }
        None
    }
}

fn parse_chat_line(line: &str, your_name: Option<&str>) -> Option<ChatLine> {
    let rest = strip_timestamp(line)?;
    const YOU: &[(&str, Channel)] = &[
        ("You shout,", Channel::Shout),
        ("You say out of character,", Channel::Ooc),
        ("You tell the group,", Channel::Group),
        ("You tell your party,", Channel::Group),
        ("You tell your group,", Channel::Group),
        ("You tell the guild,", Channel::Guild),
        ("You tell your guild,", Channel::Guild),
        ("You say to your guild,", Channel::Guild),
        ("You say to the guild,", Channel::Guild),
        ("You auction,", Channel::Auction),
        ("You tell the raid,", Channel::Raid),
        ("You tell your raid,", Channel::Raid),
        ("You say to your raid,", Channel::Raid),
        ("You say to the raid,", Channel::Raid),
    ];
    for (prefix, channel) in YOU {
        if let Some(message) = extract_you_speech(rest, &[prefix]) {
            return Some(ChatLine {
                speaker: your_name.unwrap_or("You").to_string(),
                is_you: true,
                channel: *channel,
                message,
            });
        }
    }
    const NAMED: &[(&str, Channel)] = &[
        (" shouts, ", Channel::Shout),
        (" says out of character, ", Channel::Ooc),
        (" tells the group, ", Channel::Group),
        (" tells the party, ", Channel::Group),
        (" tells the guild, ", Channel::Guild),
        (" auctions, ", Channel::Auction),
        (" tells the raid, ", Channel::Raid),
        (" tells your raid, ", Channel::Raid),
    ];
    for (marker, channel) in NAMED {
        if let Some((speaker, message)) = extract_named_speech(rest, marker) {
            let is_you = your_name.is_some_and(|n| n.eq_ignore_ascii_case(&speaker));
            return Some(ChatLine {
                speaker,
                is_you,
                channel: *channel,
                message,
            });
        }
    }
    parse_generic_quoted_speech(rest, your_name)
}

fn parse_generic_quoted_speech(rest: &str, your_name: Option<&str>) -> Option<ChatLine> {
    let comma = rest.find(", '").or_else(|| rest.find(", \""))?;
    let head = rest[..comma].trim();
    let message = unquote_speech(rest[comma + 2..].trim())?;
    if let Some(verbs) = head.strip_prefix("You ") {
        return Some(ChatLine {
            speaker: your_name.unwrap_or("You").to_string(),
            is_you: true,
            channel: channel_from_verbs(verbs),
            message,
        });
    }
    let speaker = head.split_whitespace().next()?;
    if speaker.is_empty() || !speaker.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    let lower = speaker.to_ascii_lowercase();
    if matches!(lower.as_str(), "a" | "an" | "the") {
        return None;
    }
    let verbs = head[speaker.len()..].trim();
    let is_you = your_name.is_some_and(|n| n.eq_ignore_ascii_case(speaker));
    Some(ChatLine {
        speaker: speaker.to_string(),
        is_you,
        channel: channel_from_verbs(verbs),
        message,
    })
}

fn channel_from_verbs(verbs: &str) -> Channel {
    let lower = verbs.to_ascii_lowercase();
    if lower.contains("out of character") {
        Channel::Ooc
    } else if lower.contains("shout") {
        Channel::Shout
    } else if lower.contains("auction") {
        Channel::Auction
    } else if lower.contains("guild") {
        Channel::Guild
    } else if lower.contains("group") || lower.contains("party") {
        Channel::Group
    } else if lower.contains("raid") {
        Channel::Raid
    } else if lower.contains("tell") {
        Channel::Tell
    } else {
        Channel::Say
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestChannel {
    Shout,
    Ooc,
    Group,
    Guild,
    Auction,
    Raid,
    Say,
}

pub fn format_test_log_line(speaker: &str, channel: TestChannel, message: &str) -> Result<String, String> {
    let ts = "[Fri Sep 18 16:27:00 2026]";
    let message = message.trim();
    if message.is_empty() {
        return Err("Type a chat message or command.".into());
    }
    let speaker = speaker.trim();
    let as_you = speaker.is_empty() || speaker.eq_ignore_ascii_case("you");
    let name = speaker.split_whitespace().next().unwrap_or("");
    if !as_you && name.is_empty() {
        return Err("Use YOU or a single character name.".into());
    }
    let quoted = if (message.starts_with('\'') && message.ends_with('\''))
        || (message.starts_with('"') && message.ends_with('"'))
    {
        message.to_string()
    } else {
        format!("'{message}'")
    };
    let body = match (as_you, channel) {
        (true, TestChannel::Shout) => format!("You shout, {quoted}"),
        (true, TestChannel::Ooc) => format!("You say out of character, {quoted}"),
        (true, TestChannel::Group) => format!("You tell the group, {quoted}"),
        (true, TestChannel::Guild) => format!("You tell the guild, {quoted}"),
        (true, TestChannel::Auction) => format!("You auction, {quoted}"),
        (true, TestChannel::Raid) => format!("You tell the raid, {quoted}"),
        (true, TestChannel::Say) => format!("You say, {quoted}"),
        (false, TestChannel::Shout) => format!("{name} shouts, {quoted}"),
        (false, TestChannel::Ooc) => format!("{name} says out of character, {quoted}"),
        (false, TestChannel::Group) => format!("{name} tells the group, {quoted}"),
        (false, TestChannel::Guild) => format!("{name} tells the guild, {quoted}"),
        (false, TestChannel::Auction) => format!("{name} auctions, {quoted}"),
        (false, TestChannel::Raid) => format!("{name} tells the raid, {quoted}"),
        (false, TestChannel::Say) => format!("{name} says, {quoted}"),
    };
    Ok(format!("{ts} {body}"))
}

fn strip_timestamp(line: &str) -> Option<&str> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix('[') {
        let end = rest.find(']')?;
        Some(rest[end + 1..].trim())
    } else {
        Some(line)
    }
}

fn extract_you_speech(rest: &str, prefixes: &[&str]) -> Option<String> {
    for prefix in prefixes {
        if let Some(after) = rest.strip_prefix(prefix) {
            return Some(unquote_speech(after.trim())?);
        }
    }
    None
}

fn extract_named_speech(rest: &str, marker: &str) -> Option<(String, String)> {
    let idx = rest.find(marker)?;
    let speaker = rest[..idx].trim();
    if speaker.is_empty() || speaker.contains(' ') {
        return None;
    }
    let message = unquote_speech(rest[idx + marker.len()..].trim())?;
    Some((speaker.to_string(), message))
}

fn unquote_speech(text: &str) -> Option<String> {
    let text = text.trim().trim_end_matches('.').trim();
    let bytes = text.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let start = bytes[0];
    let end = bytes[bytes.len() - 1];
    let quoted = matches!(
        (start, end),
        (b'\'', b'\'') | (b'"', b'"') | (b'`', b'`')
    );
    if quoted {
        Some(text[1..text.len() - 1].trim().to_string())
    } else {
        Some(text.to_string())
    }
}

pub fn parse_chain_command(message: &str) -> Option<(ChainCommand, ChainKind)> {
    let message = message.trim().trim_matches('"').trim_matches('\'').trim();
    let mut parts = message.split_whitespace();
    let cmd = parts.next()?.to_ascii_lowercase();
    match cmd.as_str() {
        "!mt" => Some((
            ChainCommand::MainTank {
                tank: parts.next()?.to_string(),
            },
            ChainKind::Cleric,
        )),
        "!rt" => Some((
            ChainCommand::MainTank {
                tank: parts.next()?.to_string(),
            },
            ChainKind::Rampage,
        )),
        "!ot" => Some((
            ChainCommand::OffTank {
                tank: parts.next()?.to_string(),
            },
            ChainKind::Cleric,
        )),
        "!split" => Some((
            ChainCommand::Split {
                number: parse_slot(parts.next()?)?,
            },
            ChainKind::Cleric,
        )),
        "!tank" => tank_range(&mut parts, ChainKind::Cleric, parse_slot),
        "!untank" => Some((
            ChainCommand::Untank {
                tank: parts.next()?.to_string(),
            },
            ChainKind::Cleric,
        )),
        "!skip" => match parts.next() {
            None => Some((ChainCommand::Skip { number: None }, ChainKind::Both)),
            Some(raw) => {
                let (number, kind) = parse_typed_slot(raw)?;
                Some((ChainCommand::Skip { number: Some(number) }, kind))
            }
        },
        // Old rampage-only names; letters on !skip / !back / !take / !move are enough.
        "!rskip" => Some((
            ChainCommand::Skip {
                number: parse_optional_rampage_slot(parts.next())?,
            },
            ChainKind::Rampage,
        )),
        "!back" => match parts.next() {
            None => Some((ChainCommand::Back { number: None }, ChainKind::Both)),
            Some(raw) => {
                let (number, kind) = parse_typed_slot(raw)?;
                Some((ChainCommand::Back { number: Some(number) }, kind))
            }
        },
        "!rback" => Some((
            ChainCommand::Back {
                number: parse_optional_rampage_slot(parts.next())?,
            },
            ChainKind::Rampage,
        )),
        "!reset-chain" | "!resetchain" => Some((ChainCommand::ResetChain, ChainKind::Cleric)),
        "!take" => match parts.next() {
            None => Some((ChainCommand::TakeNext { player: None }, ChainKind::Cleric)),
            Some(raw) => {
                if let Some((number, kind)) = parse_typed_slot(raw) {
                    Some((
                        ChainCommand::Take {
                            number,
                            player: optional_player(parts.next()),
                        },
                        kind,
                    ))
                } else if raw.chars().all(|c| c.is_ascii_digit()) {
                    None
                } else {
                    Some((
                        ChainCommand::TakeNext {
                            player: optional_player(Some(raw)),
                        },
                        ChainKind::Cleric,
                    ))
                }
            }
        },
        "!rtake" => match parts.next() {
            None => Some((ChainCommand::TakeNext { player: None }, ChainKind::Rampage)),
            Some(raw) => Some((
                ChainCommand::Take {
                    number: parse_rampage_slot(raw)?,
                    player: optional_player(parts.next()),
                },
                ChainKind::Rampage,
            )),
        },
        "!move" => typed_move(&mut parts),
        "!rmove" => Some((
            ChainCommand::Move {
                from: parse_rampage_slot(parts.next()?)?,
                to: parse_rampage_slot(parts.next()?)?,
            },
            ChainKind::Rampage,
        )),
        "!chain" => interval_cmd(&mut parts, ChainKind::Cleric),
        "!rchain" => interval_cmd(&mut parts, ChainKind::Rampage),
        "!startchain" | "!start-chain" => Some((
            ChainCommand::StartChain {
                tank: parts.next().map(|s| s.to_string()),
            },
            ChainKind::Both,
        )),
        "!rstartchain" | "!rstart-chain" => Some((
            ChainCommand::StartChain {
                tank: parts.next().map(|s| s.to_string()),
            },
            ChainKind::Rampage,
        )),
        "!stopchain" | "!stop-chain" | "!endchain" | "!end-chain" => Some((
            ChainCommand::EndChain {
                tank: parts.next().map(|s| s.to_string()),
            },
            ChainKind::Both,
        )),
        "!rstopchain" | "!rstop-chain" | "!rendchain" | "!rend-chain" => Some((
            ChainCommand::EndChain {
                tank: parts.next().map(|s| s.to_string()),
            },
            ChainKind::Rampage,
        )),
        "!start" => Some((
            ChainCommand::StartChain {
                tank: optional_named_tank(&mut parts),
            },
            ChainKind::Both,
        )),
        "!rstart" => Some((
            ChainCommand::StartChain {
                tank: optional_named_tank(&mut parts),
            },
            ChainKind::Rampage,
        )),
        "!stop" | "!end" => Some((
            ChainCommand::EndChain {
                tank: optional_named_tank(&mut parts),
            },
            ChainKind::Both,
        )),
        "!rstop" | "!rend" => Some((
            ChainCommand::EndChain {
                tank: optional_named_tank(&mut parts),
            },
            ChainKind::Rampage,
        )),
        _ => None,
    }
}

fn optional_player(raw: Option<&str>) -> Option<String> {
    let name = raw?.trim();
    if name.is_empty() || name.starts_with('!') {
        return None;
    }
    Some(name.to_string())
}

fn tank_range(
    parts: &mut std::str::SplitWhitespace<'_>,
    kind: ChainKind,
    parse: fn(&str) -> Option<u32>,
) -> Option<(ChainCommand, ChainKind)> {
    Some((
        ChainCommand::TankRange {
            tank: parts.next()?.to_string(),
            from: parse(parts.next()?)?,
            to: parse(parts.next()?)?,
        },
        kind,
    ))
}

fn interval_cmd(
    parts: &mut std::str::SplitWhitespace<'_>,
    kind: ChainKind,
) -> Option<(ChainCommand, ChainKind)> {
    let seconds: f64 = parts.next()?.parse().ok()?;
    if seconds <= 0.0 {
        return None;
    }
    Some((
        ChainCommand::ChainInterval {
            seconds,
            tank: parts.next().map(|s| s.to_string()),
        },
        kind,
    ))
}

fn typed_move(
    parts: &mut std::str::SplitWhitespace<'_>,
) -> Option<(ChainCommand, ChainKind)> {
    let (from, kind_from) = parse_typed_slot(parts.next()?)?;
    let (to, kind_to) = parse_typed_slot(parts.next()?)?;
    if kind_from != kind_to {
        return None;
    }
    Some((ChainCommand::Move { from, to }, kind_from))
}

fn optional_named_tank<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<String> {
    let next = parts.next()?;
    if next.eq_ignore_ascii_case("chain") {
        parts.next().map(|s| s.to_string())
    } else {
        Some(next.to_string())
    }
}

fn parse_optional_rampage_slot(raw: Option<&str>) -> Option<Option<u32>> {
    match raw {
        None => Some(None),
        Some(raw) => Some(Some(parse_rampage_slot(raw)?)),
    }
}

fn parse_slot(raw: &str) -> Option<u32> {
    let n: u32 = raw.parse().ok()?;
    (1..=999).contains(&n).then_some(n)
}

pub fn parse_letter_slot(raw: &str) -> Option<u32> {
    let raw = raw.trim();
    if raw.is_empty() || raw.len() > 3 {
        return None;
    }
    let upper = raw.to_ascii_uppercase();
    let first = upper.chars().next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    if !upper.chars().all(|c| c == first) {
        return None;
    }
    if upper == "GG" || upper == "CC" {
        return None;
    }
    Some(first as u32 - u32::from(b'A') + 1)
}

fn parse_rampage_slot(raw: &str) -> Option<u32> {
    parse_letter_slot(raw).or_else(|| parse_slot(raw).filter(|n| (1..=26).contains(n)))
}

fn parse_typed_slot(raw: &str) -> Option<(u32, ChainKind)> {
    if let Some(number) = parse_slot(raw) {
        Some((number, ChainKind::Cleric))
    } else {
        Some((parse_letter_slot(raw)?, ChainKind::Rampage))
    }
}

fn sanitize_target(raw: &str) -> String {
    let cleaned = raw
        .trim()
        .trim_matches(|c: char| matches!(c, '-' | '—' | ':' | '<' | '>' | '"' | '\''));
    cleaned
        .split_whitespace()
        .find(|w| w.chars().any(|c| c.is_ascii_alphabetic()))
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_string()
}

fn detect_tag(message: &str) -> Option<String> {
    let first = message.split_whitespace().next()?;
    if (2..=4).contains(&first.len()) && first.chars().all(|c| c.is_ascii_alphabetic()) {
        let upper = first.to_ascii_uppercase();
        if upper != "CH" && upper != "RCH" {
            return Some(upper);
        }
    }
    None
}

fn looks_like_rch_macro(message: &str, chain_tag: &str) -> bool {
    if !detect_tag(message).is_some_and(|tag| tag == chain_tag) {
        return false;
    }
    let upper = message.to_ascii_uppercase();
    upper
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|w| w == "RCH")
}

fn looks_like_ch_macro(message: &str, chain_tag: &str) -> bool {
    if !detect_tag(message).is_some_and(|tag| tag == chain_tag) {
        return false;
    }
    let upper = message.to_ascii_uppercase();
    upper
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|w| w == "CH")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> Parser {
        Parser::new()
    }

    fn line(s: &str) -> ParseResult {
        parser().parse_line(s, Some("Clericone"))
    }

    fn line_as(s: &str, you: Option<&str>) -> ParseResult {
        parser().parse_line(s, you)
    }

    fn shout(message: &str) -> ParseResult {
        line(&format!(
            "[Fri Sep 18 16:27:00 2026] Hanbox shouts, '{message}'"
        ))
    }

    fn heal(s: &str) -> CompleteHealCall {
        match line(s) {
            ParseResult::CompleteHeal(call) => call,
            other => panic!("expected CH, got {other:?} in {s}"),
        }
    }

    #[test]
    fn parse_gg_shout() {
        let call = heal(r#"[Fri Sep 18 16:27:00 2026] Curaja shouts, 'GG 014 CH -- Wreckognize'"#);
        assert_eq!(call.speaker, "Curaja");
        assert!(!call.is_you);
        assert_eq!(call.number, 14);
        assert_eq!(call.target, "Wreckognize");
        assert_eq!(call.tag.as_deref(), Some("GG"));
    }

    #[test]
    fn parse_you_shout_uses_character_name() {
        let call = heal(r#"[Fri Sep 18 16:27:00 2026] You shout, 'GG 001 CH -- Mluian'"#);
        assert_eq!(call.speaker, "Clericone");
        assert!(call.is_you);
        assert_eq!(call.number, 1);
        assert_eq!(call.target, "Mluian");
    }

    #[test]
    fn parse_you_shout_without_character_name() {
        let ParseResult::CompleteHeal(call) = line_as(
            r#"[Fri Sep 18 16:27:00 2026] You shout, 'GG 001 CH -- Mluian'"#,
            None,
        ) else {
            panic!("expected CH");
        };
        assert_eq!(call.speaker, "You");
        assert!(call.is_you);
    }

    #[test]
    fn named_shout_is_you_when_names_match_ignoring_case() {
        let call = heal(r#"[Fri Sep 18 16:27:00 2026] clericone shouts, 'GG 003 CH -- Mluian'"#);
        assert_eq!(call.speaker, "clericone");
        assert!(call.is_you);
        assert_eq!(call.number, 3);
    }

    #[test]
    fn parse_ch_in_every_allowed_channel() {
        let samples = [
            "Hanbox shouts, 'GG 001 CH -- Beefwich'",
            "Hanbox says out of character, 'GG 001 CH -- Beefwich'",
            "Hanbox tells the group, 'GG 001 CH -- Beefwich'",
            "Hanbox tells the guild, 'GG 001 CH -- Beefwich'",
            "Hanbox auctions, 'GG 001 CH -- Beefwich'",
            "Hanbox tells the raid, 'GG 001 CH -- Beefwich'",
            "Hanbox says, 'GG 001 CH -- Beefwich'",
            "Hanbox tells you, 'GG 001 CH -- Beefwich'",
            "You say out of character, 'GG 001 CH -- Beefwich'",
            "You tell the group, 'GG 001 CH -- Beefwich'",
            "You auction, 'GG 001 CH -- Beefwich'",
            "You tell the raid, 'GG 001 CH -- Beefwich'",
            "You say, 'GG 001 CH -- Beefwich'",
        ];
        for sample in samples {
            let line = format!("[Fri Sep 18 16:27:00 2026] {sample}");
            let ParseResult::CompleteHeal(call) = parser().parse_line(&line, Some("Clericone"))
            else {
                panic!("failed on {sample}");
            };
            assert_eq!(call.number, 1, "{sample}");
            assert_eq!(call.target, "Beefwich", "{sample}");
        }
    }

    #[test]
    fn say_and_tell_channels_are_accepted() {
        let ParseResult::CompleteHeal(call) = line(
            "[Fri Sep 18 16:27:00 2026] Hanbox says, 'GG 001 CH -- Beefwich'",
        ) else {
            panic!("named say");
        };
        assert_eq!(call.number, 1);
        assert_eq!(call.speaker, "Hanbox");
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You say, '!startchain'"),
            ParseResult::Command(ChainCommand::StartChain { tank: None }, _, true, ChainKind::Both)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] Two tells you, '!take 002'"),
            ParseResult::Command(
                ChainCommand::Take {
                    number: 2,
                    player: None
                },
                _,
                false,
                ChainKind::Cleric
            )
        ));
    }

    #[test]
    fn parse_ch_message_variations() {
        let samples = [
            "GG CH 001 -- Beefwich",
            "GG 001 CH --Beefwich",
            "gg 1 ch - Beefwich",
            "GG 001 CH : Beefwich",
            "GG 001 CH Beefwich",
            "GG 001 CH --Beefwich 001",
            "  GG   001   CH  --  Beefwich  ",
        ];
        for message in samples {
            let ParseResult::CompleteHeal(call) = shout(message) else {
                panic!("failed on {message}");
            };
            assert_eq!(call.number, 1, "{message}");
            assert_eq!(call.target, "Beefwich", "{message}");
        }
    }

    #[test]
    fn parse_real_p99_log_ch_and_rch_lines() {
        let call = heal(
            "[Fri Sep 18 19:15:24 2026] You say out of character, 'GG 001 CH -- Portlia'",
        );
        assert!(call.is_you);
        assert_eq!(call.number, 1);
        assert_eq!(call.target, "Portlia");
        assert_eq!(call.tag.as_deref(), Some("GG"));

        let call = heal("[Fri Sep 18 15:02:34 2026] You shout, 'GG 008 CH -- Moonglade'");
        assert_eq!(call.number, 8);
        assert_eq!(call.target, "Moonglade");

        let call = heal("[Fri Sep 18 14:54:00 2026] Nickopol shouts, 'GG  006 CH  -- Braillard'");
        assert_eq!(call.number, 6);
        assert_eq!(call.target, "Braillard");

        assert!(matches!(
            shout("CA 015 CH -- Tennesseee"),
            ParseResult::Ignored
        ));
        assert!(matches!(shout("ST 002 CH -- Gratton"), ParseResult::Ignored));
        assert!(matches!(shout("SS 001 CH -- Tank"), ParseResult::Ignored));
        assert!(matches!(shout("001 CH -- Beefwich"), ParseResult::Ignored));

        let ParseResult::RampageHeal(call) = line(
            "[Fri Sep 18 16:27:00 2026] Curaja shouts, 'GG RCH AAA -- Beefwich'",
        ) else {
            panic!("gg rch");
        };
        assert_eq!(call.number, 1);
        assert_eq!(call.target, "Beefwich");

        assert!(matches!(shout("CA RCH AAA -- Mcganahan"), ParseResult::Ignored));
        assert!(matches!(shout("LT AAA RCH -- Stilgard"), ParseResult::Ignored));
        assert!(matches!(shout("ST HHH CH -- Kaido"), ParseResult::Ignored));
        assert!(matches!(shout("QQQ - CH - Shinko"), ParseResult::Ignored));

        let other = Parser::with_chain_tag("CA");
        let ParseResult::CompleteHeal(call) = other.parse_line(
            "[Fri Sep 18 16:27:00 2026] Hanbox shouts, 'CA 015 CH -- Tennesseee'",
            Some("Clericone"),
        ) else {
            panic!("configured CA tag");
        };
        assert_eq!(call.number, 15);
        assert_eq!(call.tag.as_deref(), Some("CA"));
        assert!(matches!(
            other.parse_line(
                "[Fri Sep 18 16:27:00 2026] Hanbox shouts, 'GG 001 CH -- Portlia'",
                Some("Clericone"),
            ),
            ParseResult::Ignored
        ));
    }

    #[test]
    fn parse_double_quoted_shout() {
        let call = heal(r#"[Fri Sep 18 16:27:00 2026] Hanbox shouts, "GG 002 CH -- Beefwich""#);
        assert_eq!(call.number, 2);
        assert_eq!(call.tag.as_deref(), Some("GG"));
    }

    #[test]
    fn parse_shout_without_timestamp() {
        let call = heal(r#"Hanbox shouts, 'GG 007 CH -- Mluian'"#);
        assert_eq!(call.number, 7);
    }

    #[test]
    fn numbers_outside_1_to_999_are_not_heals() {
        assert!(matches!(shout("GG 000 CH -- Mluian"), ParseResult::MalformedHeal { .. }));
        assert!(matches!(shout("GG 1000 CH -- Mluian"), ParseResult::MalformedHeal { .. }));
    }

    #[test]
    fn malformed_macro_warns() {
        for message in [
            "GG CH -- Wreckognize",
            "GG CH Wreckognize",
        ] {
            let ParseResult::MalformedHeal { speaker, message: raw, .. } = shout(message) else {
                panic!("expected warning for {message}");
            };
            assert_eq!(speaker, "Hanbox");
            assert_eq!(raw, message);
        }
    }

    #[test]
    fn ordinary_chat_is_ignored() {
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You hit a goblin for 12 points of damage."),
            ParseResult::Ignored
        ));
        assert!(matches!(shout("inc on me"), ParseResult::Ignored));
        assert!(matches!(
            shout("I like to CH in my spare time"),
            ParseResult::Ignored
        ));
        assert!(matches!(shout("GG 001 -- Wreckognize"), ParseResult::Ignored));
    }

    #[test]
    fn commands_from_group_ooc_and_auction() {
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] Leadcleric tells the group, '!startchain'"),
            ParseResult::Command(ChainCommand::StartChain { .. }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You say out of character, '!stopchain'"),
            ParseResult::Command(ChainCommand::EndChain { .. }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] Leadcleric auctions, '!skip 001'"),
            ParseResult::Command(ChainCommand::Skip { number: Some(1) }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You shout, '!skip'"),
            ParseResult::Command(ChainCommand::Skip { number: None }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] Two tells the group, '!back'"),
            ParseResult::Command(ChainCommand::Back { number: None }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You shout, '!start-chain'"),
            ParseResult::Command(ChainCommand::StartChain { .. }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] Leadcleric tells the raid, '!start'"),
            ParseResult::Command(ChainCommand::StartChain { .. }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You tell your raid, '!stop'"),
            ParseResult::Command(ChainCommand::EndChain { .. }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] Two tells the raid, '!start chain'"),
            ParseResult::Command(ChainCommand::StartChain { .. }, ..)
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You say to the raid, '!stop chain'"),
            ParseResult::Command(ChainCommand::EndChain { .. }, ..)
        ));
    }

    #[test]
    fn you_begin_casting_complete_heal_is_ignored() {
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You begin casting Complete Heal."),
            ParseResult::Ignored
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You begin to cast Complete Heal."),
            ParseResult::Ignored
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You begin casting Complete Healing."),
            ParseResult::Ignored
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] You begin casting Greater Healing."),
            ParseResult::Ignored
        ));
    }

    #[test]
    fn guild_commands() {
        let ParseResult::Command(ChainCommand::MainTank { tank }, speaker, is_you, _) = line(
            r#"[Fri Sep 18 16:27:00 2026] Leadcleric tells the guild, '!mt Mluian'"#,
        ) else {
            panic!("mt");
        };
        assert_eq!(tank, "Mluian");
        assert_eq!(speaker, "Leadcleric");
        assert!(!is_you);

        let ParseResult::Command(ChainCommand::Take { number, player, .. }, speaker, is_you, _) = line(
            r#"[Fri Sep 18 16:27:00 2026] You tell the guild, '!take 001'"#,
        ) else {
            panic!("take");
        };
        assert_eq!(number, 1);
        assert_eq!(player, None);
        assert_eq!(speaker, "Clericone");
        assert!(is_you);

        let ParseResult::Command(ChainCommand::Take { number, player, .. }, speaker, ..) = line(
            r#"[Fri Sep 18 16:27:00 2026] You tell the guild, '!take 005 Portlia'"#,
        ) else {
            panic!("take named");
        };
        assert_eq!(number, 5);
        assert_eq!(player.as_deref(), Some("Portlia"));
        assert_eq!(speaker, "Clericone");

        let ParseResult::Command(ChainCommand::Move { from, to }, ..) = line(
            r#"[Fri Sep 18 16:27:00 2026] You tell your guild, '!move 001 002'"#,
        ) else {
            panic!("move");
        };
        assert_eq!((from, to), (1, 2));

        let ParseResult::Command(ChainCommand::ChainInterval { seconds, tank }, ..) = line(
            r#"[Fri Sep 18 16:27:00 2026] Leadcleric tells the guild, '!chain 2.5'"#,
        ) else {
            panic!("chain");
        };
        assert_eq!(seconds, 2.5);
        assert_eq!(tank, None);

        assert!(matches!(
            line(r#"[Fri Sep 18 16:27:00 2026] Leadcleric tells the guild, '!reset-chain'"#),
            ParseResult::Command(ChainCommand::ResetChain, ..)
        ));
        assert!(matches!(
            line(r#"[Fri Sep 18 16:27:00 2026] You say to your guild, '!resetchain'"#),
            ParseResult::Command(ChainCommand::ResetChain, ..)
        ));
        assert!(matches!(
            line(r#"[Fri Sep 18 16:27:00 2026] Leadcleric tells the guild, '!skip 001'"#),
            ParseResult::Command(ChainCommand::Skip { number: Some(1) }, ..)
        ));
        assert!(matches!(
            line(r#"[Fri Sep 18 16:27:00 2026] Leadcleric tells the guild, '!back 001'"#),
            ParseResult::Command(ChainCommand::Back { number: Some(1) }, ..)
        ));
        assert!(matches!(
            line(r#"[Fri Sep 18 16:27:00 2026] You say to the guild, '!TAKE 8'"#),
            ParseResult::Command(ChainCommand::Take { number: 8, player: None }, ..)
        ));
    }

    #[test]
    fn chain_command_parser_rejects_bad_args() {
        assert_eq!(parse_chain_command("!mt"), None);
        assert_eq!(
            parse_chain_command("!skip"),
            Some((ChainCommand::Skip { number: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!back"),
            Some((ChainCommand::Back { number: None }, ChainKind::Both))
        );
        assert_eq!(parse_chain_command("!skip 0"), None);
        assert_eq!(parse_chain_command("!skip 1000"), None);
        assert_eq!(parse_chain_command("!take 0"), None);
        assert_eq!(parse_chain_command("!take 1000"), None);
        assert_eq!(
            parse_chain_command("!take"),
            Some((ChainCommand::TakeNext { player: None }, ChainKind::Cleric))
        );
        assert_eq!(
            parse_chain_command("!take Portlia"),
            Some((
                ChainCommand::TakeNext {
                    player: Some("Portlia".into())
                },
                ChainKind::Cleric
            ))
        );
        assert_eq!(
            parse_chain_command("!rtake"),
            Some((ChainCommand::TakeNext { player: None }, ChainKind::Rampage))
        );
        assert_eq!(parse_chain_command("!move 001"), None);
        assert_eq!(parse_chain_command("!chain"), None);
        assert_eq!(parse_chain_command("!chain 0"), None);
        assert_eq!(parse_chain_command("!chain -1"), None);
        assert_eq!(parse_chain_command("hello"), None);
        assert_eq!(
            parse_chain_command("  !mt   Mluian extra"),
            Some((
                ChainCommand::MainTank {
                    tank: "Mluian".into(),
                },
                ChainKind::Cleric,
            ))
        );
        assert_eq!(
            parse_chain_command("!startchain"),
            Some((ChainCommand::StartChain { tank: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!start"),
            Some((ChainCommand::StartChain { tank: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!start chain"),
            Some((ChainCommand::StartChain { tank: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!stop"),
            Some((ChainCommand::EndChain { tank: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!stop chain"),
            Some((ChainCommand::EndChain { tank: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!stop-chain"),
            Some((ChainCommand::EndChain { tank: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!stopchain"),
            Some((ChainCommand::EndChain { tank: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!endchain"),
            Some((ChainCommand::EndChain { tank: None }, ChainKind::Both))
        );
        assert_eq!(
            parse_chain_command("!startchain Mluian"),
            Some((
                ChainCommand::StartChain {
                    tank: Some("Mluian".into())
                },
                ChainKind::Both
            ))
        );
        assert_eq!(
            parse_chain_command("!ot Beefwich"),
            Some((
                ChainCommand::OffTank {
                    tank: "Beefwich".into()
                },
                ChainKind::Cleric
            ))
        );
        assert_eq!(
            parse_chain_command("!split 9"),
            Some((ChainCommand::Split { number: 9 }, ChainKind::Cleric))
        );
        assert_eq!(
            parse_chain_command("!tank Beefwich 9 16"),
            Some((
                ChainCommand::TankRange {
                    tank: "Beefwich".into(),
                    from: 9,
                    to: 16
                },
                ChainKind::Cleric
            ))
        );
        assert_eq!(
            parse_chain_command("!untank Beefwich"),
            Some((
                ChainCommand::Untank {
                    tank: "Beefwich".into()
                },
                ChainKind::Cleric
            ))
        );
        assert_eq!(
            parse_chain_command("!chain 3 Beefwich"),
            Some((
                ChainCommand::ChainInterval {
                    seconds: 3.0,
                    tank: Some("Beefwich".into())
                },
                ChainKind::Cleric
            ))
        );
        assert_eq!(
            parse_chain_command("!take AAA"),
            Some((ChainCommand::Take { number: 1, player: None }, ChainKind::Rampage))
        );
        assert_eq!(
            parse_chain_command("!rtake BBB"),
            Some((ChainCommand::Take { number: 2, player: None }, ChainKind::Rampage))
        );
        assert_eq!(
            parse_chain_command("!take 001 Portlia"),
            Some((
                ChainCommand::Take {
                    number: 1,
                    player: Some("Portlia".into())
                },
                ChainKind::Cleric
            ))
        );
        assert_eq!(
            parse_chain_command("!rtake CCC Curaja"),
            Some((
                ChainCommand::Take {
                    number: 3,
                    player: Some("Curaja".into())
                },
                ChainKind::Rampage
            ))
        );
        assert_eq!(
            parse_chain_command("!take AAA Hanbox"),
            Some((
                ChainCommand::Take {
                    number: 1,
                    player: Some("Hanbox".into())
                },
                ChainKind::Rampage
            ))
        );
        assert_eq!(
            parse_chain_command("!rstartchain"),
            Some((ChainCommand::StartChain { tank: None }, ChainKind::Rampage))
        );
        assert_eq!(
            parse_chain_command("!rt Mluian"),
            Some((
                ChainCommand::MainTank {
                    tank: "Mluian".into()
                },
                ChainKind::Rampage
            ))
        );
        assert_eq!(parse_chain_command("!rt"), None);
        assert_eq!(parse_chain_command("!rot Beefwich"), None);
        assert_eq!(parse_chain_command("!rsplit CCC"), None);
        assert_eq!(parse_chain_command("!rtank Beefwich AAA FFF"), None);
        assert_eq!(parse_chain_command("!runtank Beefwich"), None);
        assert_eq!(parse_chain_command("!rmt Mluian"), None);
        assert_eq!(parse_chain_command("!rreset-chain"), None);
        assert_eq!(parse_chain_command("!rresetchain"), None);
        assert_eq!(parse_chain_command("!rreset"), None);
        assert_eq!(parse_letter_slot("AAA"), Some(1));
        assert_eq!(parse_letter_slot("c"), Some(3));
        assert_eq!(parse_letter_slot("ABC"), None);
        assert_eq!(parse_letter_slot("GG"), None);
        assert_eq!(parse_letter_slot("CC"), None);
        assert_eq!(parse_letter_slot("CCC"), Some(3));
    }

    #[test]
    fn built_in_patterns_compile_and_match_gg_and_cc() {
        let parser = Parser::new();
        let ParseResult::CompleteHeal(call) = parser.parse_line(
            "[Fri Sep 18 16:27:00 2026] You shout, 'GG 004 CH -- Mluian'",
            Some("Clericone"),
        ) else {
            panic!("built-in GG pattern");
        };
        assert_eq!(call.number, 4);
        assert_eq!(call.target, "Mluian");
        assert!(matches!(
            parser.parse_line(
                "[Fri Sep 18 16:27:00 2026] You shout, 'HEAL 4 on Mluian'",
                Some("Clericone"),
            ),
            ParseResult::Ignored
        ));
    }

    #[test]
    fn parse_rch_letter_slots() {
        let ParseResult::RampageHeal(call) = line(
            "[Fri Sep 18 16:27:00 2026] Curaja shouts, 'GG AAA RCH -- Mluian'",
        ) else {
            panic!("rch");
        };
        assert_eq!(call.number, 1);
        assert_eq!(call.target, "Mluian");
        let ParseResult::RampageHeal(call) = line(
            "[Fri Sep 18 16:27:00 2026] You shout, 'GG RCH CCC -- Beefwich'",
        ) else {
            panic!("rch you");
        };
        assert_eq!(call.number, 3);
        assert!(call.is_you);
        assert!(matches!(
            shout("GG RCH -- Mluian"),
            ParseResult::MalformedHeal { rampage: true, .. }
        ));
        assert!(matches!(
            line("[Fri Sep 18 16:27:00 2026] Two tells the guild, '!rskip'"),
            ParseResult::Command(ChainCommand::Skip { number: None }, _, _, ChainKind::Rampage)
        ));
        assert!(matches!(
            shout("GG 001 CH -- Mluian"),
            ParseResult::CompleteHeal(_)
        ));
        assert!(matches!(
            parse_chain_command("!skip AAA"),
            Some((ChainCommand::Skip { number: Some(1) }, ChainKind::Rampage))
        ));
        assert!(matches!(
            parse_chain_command("!back BBB"),
            Some((ChainCommand::Back { number: Some(2) }, ChainKind::Rampage))
        ));
        assert!(matches!(
            parse_chain_command("!skip 001"),
            Some((ChainCommand::Skip { number: Some(1) }, ChainKind::Cleric))
        ));
    }

    #[test]
    fn format_test_log_line_matches_eq_speech() {
        assert_eq!(
            format_test_log_line("YOU", TestChannel::Shout, "!take 001").unwrap(),
            "[Fri Sep 18 16:27:00 2026] You shout, '!take 001'"
        );
        assert_eq!(
            format_test_log_line("Hanbox", TestChannel::Ooc, "GG 001 CH -- Beefwich").unwrap(),
            "[Fri Sep 18 16:27:00 2026] Hanbox says out of character, 'GG 001 CH -- Beefwich'"
        );
        assert_eq!(
            format_test_log_line("Portlia", TestChannel::Group, "!skip").unwrap(),
            "[Fri Sep 18 16:27:00 2026] Portlia tells the group, '!skip'"
        );
        assert_eq!(
            format_test_log_line("Curaja", TestChannel::Guild, "GG 014 CH -- Wreckognize").unwrap(),
            "[Fri Sep 18 16:27:00 2026] Curaja tells the guild, 'GG 014 CH -- Wreckognize'"
        );
        assert_eq!(
            format_test_log_line("you", TestChannel::Auction, "!take AAA").unwrap(),
            "[Fri Sep 18 16:27:00 2026] You auction, '!take AAA'"
        );
        assert_eq!(
            format_test_log_line("Two", TestChannel::Raid, "!startchain").unwrap(),
            "[Fri Sep 18 16:27:00 2026] Two tells the raid, '!startchain'"
        );
        assert_eq!(
            format_test_log_line("YOU", TestChannel::Say, "!take 001").unwrap(),
            "[Fri Sep 18 16:27:00 2026] You say, '!take 001'"
        );
        assert!(format_test_log_line("YOU", TestChannel::Shout, "  ").is_err());
    }

    #[test]
    fn format_test_log_line_parses_like_a_real_log() {
        let you_take = format_test_log_line("YOU", TestChannel::Shout, "!take 001").unwrap();
        assert!(matches!(
            parser().parse_line(&you_take, Some("Portlia")),
            ParseResult::Command(
                ChainCommand::Take {
                    number: 1,
                    player: None
                },
                _,
                true,
                ChainKind::Cleric
            )
        ));
        let named = format_test_log_line("Hanbox", TestChannel::Guild, "GG 001 CH -- Beefwich")
            .unwrap();
        let ParseResult::CompleteHeal(call) = parser().parse_line(&named, Some("Portlia")) else {
            panic!("expected CH");
        };
        assert_eq!(call.speaker, "Hanbox");
        assert!(!call.is_you);
        assert_eq!(call.number, 1);
        for channel in [
            TestChannel::Shout,
            TestChannel::Ooc,
            TestChannel::Group,
            TestChannel::Guild,
            TestChannel::Auction,
            TestChannel::Raid,
            TestChannel::Say,
        ] {
            let line = format_test_log_line("YOU", channel, "!take 002").unwrap();
            assert!(
                matches!(
                    parser().parse_line(&line, Some("Portlia")),
                    ParseResult::Command(ChainCommand::Take { number: 2, .. }, _, true, _)
                ),
                "channel {channel:?} should parse"
            );
        }
    }

    #[test]
    fn looks_like_ch_macro_requires_more_than_the_word_ch() {
        assert!(!looks_like_ch_macro("I like to CH in my spare time", "GG"));
        assert!(looks_like_ch_macro("GG CH -- Tank", "GG"));
        assert!(!looks_like_ch_macro("CH 12", "GG"));
        assert!(!looks_like_ch_macro("CA 015 CH -- Tank", "GG"));
        assert!(looks_like_ch_macro("CA 015 CH -- Tank", "CA"));
        assert!(!looks_like_ch_macro("complete CH --", "GG"));
        assert!(!looks_like_ch_macro("GG AAA RCH -- Tank", "GG"));
        assert!(looks_like_rch_macro("GG AAA RCH -- Tank", "GG"));
        assert!(!looks_like_rch_macro("ST HHH CH -- Kaido", "GG"));
        assert!(!looks_like_rch_macro("GG 001 CH -- Tank", "GG"));
    }
}
