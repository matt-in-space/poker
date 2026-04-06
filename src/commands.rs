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
    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.is_empty() {
        return Ok(None);
    }

    let cmd = parts[0].to_lowercase();
    let args = &parts[1..];

    // Check for compact shortcuts: d=deal, f=flop, t=turn, r=river, b=odds
    if cmd.len() > 1 {
        if let Some(rest) = cmd.strip_prefix('d') {
            if cmd != "deal" {
                return parse_compact_deal(state, rest);
            }
        }
        if let Some(rest) = cmd.strip_prefix('f') {
            if cmd != "first" && cmd != "firstin" && cmd != "flop" {
                return parse_compact_flop(state, rest);
            }
        }
        if let Some(rest) = cmd.strip_prefix('t') {
            if cmd != "turn" {
                return parse_compact_single_card(state, rest, Street::Turn);
            }
        }
        if let Some(rest) = cmd.strip_prefix('r') {
            if cmd != "raise" && cmd != "ranges" && cmd != "river" {
                return parse_compact_single_card(state, rest, Street::River);
            }
        }
        if let Some(rest) = cmd.strip_prefix('b') {
            if !rest.is_empty() {
                return parse_compact_odds(state, rest);
            }
        }
    }

    match cmd.as_str() {
        "n" | "new" => cmd_new(state),
        "deal" => cmd_deal(state, args),
        "flop" => cmd_flop(state, args),
        "turn" => cmd_turn(state, args),
        "river" => cmd_river(state, args),
        "odds" => cmd_odds(state, args),
        "limp" => cmd_action(state, Action::FacingLimp),
        "raise" => cmd_action(state, Action::FacingRaise),
        "first" | "firstin" => cmd_action(state, Action::FirstIn),
        "players" => cmd_players(state, args),
        "pos" => cmd_pos(state, args),
        "ranges" => cmd_ranges(),
        "help" => cmd_help(),
        "quit" | "exit" => std::process::exit(0),
        _ => Ok(Some(format!(
            "Unknown command: '{cmd}' — type 'help' for a list of commands"
        ))),
    }
}

// --- Compact shortcut parsers ---

/// Split a string into cards by finding suit characters (s, h, d, c).
fn find_suit_positions(chars: &[char]) -> Vec<usize> {
    let mut positions: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, 's' | 'h' | 'c'))
        .map(|(i, _)| i)
        .collect();

    // 'd' is both a suit (diamonds) and command prefix — only count if after a rank char
    for (i, &c) in chars.iter().enumerate() {
        if c == 'd' && i > 0 {
            positions.push(i);
        }
    }
    positions.sort();
    positions
}

