use colored::Colorize;

use crate::card::Card;
use crate::error::PokerError;
use crate::eval;
use crate::hand_state::{Action, HandState, Street};
use crate::outs;
use crate::position::{self, Position};
use crate::pot::PotOdds;
use crate::preflop::{self, HoleCardType};
use crate::table_display;

pub fn execute(state: &mut HandState, input: &str) -> Result<Option<String>, PokerError> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(None);
    }

    // Auto-detect raw card input: starts with a rank char.
    // Routes to deal/flop/turn/river based on current street.
    if input.chars().next().map(is_rank_char).unwrap_or(false) {
        return parse_auto_cards(state, input);
    }

    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts[0].to_lowercase();
    let args = &parts[1..];

    // Compact odds shortcut: b<bet>p<pot>
    if cmd.len() > 1
        && cmd != "blinds"
        && let Some(rest) = cmd.strip_prefix('b')
        && !rest.is_empty()
    {
        return parse_compact_odds(state, rest);
    }

    match cmd.as_str() {
        "n" | "new" => cmd_new(state),
        "odds" => cmd_odds(state, args),
        "limp" => cmd_action(state, Action::FacingLimp, &[]),
        "raise" => cmd_action(state, Action::FacingRaise, args),
        "first" | "firstin" => cmd_action(state, Action::FirstIn, &[]),
        "players" => cmd_players(state, args),
        "pos" => cmd_pos(state, args),
        "blinds" => cmd_blinds(state, args),
        "ranges" => cmd_ranges(),
        "help" => cmd_help(),
        "quit" | "exit" => std::process::exit(0),
        _ => Ok(Some(format!(
            "Unknown command: '{cmd}' — type 'help' for a list of commands"
        ))),
    }
}

// --- Auto card-input parsing ---

fn is_rank_char(c: char) -> bool {
    matches!(
        c,
        '1'..='9' | 'T' | 'J' | 'Q' | 'K' | 'A' | 't' | 'j' | 'q' | 'k' | 'a'
    )
}

/// Split a string into cards by finding suit characters (s, h, d, c).
fn find_suit_positions(chars: &[char]) -> Vec<usize> {
    let mut positions: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, 's' | 'h' | 'c' | 'd'))
        .map(|(i, _)| i)
        .collect();
    positions.sort();
    positions
}

/// Route raw card input based on the current street.
fn parse_auto_cards(state: &mut HandState, input: &str) -> Result<Option<String>, PokerError> {
    if !state.configured {
        return Err(PokerError::NotConfigured);
    }

    // Tolerate spaces between cards: "2h 3s 4c" works as well as "2h3s4c".
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();

    match (state.street, state.hole_cards.is_some()) {
        (Street::Preflop, false) => parse_deal_input(state, &cleaned),
        (Street::Preflop, true) => parse_board_cards(state, &cleaned, 3),
        (Street::Flop, _) => parse_board_cards(state, &cleaned, 1),
        (Street::Turn, _) => parse_board_cards(state, &cleaned, 1),
        (Street::River, _) => Err(PokerError::WrongStreet {
            expected: "River already dealt — type 'n' for the next hand",
        }),
    }
}

/// Parse hole cards with optional trailing action suffix: "2h8s", "AhKsr60", "ThKcl"
fn parse_deal_input(state: &mut HandState, cleaned: &str) -> Result<Option<String>, PokerError> {
    let lower = cleaned.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let suits = find_suit_positions(&chars);

    if suits.len() < 2 {
        return Err(PokerError::WrongArgCount {
            command: "",
            usage: "<c1><c2>[l|r[amount]] — e.g. AhKs, 2h8sl, AhKsr60",
        });
    }

    let card1_str: String = chars[..=suits[0]].iter().collect();
    let card2_str: String = chars[suits[0] + 1..=suits[1]].iter().collect();
    let remainder: String = chars[suits[1] + 1..].iter().collect();

    let card1 = Card::parse(&card1_str)?;
    let card2 = Card::parse(&card2_str)?;

    if card1 == card2 {
        return Err(PokerError::DuplicateCard(card1));
    }

    let (action, raise_amount) = parse_action_suffix(&remainder)?;

    state.hole_cards = Some([card1, card2]);
    state.action = action;
    state.raise_amount = raise_amount;

    Ok(Some(format_recommendation(state)))
}

