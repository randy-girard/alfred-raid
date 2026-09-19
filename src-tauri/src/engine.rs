use crate::chain::ChainState;
use crate::parser::{ChainKind, ParseResult, Parser};

pub fn apply_lines(
    parser: &Parser,
    chain: &mut ChainState,
    rampage: &mut ChainState,
    character: Option<&str>,
    lines: &[String],
) -> bool {
    if let Some(name) = character {
        chain.set_your_name(name.to_string());
        rampage.set_your_name(name.to_string());
    }
    let your_name = chain.your_name.clone();
    let mut changed = false;
    for line in lines {
        match parser.parse_line(line, your_name.as_deref()) {
            ParseResult::CompleteHeal(call) => {
                chain.apply_heal(call);
                changed = true;
            }
            ParseResult::RampageHeal(call) => {
                rampage.apply_heal(call);
                changed = true;
            }
            ParseResult::Command(cmd, speaker, _, kind) => {
                apply_command(chain, rampage, cmd, speaker, kind);
                changed = true;
            }
            ParseResult::MalformedHeal {
                speaker,
                message,
                rampage: is_rampage,
            } => {
                let target = if is_rampage {
                    &mut *rampage
                } else {
                    &mut *chain
                };
                let kind = if is_rampage { "RCH" } else { "CH" };
                target.set_warning(format!(
                    "{speaker} sent a {kind} macro Alfred could not read: {message}"
                ));
                changed = true;
            }
            ParseResult::Ignored => {}
        }
    }
    changed
}