/// Parse compact deal: "2h8s", "ThKs", "AhKsr" (trailing r=raise, l=limp)
fn parse_compact_deal(state: &mut HandState, input: &str) -> Result<Option<String>, PokerError> {
    let lower = input.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let suits = find_suit_positions(&chars);

    if suits.len() < 2 {
        return Err(PokerError::WrongArgCount {
            command: "d",
            usage: "<card1><card2> — e.g. d2h8s, dThKc",
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

    let action = match remainder.as_str() {
        "l" | "limp" => Action::FacingLimp,
        "r" | "raise" => Action::FacingRaise,
        "" => Action::FirstIn,
        _ => {
            return Err(PokerError::WrongArgCount {
                command: "d",
                usage: "<card1><card2>[l|r] — e.g. d2h8s, dThKcr",
            });
        }
    };

    if !state.configured {
        return Err(PokerError::NotConfigured);
    }

    state.hole_cards = Some([card1, card2]);
    state.action = action;

    Ok(Some(format_recommendation(state)))
}

/// Parse compact flop: "2h3s4c"
fn parse_compact_flop(state: &mut HandState, input: &str) -> Result<Option<String>, PokerError> {
    let lower = input.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let suits = find_suit_positions(&chars);

    if suits.len() < 3 {
        return Err(PokerError::WrongArgCount {
            command: "f",
            usage: "<c1><c2><c3> — e.g. f2h3s4c",
        });
    }

    let c1_str: String = chars[..=suits[0]].iter().collect();
    let c2_str: String = chars[suits[0] + 1..=suits[1]].iter().collect();
    let c3_str: String = chars[suits[1] + 1..=suits[2]].iter().collect();

    let cards = [
        Card::parse(&c1_str)?,
        Card::parse(&c2_str)?,
        Card::parse(&c3_str)?,
    ];

    do_flop(state, &cards)
}

/// Parse compact single card for turn/river: "5d", "Kh"
fn parse_compact_single_card(
    state: &mut HandState,
    input: &str,
    next_street: Street,
) -> Result<Option<String>, PokerError> {
    let card = Card::parse(input)?;

    match next_street {
        Street::Turn => do_turn(state, card),
        Street::River => do_river(state, card),
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
    Ok(Some(format!(
        "Position advanced to {} ({}).",
        pos.short_name(),
        pos.long_name()
    )))
}

fn cmd_deal(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if !state.configured {
        return Err(PokerError::NotConfigured);
    }

    if args.len() < 2 || args.len() > 3 {
        return Err(PokerError::WrongArgCount {
            command: "deal",
            usage: "<card1> <card2> [limp|raise]",
        });
    }

    let card1 = Card::parse(args[0])?;
    let card2 = Card::parse(args[1])?;

    if card1 == card2 {
        return Err(PokerError::DuplicateCard(card1));
    }

    let action = if let Some(action_str) = args.get(2) {
        match action_str.to_lowercase().as_str() {
            "limp" => Action::FacingLimp,
            "raise" => Action::FacingRaise,
            _ => {
                return Err(PokerError::WrongArgCount {
                    command: "deal",
                    usage: "<card1> <card2> [limp|raise]",
                });
            }
        }
    } else {
        Action::FirstIn
    };

    state.hole_cards = Some([card1, card2]);
    state.action = action;

    Ok(Some(format_recommendation(state)))
}

fn cmd_flop(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.len() != 3 {
        return Err(PokerError::WrongArgCount {
            command: "flop",
            usage: "<card1> <card2> <card3>",
        });
    }

    let cards = [
        Card::parse(args[0])?,
        Card::parse(args[1])?,
        Card::parse(args[2])?,
    ];

    do_flop(state, &cards)
}

fn cmd_turn(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.len() != 1 {
        return Err(PokerError::WrongArgCount {
            command: "turn",
            usage: "<card>",
        });
    }
    let card = Card::parse(args[0])?;
    do_turn(state, card)
}

fn cmd_river(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.len() != 1 {
        return Err(PokerError::WrongArgCount {
            command: "river",
            usage: "<card>",
        });
    }
    let card = Card::parse(args[0])?;
    do_river(state, card)
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
            expected: "Flop already set — use 'turn' or 't' for the next card",
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
            expected: "Use 'flop' or 'f' first",
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
            expected: "Use 'turn' or 't' first",
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
    let mut output = format!(
        "Pot odds: call ${bet} into ${pot} → need {:.1}% equity",
        odds.required_equity
    );

    if let Some(hole) = &state.hole_cards {
        if !state.board.is_empty() && state.street != Street::River {
            let analysis = outs::analyze_outs(hole, &state.board, state.street);
            let eq = analysis.equity_percent;

            if analysis.total_outs > 0 {
                output.push_str(&format!(
                    "\nYou have: ~{:.0}% equity ({} outs)",
                    eq, analysis.total_outs
                ));
            } else {
                output.push_str("\nYou have: ~0% draw equity (no outs)");
            }

            if eq >= odds.required_equity {
                output.push_str(&format!(
                    "\nVerdict: {}",
                    "CALL — profitable".green().bold()
                ));
            } else {
                output.push_str(&format!(
                    "\nVerdict: {}",
                    "FOLD — not enough equity".red()
                ));
            }
        } else if state.board.is_empty() {
            output.push_str("\n(Deal a flop to compare against your equity)");
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

fn cmd_action(state: &mut HandState, action: Action) -> Result<Option<String>, PokerError> {
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
    Ok(Some(format_recommendation(state)))
}

pub fn format_recommendation(state: &HandState) -> String {
    let [card1, card2] = state.hole_cards.unwrap();
    let position = state.position().unwrap();

    let hole_type = HoleCardType::from_cards(card1, card2);
    let rec = preflop::recommend(&hole_type, position, state.num_players, state.action);

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
    };

    let action_label = match state.action {
        Action::FirstIn => "",
        Action::FacingLimp => "  (facing limp)",
        Action::FacingRaise => "  (facing raise)",
    };

    let table = table_display::render_table(state.num_players, Some(position));

    format!(
        "{table}\n\n\
         Hand: {card1} {card2}  ({label} — {category})\n\
         Position: {pos} ({long}){action_label}\n\
         Recommendation: {rec}\n\
         {desc}",
        label = hole_type.label(),
        category = hole_type.category(),
        pos = position.short_name(),
        long = position.long_name(),
        rec = rec_colored,
        desc = rec_desc,
    )
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

    state.num_players = n.clamp(2, 9);
    state.position_index = 0;

    let positions = position::positions_for_table_size(state.num_players);
    let train: Vec<&str> = positions.iter().map(|p| p.short_name()).collect();

    let mut output = format!(
        "Players set to {}.\nPositions: {}\n",
        state.num_players,
        train.join(" -> ")
    );

    if state.configured {
        let pos = state.position().unwrap();
        output.push_str(&format!(
            "Position reset to {} ({}).",
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

    let mut output = format!(
        "Position set to {} ({}).",
        position.short_name(),
        position.long_name()
    );

    if state.hole_cards.is_some() {
        output.push_str(&format!("\n\n{}", format_recommendation(state)));
    }

    Ok(Some(output))
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

Commands:                                Shortcuts:
  deal <c1> <c2> [limp|raise]              d<c1><c2>[l|r]  e.g. d2h8s, dThKcr
  flop <c1> <c2> <c3>                      f<c1><c2><c3>   e.g. f2h3s4c
  turn <card>                              t<card>         e.g. t5d
  river <card>                             r<card>         e.g. r6h
  new                                      n
  odds                                     Show outs, equity, and bet suggestion
  odds <bet> <pot>                         b<bet>p<pot>    e.g. b25p50
  limp                 Facing a limp (re-shows recommendation)
  raise                Facing a raise (re-shows recommendation)
  first                Reset to first-in (re-shows recommendation)
  players <2-9>        Set number of players at the table
  pos <position>       Set your current position
  ranges               Info about the ranges and how they work
  help                 Show this help
  quit / exit          Exit the program

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

    let mut parts = vec![format!("[{pos_name} · {} players]", state.num_players)];

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