fn parse_action_suffix(rest: &str) -> Result<(Action, Option<u64>), PokerError> {
    if rest.is_empty() {
        return Ok((Action::FirstIn, None));
    }
    if rest == "l" || rest == "limp" {
        return Ok((Action::FacingLimp, None));
    }
    if rest == "r" || rest == "raise" {
        return Ok((Action::FacingRaise, None));
    }
    if let Some(amt_str) = rest.strip_prefix('r')
        && let Ok(amt) = amt_str.parse::<u64>()
        && amt > 0
    {
        return Ok((Action::FacingRaise, Some(amt)));
    }
    Err(PokerError::WrongArgCount {
        command: "",
        usage: "<c1><c2>[l|r[amount]] — e.g. AhKs, 2h8sl, AhKsr60",
    })
}

/// Parse exactly `expected` cards (1 or 3) with no trailing junk.
fn parse_board_cards(
    state: &mut HandState,
    cleaned: &str,
    expected: usize,
) -> Result<Option<String>, PokerError> {
    let lower = cleaned.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let suits = find_suit_positions(&chars);

    let usage: &'static str = match (expected, state.street) {
        (3, _) => "flop: <c1><c2><c3> — e.g. 2h3s4c",
        (1, Street::Flop) => "turn: <card> — e.g. 5d",
        (1, Street::Turn) => "river: <card> — e.g. 6h",
        _ => "card",
    };

    if suits.len() != expected || suits[expected - 1] != chars.len() - 1 {
        return Err(PokerError::WrongArgCount { command: "", usage });
    }

    let mut cards = Vec::with_capacity(expected);
    let mut start = 0;
    for &end in &suits {
        let s: String = chars[start..=end].iter().collect();
        cards.push(Card::parse(&s)?);
        start = end + 1;
    }

    match (state.street, state.hole_cards.is_some()) {
        (Street::Preflop, true) => {
            let arr: [Card; 3] = [cards[0], cards[1], cards[2]];
            do_flop(state, &arr)
        }
        (Street::Flop, _) => do_turn(state, cards[0]),
        (Street::Turn, _) => do_river(state, cards[0]),
        _ => unreachable!(),
    }
}

/// Parse compact odds: "25p50" → bet=25, pot=50
fn parse_compact_odds(state: &mut HandState, input: &str) -> Result<Option<String>, PokerError> {
    let lower = input.to_lowercase();
    let parts: Vec<&str> = lower.split('p').collect();

    if parts.len() != 2 {
        return Err(PokerError::WrongArgCount {
            command: "b",
            usage: "<bet>p<pot> — e.g. b25p50",
        });
    }

    let bet: u64 = parts[0].parse().map_err(|_| PokerError::WrongArgCount {
        command: "b",
        usage: "<bet>p<pot> — e.g. b25p50",
    })?;
    let pot: u64 = parts[1].parse().map_err(|_| PokerError::WrongArgCount {
        command: "b",
        usage: "<bet>p<pot> — e.g. b25p50",
    })?;

    do_odds(state, bet, pot)
}

// --- Command implementations ---

fn cmd_new(state: &mut HandState) -> Result<Option<String>, PokerError> {
    state.reset();
    state.advance_position();
    let pos = state.position().unwrap();
    let table = table_display::render_table(state.num_players, Some(pos));
    Ok(Some(format!(
        "{table}\n\nPosition advanced to {} ({}).",
        pos.short_name(),
        pos.long_name()
    )))
}

fn cmd_odds(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.is_empty() {
        return do_hand_summary(state);
    }

    if args.len() != 2 {
        return Err(PokerError::WrongArgCount {
            command: "odds",
            usage: "[<bet> <pot>]  or  no args for hand summary",
        });
    }

    let bet: u64 = args[0].parse().map_err(|_| PokerError::WrongArgCount {
        command: "odds",
        usage: "<bet> <pot>",
    })?;
    let pot: u64 = args[1].parse().map_err(|_| PokerError::WrongArgCount {
        command: "odds",
        usage: "<bet> <pot>",
    })?;

    do_odds(state, bet, pot)
}

// --- Shared logic ---

