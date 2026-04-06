use colored::Colorize;

use crate::card::Card;
use crate::error::PokerError;
use crate::hand_state::{Action, HandState};
use crate::position::{self, Position};
use crate::preflop::{self, HoleCardType};
use crate::table_display;

pub fn execute(state: &mut HandState, input: &str) -> Result<Option<String>, PokerError> {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.is_empty() {
        return Ok(None);
    }

    let cmd = parts[0].to_lowercase();
    let args = &parts[1..];

    match cmd.as_str() {
        "new" => cmd_new(state),
        "deal" => cmd_deal(state, args),
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

Commands:
  deal <c1> <c2> [limp|raise]   Deal hole cards, get preflop recommendation
  limp                          Update to facing a limp (re-shows recommendation)
  raise                         Update to facing a raise (re-shows recommendation)
  first                         Reset to first-in (re-shows recommendation)
  new                           Advance position and reset for next hand
  players <2-9>                 Set number of players at the table
  pos <position>                Set your current position
  ranges                        Info about the ranges and how they work
  help                          Show this help
  quit / exit                   Exit the program

Card notation: rank + suit (e.g. As, Td, 2c, KH)
  Ranks: 2 3 4 5 6 7 8 9 T J Q K A  (10 also accepted for T)
  Suits: s h d c

Positions: utg, utg1, utg2, mp, hj, co, btn, sb, bb";

    Ok(Some(help.to_string()))
}