fn apply_command(
    chain: &mut ChainState,
    rampage: &mut ChainState,
    cmd: crate::parser::ChainCommand,
    speaker: String,
    kind: ChainKind,
) {
    match kind {
        ChainKind::Cleric => {
            if let Some(warning) = chain.apply_command(cmd, speaker) {
                chain.set_warning(warning);
            }
        }
        ChainKind::Rampage => {
            if let Some(warning) = rampage.apply_command(cmd, speaker) {
                rampage.set_warning(warning);
            }
        }
        ChainKind::Both => {
            let ch = chain.apply_command(cmd.clone(), speaker.clone());
            let rch = rampage.apply_command(cmd, speaker);
            if ch.is_some() && rch.is_some() {
                if let Some(warning) = ch {
                    chain.set_warning(warning);
                }
                if let Some(warning) = rch {
                    rampage.set_warning(warning);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn ts(rest: &str) -> String {
        format!("[Fri Sep 18 16:27:00 2026] {rest}")
    }

    fn apply_ch(
        parser: &Parser,
        chain: &mut ChainState,
        character: Option<&str>,
        lines: &[String],
    ) -> bool {
        let mut rampage = ChainState::new_rampage(2.0, 10.0);
        apply_lines(parser, chain, &mut rampage, character, lines)
    }

    fn run(lines: &[&str]) -> ChainState {
        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        let owned: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
        apply_ch(&parser, &mut chain, Some("Clericone"), &owned);
        chain
    }

    fn run_rampage(lines: &[&str]) -> ChainState {
        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        let mut rampage = ChainState::new_rampage(2.0, 10.0);
        let owned: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
        apply_lines(&parser, &mut chain, &mut rampage, Some("Clericone"), &owned);
        rampage
    }

    #[test]
    fn ignored_combat_does_not_change_chain() {
        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        let changed = apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("You hit a goblin for 12 points of damage.")],
        );
        assert!(!changed);
        assert!(chain.slots.is_empty());
    }

    #[test]
    fn raid_sequence_assigns_tank_slots_and_current() {
        let chain = run(&[
            &ts("Leadcleric tells the guild, '!mt Mluian'"),
            &ts("You tell the guild, '!take 001'"),
            &ts("Two tells the guild, '!take 002'"),
            &ts("Three tells the guild, '!take 003'"),
            &ts("You shout, 'GG 001 CH -- Mluian'"),
        ]);
        assert_eq!(chain.tank.as_deref(), Some("Mluian"));
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.slots.get(&2).unwrap().player, "Two");
        assert_eq!(chain.current_number, Some(1));
        assert_eq!(chain.next_after(1), Some(2));
        let snap = chain.snapshot();
        assert_eq!(snap.next_number, Some(2));
        assert!(snap.you_are_next_in.is_none());
    }

    #[test]
    fn skip_and_back_change_who_is_next() {
        let mut chain = run(&[
            &ts("You tell the guild, '!take 001'"),
            &ts("Two tells the guild, '!take 002'"),
            &ts("Three tells the guild, '!take 003'"),
            &ts("You shout, 'GG 001 CH -- Mluian'"),
            &ts("Leadcleric tells the group, '!skip 002'"),
        ]);
        assert_eq!(chain.next_after(1), Some(3));
        let parser = Parser::new();
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("Leadcleric says out of character, '!back 002'")],
        );
        assert_eq!(chain.next_after(1), Some(2));
    }

    #[test]
    fn skip_and_back_without_number_use_the_speaker() {
        let mut chain = run(&[
            &ts("You tell the guild, '!take 001'"),
            &ts("Two tells the guild, '!take 002'"),
            &ts("You shout, '!skip'"),
        ]);
        assert!(chain.skipped.contains(&1));
        assert!(!chain.skipped.contains(&2));
        let parser = Parser::new();
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("Two tells the group, '!skip'")],
        );
        assert!(chain.skipped.contains(&2));
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("You tell the guild, '!back'")],
        );
        assert!(!chain.skipped.contains(&1));
        assert!(chain.skipped.contains(&2));
    }

    #[test]
    fn malformed_macro_sets_warning_and_valid_heal_clears_it() {
        let mut chain = run(&[&ts("Curaja shouts, 'GG CH -- Wreckognize'")]);
        assert!(chain.warning.as_deref().unwrap().contains("could not read"));
        let parser = Parser::new();
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("Curaja auctions, 'GG 014 CH -- Wreckognize'")],
        );
        assert_eq!(chain.warning, None);
        assert_eq!(chain.slots.get(&14).unwrap().player, "Curaja");
    }

    #[test]
    fn reset_clears_slots_but_keeps_tank_and_interval() {
        let mut chain = run(&[
            &ts("Leadcleric tells the guild, '!mt Mluian'"),
            &ts("Leadcleric tells the guild, '!chain 2.5'"),
            &ts("You shout, 'GG 001 CH -- Mluian'"),
            &ts("Leadcleric tells the guild, '!reset-chain'"),
        ]);
        assert!(chain.slots.is_empty());
        assert_eq!(chain.current_number, None);
        assert!(!chain.running);
        assert_eq!(chain.tank.as_deref(), Some("Mluian"));
        assert_eq!(chain.interval_seconds, 2.5);
        let parser = Parser::new();
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("Leadcleric tells the guild, '!resetchain'")],
        );
        assert!(chain.slots.is_empty());
    }

    #[test]
    fn you_placeholder_is_rewritten_when_character_is_learned() {
        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        apply_ch(
            &parser,
            &mut chain,
            None,
            &[ts("You tell the guild, '!take 001'")],
        );
        assert_eq!(chain.slots.get(&1).unwrap().player, "You");
        apply_ch(&parser, &mut chain, Some("Clericone"), &[]);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.your_name.as_deref(), Some("Clericone"));
    }

    #[test]
    fn failed_skip_sets_warning() {
        let chain = run(&[&ts("Leadcleric tells the guild, '!skip 004'")]);
        assert!(chain.warning.as_deref().unwrap().contains("004"));
    }

    #[test]
    fn shouting_moves_a_cleric_to_a_new_number() {
        let chain = run(&[
            &ts("Two shouts, 'GG 002 CH -- Mluian'"),
            &ts("Two shouts, 'GG 005 CH -- Mluian'"),
        ]);
        assert!(chain.slots.get(&2).is_none());
        assert_eq!(chain.slots.get(&5).unwrap().player, "Two");
        assert_eq!(chain.current_number, Some(5));
    }

    #[test]
    fn take_command_moves_a_cleric_to_a_new_number() {
        let chain = run(&[
            &ts("Two tells the guild, '!take 002'"),
            &ts("Two tells the guild, '!take 005'"),
        ]);
        assert!(chain.slots.get(&2).is_none());
        assert_eq!(chain.slots.get(&5).unwrap().player, "Two");
        assert_eq!(chain.current_number, None);
        assert!(!chain.running);
        assert!(chain.slots.get(&5).unwrap().last_shout_ms.is_none());
    }

    #[test]
    fn take_without_a_number_assigns_the_next_free_slot() {
        let chain = run(&[
            &ts("You tell the guild, '!take'"),
            &ts("Two tells the guild, '!take'"),
            &ts("Three tells the guild, '!take'"),
        ]);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.slots.get(&2).unwrap().player, "Two");
        assert_eq!(chain.slots.get(&3).unwrap().player, "Three");
        assert_eq!(chain.warning.as_deref(), Some("Three got 003."));
    }

    #[test]
    fn take_with_a_name_sets_that_character() {
        let chain = run(&[
            &ts("You tell the guild, '!take 001'"),
            &ts("You tell the guild, '!take 008 Portlia'"),
        ]);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.slots.get(&8).unwrap().player, "Portlia");
    }

    #[test]
    fn start_and_end_toggle_the_clock_after_take() {
        let mut chain = run(&[&ts("You tell the guild, '!take 001'")]);
        assert!(!chain.running);
        let parser = Parser::new();
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("Leadcleric tells the raid, '!start'")],
        );
        assert!(!chain.running);
        assert!(chain.snapshot().armed);
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("You shout, 'GG 001 CH -- Mluian'")],
        );
        assert!(chain.running);
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("You tell your raid, '!stop chain'")],
        );
        assert!(!chain.running);
        assert_eq!(chain.slots.len(), 1);
    }

    #[test]
    fn split_ot_chain_is_separate_and_hidden_from_you() {
        let chain = run(&[
            &ts("Leadcleric tells the guild, '!mt Mluian'"),
            &ts("You tell the guild, '!take 001'"),
            &ts("Two tells the guild, '!take 002'"),
            &ts("Three tells the guild, '!take 009'"),
            &ts("Leadcleric tells the raid, '!ot Beefwich'"),
            &ts("Leadcleric tells the raid, '!split 9'"),
            &ts("Leadcleric tells the raid, '!startchain'"),
        ]);
        assert!(!chain.running);
        assert!(chain.snapshot().armed);
        let snap = chain.snapshot();
        assert!(snap
            .tanks
            .iter()
            .any(|tank| tank.name == "Beefwich" && tank.armed));
        assert_eq!(snap.your_tank.as_deref(), Some("Mluian"));
        assert!(snap.slots.iter().all(|slot| slot.number < 9));
        assert!(snap
            .slots
            .iter()
            .all(|slot| slot.tank.as_deref() == Some("Mluian")));
    }

    #[test]
    fn shouting_an_occupied_number_warns_and_does_not_kick_the_occupant() {
        let chain = run(&[
            &ts("You shout, 'GG 001 CH -- Mluian'"),
            &ts("Two shouts, 'GG 001 CH -- Mluian'"),
        ]);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert!(chain.slots.values().all(|slot| slot.player != "Two"));
        assert!(chain.warning.as_deref().unwrap().contains("already taken"));
        assert!(!chain.warning_urgent);
    }

    #[test]
    fn shouting_the_wrong_ch_target_warns_when_on_a_running_chain() {
        let chain = run(&[
            &ts("Leadcleric tells the guild, '!mt Mluian'"),
            &ts("You shout, 'GG 001 CH -- Mluian'"),
            &ts("Leadcleric tells the guild, '!startchain'"),
            &ts("You shout, 'GG 001 CH -- Mluian'"),
            &ts("You shout, 'GG 001 CH -- Portlia'"),
        ]);
        assert_eq!(
            chain.warning.as_deref(),
            Some("You CHed Portlia instead of Mluian.")
        );
        assert!(chain.warning_urgent);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
    }

    #[test]
    fn start_and_end_from_any_channel() {
        let mut chain = run(&[
            &ts("You tell the guild, '!take 001'"),
            &ts("Two tells the group, '!take 002'"),
            &ts("Leadcleric auctions, '!startchain'"),
        ]);
        assert!(!chain.running);
        assert!(chain.snapshot().armed);
        let parser = Parser::new();
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[ts("You say, '!stopchain'")],
        );
        assert!(!chain.running);
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[
                ts("Leadcleric says, '!startchain'"),
                ts("You say out of character, '!stopchain'"),
            ],
        );
        assert!(!chain.running);
    }

    #[test]
    fn shout_updates_offset_while_running() {
        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        chain.set_your_name("Clericone".into());
        apply_ch(
            &parser,
            &mut chain,
            Some("Clericone"),
            &[
                ts("You tell the guild, '!take 001'"),
                ts("Two tells the guild, '!take 002'"),
            ],
        );
        chain.apply_command_at(
            crate::parser::ChainCommand::StartChain { tank: None },
            "Lead".into(),
            10_000,
        );
        chain.apply_heal_at(
            crate::parser::CompleteHealCall {
                speaker: "Clericone".into(),
                is_you: true,
                number: 1,
                target: "Mluian".into(),
                tag: Some("GG".into()),
                raw: "GG 001 CH -- Mluian".into(),
            },
            10_000,
        );
        chain.apply_heal_at(
            crate::parser::CompleteHealCall {
                speaker: "Clericone".into(),
                is_you: true,
                number: 1,
                target: "Mluian".into(),
                tag: Some("GG".into()),
                raw: "GG 001 CH -- Mluian".into(),
            },
            10_180,
        );
        assert!((chain.slots.get(&1).unwrap().last_offset_seconds.unwrap() - 0.18).abs() < 0.05);
    }

    #[test]
    fn rampage_rch_and_letter_commands_use_the_rampage_chain() {
        let rampage = run_rampage(&[
            &ts("You shout, 'GG AAA RCH -- Mluian'"),
            &ts("Two shouts, 'GG BBB RCH -- Mluian'"),
            &ts("Leadcleric tells the raid, '!take CCC'"),
            &ts("Leadcleric tells the raid, '!startchain'"),
        ]);
        assert_eq!(rampage.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(rampage.slots.get(&2).unwrap().player, "Two");
        assert_eq!(rampage.slots.get(&3).unwrap().player, "Leadcleric");
        assert!(!rampage.running);
        assert!(rampage.snapshot().armed);
        let named = run_rampage(&[&ts("Leadcleric tells the raid, '!rt Mluian'")]);
        assert_eq!(named.tank.as_deref(), Some("Mluian"));
        let ignored_split = run_rampage(&[&ts("Leadcleric tells the raid, '!rot Beefwich'")]);
        assert!(ignored_split.tank.is_none());
        let ch = run(&[
            &ts("You shout, 'GG AAA RCH -- Mluian'"),
            &ts("You tell the guild, '!take 001'"),
        ]);
        assert!(ch.slots.get(&1).is_some());
        assert_eq!(ch.slots.get(&1).unwrap().player, "Clericone");
        assert!(ch.slots.get(&2).is_none());
    }

    #[test]
    fn startchain_starts_ch_and_rampage_together() {
        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        let mut rampage = ChainState::new_rampage(2.0, 10.0);
        apply_lines(
            &parser,
            &mut chain,
            &mut rampage,
            Some("Clericone"),
            &[
                ts("You shout, 'GG 001 CH -- Mluian'"),
                ts("You shout, 'GG AAA RCH -- Mluian'"),
                ts("Two shouts, 'GG BBB RCH -- Mluian'"),
                ts("Leadcleric tells the raid, '!startchain'"),
            ],
        );
        assert!(!chain.running);
        assert!(chain.snapshot().armed);
        assert_eq!(
            chain.snapshot().warning.as_deref(),
            Some("Chain is starting.")
        );
        assert!(!rampage.running);
        assert!(rampage.snapshot().armed);
        assert_eq!(
            rampage.snapshot().warning.as_deref(),
            Some("Chain is starting.")
        );
        apply_lines(
            &parser,
            &mut chain,
            &mut rampage,
            Some("Clericone"),
            &[
                ts("You shout, 'GG 001 CH -- Mluian'"),
                ts("Two shouts, 'GG BBB RCH -- Mluian'"),
            ],
        );
        assert!(chain.running);
        assert_eq!(chain.snapshot().current_number, Some(1));
        assert!(rampage.running);
        assert_eq!(rampage.snapshot().current_number, Some(2));
        apply_lines(
            &parser,
            &mut chain,
            &mut rampage,
            Some("Clericone"),
            &[ts("Leadcleric tells the raid, '!stopchain'")],
        );
        assert!(!chain.running);
        assert!(!rampage.running);
    }

    #[test]
    fn ch_macros_do_not_fill_the_rampage_chain() {
        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        let mut rampage = ChainState::new_rampage(2.0, 10.0);
        apply_lines(
            &parser,
            &mut chain,
            &mut rampage,
            Some("Clericone"),
            &[
                ts("You shout, 'GG 001 CH -- Mluian'"),
                ts("Two shouts, 'GG BBB RCH -- Mluian'"),
            ],
        );
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert!(chain.slots.get(&2).is_none());
        assert!(rampage.slots.get(&1).is_none());
        assert_eq!(rampage.slots.get(&2).unwrap().player, "Two");
        assert_eq!(
            rampage.snapshot().slot_format,
            crate::chain::SlotFormat::Letter
        );
    }

    #[test]
    fn skip_and_back_without_a_slot_apply_to_both_chains() {
        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        let mut rampage = ChainState::new_rampage(2.0, 10.0);
        apply_lines(
            &parser,
            &mut chain,
            &mut rampage,
            Some("Clericone"),
            &[
                ts("You shout, 'GG 001 CH -- Mluian'"),
                ts("You shout, 'GG AAA RCH -- Mluian'"),
                ts("Two shouts, 'GG BBB RCH -- Mluian'"),
                ts("You shout, '!skip'"),
            ],
        );
        assert!(chain.skipped.contains(&1));
        assert!(rampage.skipped.contains(&1));
        assert!(!rampage.skipped.contains(&2));
        assert!(chain.warning.is_none());
        assert!(rampage.warning.is_none());
        apply_lines(
            &parser,
            &mut chain,
            &mut rampage,
            Some("Clericone"),
            &[ts("You shout, '!back'")],
        );
        assert!(!chain.skipped.contains(&1));
        assert!(!rampage.skipped.contains(&1));
        apply_lines(
            &parser,
            &mut chain,
            &mut rampage,
            Some("Clericone"),
            &[ts("You shout, '!skip AAA'")],
        );
        assert!(!chain.skipped.contains(&1));
        assert!(rampage.skipped.contains(&1));
    }

    #[test]
    fn formatted_test_lines_apply_like_log_lines() {
        use crate::parser::{format_test_log_line, TestChannel};

        let parser = Parser::new();
        let mut chain = ChainState::new(2.0, 10.0);
        let take = format_test_log_line("YOU", TestChannel::Shout, "!take 001").unwrap();
        let heal = format_test_log_line("YOU", TestChannel::Shout, "GG 001 CH -- Mluian").unwrap();
        apply_ch(&parser, &mut chain, Some("Clericone"), &[take, heal]);
        assert_eq!(chain.slots.get(&1).unwrap().player, "Clericone");
        assert_eq!(chain.current_number, Some(1));
        let other = format_test_log_line("Two", TestChannel::Guild, "!take 002").unwrap();
        apply_ch(&parser, &mut chain, Some("Clericone"), &[other]);
        assert_eq!(chain.slots.get(&2).unwrap().player, "Two");
    }
}