fn do_flop(state: &mut HandState, cards: &[Card; 3]) -> Result<Option<String>, PokerError> {
    if state.hole_cards.is_none() {
        return Err(PokerError::NoDeal);
    }
    if state.street != Street::Preflop {
        return Err(PokerError::WrongStreet {
            expected: "Flop already set — enter the turn card (e.g. 5d)",
        });
    }

    state.check_duplicates(cards)?;
    state.board.extend_from_slice(cards);
    state.street = Street::Flop;

    Ok(Some(format_board_analysis(state)))
}

fn do_turn(state: &mut HandState, card: Card) -> Result<Option<String>, PokerError> {
    if state.hole_cards.is_none() {
        return Err(PokerError::NoDeal);
    }
    if state.street != Street::Flop {
        return Err(PokerError::WrongStreet {
            expected: "Enter the flop first (e.g. 2h3s4c)",
        });
    }

    state.check_not_duplicate(card)?;
    state.board.push(card);
    state.street = Street::Turn;

    Ok(Some(format_board_analysis(state)))
}

fn do_river(state: &mut HandState, card: Card) -> Result<Option<String>, PokerError> {
    if state.hole_cards.is_none() {
        return Err(PokerError::NoDeal);
    }
    if state.street != Street::Turn {
        return Err(PokerError::WrongStreet {
            expected: "Enter the turn first (e.g. 5d)",
        });
    }

    state.check_not_duplicate(card)?;
    state.board.push(card);
    state.street = Street::River;

    Ok(Some(format_board_analysis(state)))
}

fn do_hand_summary(state: &HandState) -> Result<Option<String>, PokerError> {
    if state.hole_cards.is_none() {
        return Err(PokerError::NoDeal);
    }
    if state.board.is_empty() {
        return Ok(Some("Deal a flop first to see outs and betting advice.".to_string()));
    }
    Ok(Some(format_board_analysis(state)))
}

fn bet_suggestion(made: &eval::MadeHand, equity: f64) -> String {
    use eval::MadeHand::*;
    use eval::PairQuality::*;

    let (action, sizing, why) = match made {
        StraightFlush(_) | FourOfAKind(_) => (
            "Bet for value",
            "2/3 to full pot",
            "You have a near-unbeatable hand. Bet big to get paid off.",
        ),
        FullHouse { .. } => (
            "Bet for value",
            "2/3 to full pot",
            "Very strong hand. Bet big — you want opponents to call with worse.",
        ),
        Flush | Straight(_) => (
            "Bet for value",
            "1/2 to 2/3 pot",
            "Strong made hand. Bet to build the pot while you're ahead.",
        ),
        ThreeOfAKind { is_set: true, .. } => (
            "Bet for value",
            "1/2 to 2/3 pot",
            "A set is well-disguised. Opponents won't see it coming — bet for value.",
        ),
        ThreeOfAKind { is_set: false, .. } => (
            "Bet cautiously",
            "1/3 to 1/2 pot",
            "Trips (board pair + your card) is strong but obvious. Bet smaller — opponents may already be wary.",
        ),
        TwoPair { .. } => (
            "Bet for value",
            "1/2 to 2/3 pot",
            "Two pair is strong but vulnerable to straights and flushes. Bet to charge draws.",
        ),
        Pair { quality: Overpair, .. } => (
            "Bet for value",
            "1/2 to 2/3 pot",
            "Your pocket pair beats everything on the board. Bet to protect against overcards hitting.",
        ),
        Pair { quality: Top, .. } => (
            "Bet for value/protection",
            "1/3 to 1/2 pot",
            "Top pair is decent but beatable. Bet enough to charge draws but don't overcommit.",
        ),
        Pair { quality: Second, .. } => {
            if equity >= 25.0 {
                (
                    "Bet small or check",
                    "1/3 pot",
                    "Second pair plus draws gives you options. A small bet can take it down or build equity.",
                )
            } else {
                (
                    "Check",
                    "—",
                    "Second pair with no draws is marginal. Check to control the pot size.",
                )
            }
        }
        Pair { .. } => (
            "Check",
            "—",
            "Weak pair — not worth betting. Check and see what develops.",
        ),
        HighCard(_) => {
            if equity >= 35.0 {
                (
                    "Semi-bluff",
                    "1/2 to 2/3 pot",
                    "You have nothing now but a strong draw. Betting wins two ways: they fold, or you hit your draw.",
                )
            } else if equity >= 20.0 {
                (
                    "Check or small stab",
                    "1/3 pot",
                    "Weak draw. A small bet might take it down, but don't put too much in.",
                )
            } else {
                (
                    "Check/fold",
                    "—",
                    "No made hand, no draw. Give up unless you can bluff on a good board.",
                )
            }
        }
    };

    if sizing == "—" {
        format!("Suggestion: {}\n{}", action.bold(), why)
    } else {
        format!("Suggestion: {} ({})\n{}", action.bold(), sizing, why)
    }
}

fn do_odds(state: &HandState, bet: u64, pot: u64) -> Result<Option<String>, PokerError> {
    let odds = PotOdds::calculate(pot, bet);

    let pct = if pot > 0 {
        (bet as f64 / pot as f64) * 100.0
    } else {
        0.0
    };
    let sizing_label = if pct <= 20.0 {
        "very small, under 1/4 pot"
    } else if pct <= 40.0 {
        "small, 1/4 to 1/3 pot"
    } else if pct <= 60.0 {
        "half pot"
    } else if pct <= 80.0 {
        "2/3 pot"
    } else if pct <= 110.0 {
        "pot-sized"
    } else {
        "overbet"
    };

    let mut output = format!(
        "${bet} into ${pot} pot ({pct:.0}% pot — {sizing_label})\nPot odds: need {:.1}% equity to break even",
        odds.required_equity
    );

    if let Some(hole) = &state.hole_cards {
        if state.board.is_empty() {
            output.push_str("\n(Deal a flop to compare against your equity)");
        } else {
            let made = eval::evaluate(hole, &state.board);
            output.push_str(&format!("\n\nYour hand: {}", made.to_string().bold()));

            let eq = if state.street == Street::River {
                0.0
            } else {
                let analysis = outs::analyze_outs(hole, &state.board, state.street);
                if analysis.total_outs > 0 {
                    output.push_str(&format!(
                        "\nYou have: ~{:.0}% equity ({} outs)",
                        analysis.equity_percent, analysis.total_outs
                    ));
                } else {
                    output.push_str("\nYou have: ~0% draw equity (no outs)");
                }
                analysis.equity_percent
            };

            output.push_str(&format!("\n{}", bet_suggestion(&made, eq)));

            if state.street != Street::River {
                if eq >= odds.required_equity {
                    output.push_str(&format!(
                        "\n\nVerdict: {}",
                        "CALL — profitable".green().bold()
                    ));
                } else {
                    output.push_str(&format!(
                        "\n\nVerdict: {}",
                        "FOLD — not enough equity".red()
                    ));
                }
            }
        }
    }

    Ok(Some(output))
}

fn format_board_analysis(state: &HandState) -> String {
    let hole = state.hole_cards.unwrap();
    let board_str = state
        .board
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ");

    let made = eval::evaluate(&hole, &state.board);

    let mut output = if state.street == Street::River {
        format!(
            "Board: {board_str}\nFinal hand: {}\n",
            made.to_string().green().bold()
        )
    } else {
        format!(
            "Board: {board_str}\nMade hand: {}\n",
            made.to_string().bold()
        )
    };

    if state.street != Street::River {
        let analysis = outs::analyze_outs(&hole, &state.board, state.street);
        let rule = if state.street == Street::Flop {
            "rule of 4"
        } else {
            "rule of 2"
        };

        if analysis.draws.is_empty() {
            output.push_str("No draws.\n");
        } else {
            output.push('\n');
            for draw in &analysis.draws {
                let outs_str = draw
                    .outs
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                output.push_str(&format!(
                    "  {} ({} outs): {}\n",
                    draw.draw_type,
                    draw.outs.len(),
                    outs_str
                ));
            }
            output.push_str(&format!(
                "\nTotal: {} outs (~{:.0}% equity, {})\n",
                analysis.total_outs, analysis.equity_percent, rule
            ));
        }

        output.push_str(&format!("\n{}", bet_suggestion(&made, analysis.equity_percent)));
    } else {
        output.push_str(&format!("\n{}", bet_suggestion(&made, 0.0)));
    }

    output
}

fn cmd_action(state: &mut HandState, action: Action, args: &[&str]) -> Result<Option<String>, PokerError> {
    if state.hole_cards.is_none() {
        return Err(PokerError::WrongArgCount {
            command: match action {
                Action::FacingLimp => "limp",
                Action::FacingRaise => "raise",
                Action::FirstIn => "first",
            },
            usage: "(deal cards first)",
        });
    }

    state.action = action;

    if action == Action::FacingRaise {
        state.raise_amount = args.first().and_then(|s| s.parse::<u64>().ok());
    } else {
        state.raise_amount = None;
    }

    Ok(Some(format_recommendation(state)))
}

pub fn format_recommendation(state: &HandState) -> String {
    let [card1, card2] = state.hole_cards.unwrap();
    let position = state.position().unwrap();

    let hole_type = HoleCardType::from_cards(card1, card2);
    let raise_bb = match (state.raise_amount, state.big_blind) {
        (Some(raise), Some(bb)) if bb > 0 => Some(raise as f64 / bb as f64),
        _ => None,
    };
    let rec = preflop::recommend(&hole_type, position, state.num_players, state.action, raise_bb);

    let (rec_colored, rec_desc) = match rec {
        preflop::Recommendation::Open => (
            "RAISE".green().bold(),
            "You're first in — raise to enter the pot.",
        ),
        preflop::Recommendation::Fold => (
            "FOLD".red().bold(),
            "Not worth playing from this position.",
        ),
        preflop::Recommendation::ThreeBet => (
            "3-BET".yellow().bold(),
            "Re-raise — you have a premium hand.",
        ),
        preflop::Recommendation::IsoRaise => (
            "RAISE".green().bold(),
            "Raise over the limper to isolate them and play heads-up.",
        ),
        preflop::Recommendation::Call => (
            "CALL".cyan().bold(),
            "Call the raise — good enough to see a flop but not to re-raise.",
        ),
        preflop::Recommendation::Check => (
            "CHECK".cyan().bold(),
            "You're in the big blind — check and see a free flop.",
        ),
    };

    let action_label = match (state.action, state.raise_amount, state.big_blind) {
        (Action::FirstIn, _, _) => String::new(),
        (Action::FacingLimp, _, _) => "  (facing limp)".to_string(),
        (Action::FacingRaise, Some(amt), Some(bb)) if bb > 0 => {
            let mult = amt as f64 / bb as f64;
            format!("  (facing raise: {amt} = {mult:.1}x BB)")
        }
        (Action::FacingRaise, Some(amt), _) => format!("  (facing raise: {amt})"),
        (Action::FacingRaise, None, _) => "  (facing raise)".to_string(),
    };

    let sizing = format_sizing(state.big_blind, state.raise_amount, rec);

    format!(
        "Hand: {card1} {card2}  ({label} — {category})\n\
         Position: {pos} ({long}){action_label}\n\
         Recommendation: {rec}\n\
         {desc}{sizing}",
        label = hole_type.label(),
        category = hole_type.category(),
        pos = position.short_name(),
        long = position.long_name(),
        rec = rec_colored,
        desc = rec_desc,
    )
}

fn format_sizing(big_blind: Option<u64>, raise_amount: Option<u64>, rec: preflop::Recommendation) -> String {
    match (rec, big_blind, raise_amount) {
        // Opening: 2.5-3x BB
        (preflop::Recommendation::Open, Some(bb), _) => {
            let low = bb * 5 / 2;
            let high = bb * 3;
            format!("\nSizing: 2.5–3x BB → {low}–{high}")
        }
        (preflop::Recommendation::Open, None, _) => {
            "\nSizing: 2.5–3x BB".to_string()
        }

        // 3-bet: 3x the raise if known, otherwise 7-10x BB
        (preflop::Recommendation::ThreeBet, Some(bb), Some(raise)) => {
            let size = raise * 3;
            let mult = size as f64 / bb as f64;
            format!("\nSizing: re-raise to {size} ({mult:.0}x BB)")
        }
        (preflop::Recommendation::ThreeBet, None, Some(raise)) => {
            let size = raise * 3;
            format!("\nSizing: re-raise to {size}")
        }
        (preflop::Recommendation::ThreeBet, Some(bb), None) => {
            let low = bb * 7;
            let high = bb * 10;
            format!("\nSizing: re-raise to {low}–{high} (7–10x BB)")
        }
        (preflop::Recommendation::ThreeBet, None, None) => {
            "\nSizing: re-raise to 7–10x BB".to_string()
        }

        // Iso-raise: 3-4x BB + 1 BB per limper
        (preflop::Recommendation::IsoRaise, Some(bb), _) => {
            let low = bb * 3;
            let high = bb * 4;
            format!("\nSizing: 3–4x BB + 1 BB per limper → {low}–{high} + {bb}/limper")
        }
        (preflop::Recommendation::IsoRaise, None, _) => {
            "\nSizing: 3–4x BB + 1 BB per limper".to_string()
        }

        // Call: show call cost if known
        (preflop::Recommendation::Call, Some(bb), Some(raise)) => {
            let mult = raise as f64 / bb as f64;
            format!("\nCost to call: {raise} ({mult:.1}x BB)")
        }
        (preflop::Recommendation::Call, _, Some(raise)) => {
            format!("\nCost to call: {raise}")
        }
        (preflop::Recommendation::Call, _, None) => {
            "\nCost to call: match the raise".to_string()
        }

        _ => String::new(),
    }
}

fn cmd_players(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.is_empty() {
        return Err(PokerError::WrongArgCount {
            command: "players",
            usage: "<2-9>",
        });
    }

    let n: u8 = args[0].parse().map_err(|_| PokerError::WrongArgCount {
        command: "players",
        usage: "<2-9>",
    })?;

    // Capture the current position (if any) before changing the table size,
    // so we can try to preserve it across the resize.
    let prev_position = state.position();

    state.num_players = n.clamp(2, 9);

    let position_preserved = match prev_position {
        Some(pos) if state.set_position(pos) => true,
        _ => {
            state.position_index = 0;
            false
        }
    };

    let positions = position::positions_for_table_size(state.num_players);
    let train: Vec<&str> = positions.iter().map(|p| p.short_name()).collect();

    let table_str = if state.configured {
        let pos = state.position().unwrap();
        format!("{}\n\n", table_display::render_table(state.num_players, Some(pos)))
    } else {
        format!("{}\n\n", table_display::render_table(state.num_players, None))
    };

    let mut output = format!(
        "{table_str}Players set to {}.\nPositions: {}\n",
        state.num_players,
        train.join(" -> ")
    );

    if state.configured {
        let pos = state.position().unwrap();
        let label = if position_preserved { "Position:" } else { "Position reset to" };
        output.push_str(&format!(
            "{label} {} ({}).",
            pos.short_name(),
            pos.long_name()
        ));

        if state.hole_cards.is_some() {
            output.push_str(&format!("\n\n{}", format_recommendation(state)));
        }
    }

    Ok(Some(output))
}

fn cmd_pos(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.is_empty() {
        return Err(PokerError::WrongArgCount {
            command: "pos",
            usage: "<position>",
        });
    }

    let position = Position::parse(args[0])?;

    let positions = position::positions_for_table_size(state.num_players);
    if !positions.contains(&position) {
        let valid: Vec<&str> = positions.iter().map(|p| p.short_name()).collect();
        return Ok(Some(format!(
            "Position {} is not valid for a {}-player table.\nValid positions: {}",
            position.short_name(),
            state.num_players,
            valid.join(", ")
        )));
    }

    state.set_position(position);
    state.configured = true;

    let table = table_display::render_table(state.num_players, Some(position));
    let mut output = format!(
        "{table}\n\nPosition set to {} ({}).",
        position.short_name(),
        position.long_name()
    );

    if state.hole_cards.is_some() {
        output.push_str(&format!("\n\n{}", format_recommendation(state)));
    }

    Ok(Some(output))
}

fn cmd_blinds(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.is_empty() {
        return match state.big_blind {
            Some(bb) => Ok(Some(format!("Big blind: {bb}"))),
            None => Err(PokerError::WrongArgCount {
                command: "blinds",
                usage: "<big blind amount> — e.g. blinds 20",
            }),
        };
    }

    let bb: u64 = args[0].parse().map_err(|_| PokerError::WrongArgCount {
        command: "blinds",
        usage: "<big blind amount> — e.g. blinds 20",
    })?;

    if bb == 0 {
        return Err(PokerError::WrongArgCount {
            command: "blinds",
            usage: "<big blind amount> — must be > 0",
        });
    }

    state.big_blind = Some(bb);
    Ok(Some(format!("Big blind set to {bb}. Sizing guidance will use this amount.")))
}

fn cmd_ranges() -> Result<Option<String>, PokerError> {
    let info = "\
About These Ranges
==================

Source: Based on Upswing Poker simplified GTO charts, tightened for
micro-stakes rake. 100bb stack depth, NL cash games.

How they work:
  RAISE     You're first in or facing a limp — raise to enter the pot.
  FOLD      This hand isn't profitable from this position.
  3-BET     Re-raise. Your hand is strong enough to put in a 3rd bet,
            or it's a good bluff candidate (like A5s — it blocks AA/AK
            and has decent equity if called).
  CALL      Facing a raise, your hand is good enough to see a flop
            but not strong enough to re-raise.

Caveats:
  - These are simplified. Real solvers use mixed strategies (e.g. open
    A9o from UTG 35% of the time). This tool says raise or fold.
  - 3-bet ranges here are polarized: premium hands for value + low
    suited aces (A5s-A2s) as bluffs. At micros, lean toward value
    since opponents call too much.
  - Facing a raise, the right play depends on WHO raised and from
    where. This tool gives a rough guide based on your position only.
  - At micros, players are generally too loose. You can often play
    tighter than these charts and still do well.";

    Ok(Some(info.to_string()))
}

fn cmd_help() -> Result<Option<String>, PokerError> {
    let help = "\
Poker CLI — Preflop Study Tool

Card entry (no command — inferred from current street):
  AhKs            Hole cards (preflop)
  AhKsr60         Hole cards facing a 60-chip raise   (l = limp, r = raise)
  2h3s4c          Flop
  5d              Turn
  6h              River
  n / new         Start a new hand (advances position)

Action override (re-evaluates current hand):
  limp            Treat as facing a limp
  raise [amount]  Treat as facing a raise
  first           Reset to first-in

Pot odds:
  odds                  Show outs, equity, and bet suggestion
  odds <bet> <pot>      Pot odds for a specific bet
  b<bet>p<pot>          Shorthand — e.g. b25p50

Setup:
  players <2-9>     Set number of players at the table
  pos <position>    Set your current position
  blinds <amount>   Set big blind for sizing guidance (e.g. blinds 20)
  ranges            Info about the ranges and how they work
  help              Show this help
  quit / exit       Exit the program

Card notation: rank + suit (e.g. As, Td, 2c, KH)
  Ranks: 2 3 4 5 6 7 8 9 T J Q K A  (10 also accepted for T)
  Suits: s h d c

Positions: utg, utg1, utg2, mp, hj, co, btn, sb, bb";

    Ok(Some(help.to_string()))
}

pub fn format_status(state: &HandState) -> String {
    let pos_name = state
        .position()
        .map(|p| p.short_name().to_string())
        .unwrap_or_else(|| "?".to_string());

    let blinds_str = match state.big_blind {
        Some(bb) => format!(" · BB {bb}"),
        None => String::new(),
    };
    let mut parts = vec![format!("[{pos_name} · {} players{blinds_str}]", state.num_players)];

    if let Some([c1, c2]) = &state.hole_cards {
        let hole_type = HoleCardType::from_cards(*c1, *c2);
        parts.push(format!("Hand: {c1} {c2} ({})", hole_type.label()));
    }

    if !state.board.is_empty() {
        let board_str = state
            .board
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        parts.push(format!("Board: {board_str}"));

        let hole = state.hole_cards.unwrap();
        let made = eval::evaluate(&hole, &state.board);

        if state.street != Street::River {
            let analysis = outs::analyze_outs(&hole, &state.board, state.street);
            if analysis.total_outs > 0 {
                parts.push(format!(
                    "{} · {} outs · ~{:.0}% equity",
                    made, analysis.total_outs, analysis.equity_percent
                ));
            } else {
                parts.push(format!("{} · no draws", made));
            }
        } else {
            parts.push(format!("{}", made));
        }
    }

    parts.join("  |  ")
}
