use colored::Colorize;

use crate::card::Card;
use crate::error::PokerError;
use crate::eval;
use crate::hand_state::{Action, ActionEntry, HandState, Street};
use crate::outs;
use crate::position::Position;
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

    match cmd.as_str() {
        "new" => cmd_new(state),
        "deal" => cmd_deal(state, args),
        "flop" => cmd_flop(state, args),
        "turn" => cmd_turn(state, args),
        "river" => cmd_river(state, args),
        "pot" => cmd_pot(state, args),
        "table" => cmd_table(state, args),
        "play" => cmd_play(state, args),
        "pos" => cmd_pos(state, args),
        "help" => cmd_help(),
        "guide" => cmd_guide(args),
        "quit" | "exit" => std::process::exit(0),
        _ => Ok(Some(format!("Unknown command: '{cmd}' — type 'help' for a list of commands"))),
    }
}

fn cmd_new(state: &mut HandState) -> Result<Option<String>, PokerError> {
    state.reset();
    Ok(Some("Hand reset. Ready for a new hand.".to_string()))
}

fn cmd_deal(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.len() < 3 {
        return Err(PokerError::WrongArgCount {
            command: "deal",
            usage: "<card1> <card2> <position> [players]",
        });
    }

    let card1 = Card::parse(args[0])?;
    let card2 = Card::parse(args[1])?;

    if card1 == card2 {
        return Err(PokerError::DuplicateCard(card1));
    }

    let position = Position::parse(args[2])?;

    if args.len() >= 4 {
        if let Ok(n) = args[3].parse::<u8>() {
            state.num_players = n.clamp(2, 9);
        }
    }

    state.reset();
    state.hole_cards = Some([card1, card2]);
    state.position = Some(position);

    let hole_type = HoleCardType::from_cards(card1, card2);
    let rec = preflop::recommend(&hole_type, position, state.num_players);

    let rec_colored = match rec {
        preflop::Recommendation::Open => "OPEN".green().bold(),
        preflop::Recommendation::Fold => "FOLD".red().bold(),
        preflop::Recommendation::ThreeBet => "3-BET".yellow().bold(),
    };

    let table = table_display::render_table(state.num_players, Some(position));

    let output = format!(
        "{table}\n\n\
         Hand: {card1} {card2}  ({label} — {category})\n\
         Position: {pos} ({long})\n\
         Recommendation: {rec}",
        label = hole_type.label(),
        category = hole_type.category(),
        pos = position.short_name(),
        long = position.long_name(),
        rec = rec_colored,
    );

    Ok(Some(output))
}

fn cmd_flop(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if state.hole_cards.is_none() {
        return Err(PokerError::NoDeal);
    }
    if state.street != Street::Preflop {
        return Err(PokerError::WrongStreet {
            expected: "Flop already set — use 'turn' for the next card",
        });
    }
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
    state.check_duplicates(&cards)?;

    state.board.extend_from_slice(&cards);
    state.street = Street::Flop;

    let hole = state.hole_cards.unwrap();
    let made = eval::evaluate(&hole, &state.board);
    let analysis = outs::analyze_outs(&hole, &state.board, Street::Flop);

    let mut output = format!(
        "Board: {}\n",
        state.board.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
    );

    output.push_str(&format!("Made hand: {}\n", made.to_string().bold()));

    if analysis.draws.is_empty() {
        output.push_str("No draws.\n");
    } else {
        output.push('\n');
        for draw in &analysis.draws {
            let outs_str = draw.outs.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ");
            output.push_str(&format!(
                "  {} ({} outs): {}\n",
                draw.draw_type,
                draw.outs.len(),
                outs_str
            ));
        }
        output.push_str(&format!(
            "\nTotal outs: {} (~{:.0}% equity, rule of 4)\n",
            analysis.total_outs, analysis.equity_percent
        ));
    }

    Ok(Some(output))
}

fn cmd_turn(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if state.hole_cards.is_none() {
        return Err(PokerError::NoDeal);
    }
    if state.street != Street::Flop {
        return Err(PokerError::WrongStreet {
            expected: "No flop set yet — use 'flop' first",
        });
    }
    if args.len() != 1 {
        return Err(PokerError::WrongArgCount {
            command: "turn",
            usage: "<card>",
        });
    }

    let card = Card::parse(args[0])?;
    state.check_not_duplicate(card)?;

    state.board.push(card);
    state.street = Street::Turn;

    let hole = state.hole_cards.unwrap();
    let made = eval::evaluate(&hole, &state.board);
    let analysis = outs::analyze_outs(&hole, &state.board, Street::Turn);

    let mut output = format!(
        "Board: {}\n",
        state.board.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
    );

    output.push_str(&format!("Made hand: {}\n", made.to_string().bold()));

    if analysis.draws.is_empty() {
        output.push_str("No draws.\n");
    } else {
        output.push('\n');
        for draw in &analysis.draws {
            let outs_str = draw.outs.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ");
            output.push_str(&format!(
                "  {} ({} outs): {}\n",
                draw.draw_type,
                draw.outs.len(),
                outs_str
            ));
        }
        output.push_str(&format!(
            "\nTotal outs: {} (~{:.0}% equity, rule of 2)\n",
            analysis.total_outs, analysis.equity_percent
        ));
    }

    Ok(Some(output))
}

fn cmd_river(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if state.hole_cards.is_none() {
        return Err(PokerError::NoDeal);
    }
    if state.street != Street::Turn {
        return Err(PokerError::WrongStreet {
            expected: "No turn set yet — use 'turn' first",
        });
    }
    if args.len() != 1 {
        return Err(PokerError::WrongArgCount {
            command: "river",
            usage: "<card>",
        });
    }

    let card = Card::parse(args[0])?;
    state.check_not_duplicate(card)?;

    state.board.push(card);
    state.street = Street::River;

    let hole = state.hole_cards.unwrap();
    let made = eval::evaluate(&hole, &state.board);

    let output = format!(
        "Board: {}\n\
         Final hand: {}",
        state.board.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" "),
        made.to_string().green().bold(),
    );

    Ok(Some(output))
}

fn cmd_pot(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    match args.len() {
        1 => {
            // pot <bet> — use tracked pot
            let bet: u64 = args[0]
                .parse()
                .map_err(|_| PokerError::WrongArgCount {
                    command: "pot",
                    usage: "<pot_size> <bet_size>  or  <bet_size> (if pot is tracked)",
                })?;
            if state.pot == 0 {
                return Err(PokerError::WrongArgCount {
                    command: "pot",
                    usage: "<pot_size> <bet_size>  (no tracked pot — provide both values)",
                });
            }
            let odds = PotOdds::calculate(state.pot, bet);
            let mut output = format!("{odds}");
            output.push_str(&equity_comparison(state, &odds));
            Ok(Some(output))
        }
        2 => {
            let pot_size: u64 = args[0]
                .parse()
                .map_err(|_| PokerError::WrongArgCount {
                    command: "pot",
                    usage: "<pot_size> <bet_size>",
                })?;
            let bet_size: u64 = args[1]
                .parse()
                .map_err(|_| PokerError::WrongArgCount {
                    command: "pot",
                    usage: "<pot_size> <bet_size>",
                })?;
            let odds = PotOdds::calculate(pot_size, bet_size);
            let mut output = format!("{odds}");
            output.push_str(&equity_comparison(state, &odds));
            Ok(Some(output))
        }
        _ => Err(PokerError::WrongArgCount {
            command: "pot",
            usage: "<pot_size> <bet_size>",
        }),
    }
}

fn equity_comparison(state: &HandState, odds: &PotOdds) -> String {
    if let Some(hole) = &state.hole_cards {
        if !state.board.is_empty() && state.street != Street::River {
            let analysis = outs::analyze_outs(hole, &state.board, state.street);
            if analysis.total_outs > 0 {
                let eq = analysis.equity_percent;
                let needed = odds.required_equity;
                if eq >= needed {
                    return format!(
                        "\n{} (~{:.0}% equity vs {:.1}% needed)",
                        "Profitable call".green().bold(),
                        eq,
                        needed
                    );
                } else {
                    return format!(
                        "\n{} (~{:.0}% equity vs {:.1}% needed)",
                        "Unprofitable — fold or look for other factors".red(),
                        eq,
                        needed
                    );
                }
            }
        }
    }
    String::new()
}

fn cmd_table(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if let Some(n_str) = args.first() {
        if let Ok(n) = n_str.parse::<u8>() {
            state.num_players = n.clamp(2, 9);
        }
    }

    if let Some(pos_str) = args.get(1) {
        state.position = Some(Position::parse(pos_str)?);
    }

    let table = table_display::render_table(state.num_players, state.position);
    Ok(Some(table))
}

fn cmd_play(state: &mut HandState, args: &[&str]) -> Result<Option<String>, PokerError> {
    if args.len() < 2 {
        return Err(PokerError::WrongArgCount {
            command: "play",
            usage: "<position> <action> [amount]",
        });
    }

    let position = Position::parse(args[0])?;
    let action = match args[1].to_lowercase().as_str() {
        "fold" => Action::Fold,
        "check" => Action::Check,
        "call" => Action::Call,
        "raise" => {
            if args.len() < 3 {
                return Err(PokerError::WrongArgCount {
                    command: "play",
                    usage: "<position> raise <amount>",
                });
            }
            let amt: u64 = args[2].parse().map_err(|_| PokerError::WrongArgCount {
                command: "play",
                usage: "<position> raise <amount>",
            })?;
            Action::Raise(amt)
        }
        "allin" | "all-in" => Action::AllIn,
        other => {
            return Ok(Some(format!(
                "Unknown action '{other}' — use fold, check, call, raise <amount>, or allin"
            )));
        }
    };

    // Update pot for calls and raises
    match action {
        Action::Call => {
            // Without bet tracking we can't auto-add, but note it
        }
        Action::Raise(amt) => {
            state.pot += amt;
        }
        _ => {}
    }

    let entry = ActionEntry {
        position,
        action,
        street: state.street,
    };
    state.actions.push(entry);

    // Show action log for current street
    let street_actions = state.actions_on_street(state.street);
    let mut output = String::new();
    for a in &street_actions {
        output.push_str(&format!("  {} {}\n", a.position, a.action));
    }
    output.push_str(&format!("Pot: ${}", state.pot));

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
    state.position = Some(position);
    Ok(Some(format!("Position updated to {} ({})", position.short_name(), position.long_name())))
}

fn cmd_help() -> Result<Option<String>, PokerError> {
    let help = "\
Poker CLI — Texas Hold'em Study Tool

Commands:
  new                              Reset hand state for a new hand
  deal <c1> <c2> <pos> [players]   Deal hole cards, set position
  flop <c1> <c2> <c3>             Set flop cards
  turn <card>                      Set turn card
  river <card>                     Set river card
  pot <pot> <bet>                  Calculate pot odds
  pot <bet>                        Pot odds using tracked pot
  table [players] [position]       Show/update table layout
  play <pos> <action> [amount]     Record a player action
  pos <position>                   Update your position
  help                             Show this help
  guide [topic]                    Poker concepts guide
  quit / exit                      Exit the program

Card notation: rank + suit (e.g. As, Td, 2c, KH)
  Ranks: 2 3 4 5 6 7 8 9 T J Q K A  (10 also accepted for T)
  Suits: s h d c

Positions: utg, utg1, utg2, mp, hj, co, btn, sb, bb";

    Ok(Some(help.to_string()))
}

fn cmd_guide(args: &[&str]) -> Result<Option<String>, PokerError> {
    let topic = args.first().map(|s| s.to_lowercase());

    match topic.as_deref() {
        Some("streets") => Ok(Some(GUIDE_STREETS.to_string())),
        Some("positions") | Some("position") | Some("pos") => Ok(Some(GUIDE_POSITIONS.to_string())),
        Some("preflop") | Some("recommendations") | Some("rec") => Ok(Some(GUIDE_PREFLOP.to_string())),
        Some("hands") | Some("categories") | Some("hand") => Ok(Some(GUIDE_HANDS.to_string())),
        Some("outs") | Some("equity") | Some("draws") => Ok(Some(GUIDE_OUTS.to_string())),
        Some("pot") | Some("odds") | Some("potodds") => Ok(Some(GUIDE_POT_ODDS.to_string())),
        Some("made") | Some("strength") | Some("eval") => Ok(Some(GUIDE_MADE_HANDS.to_string())),
        Some(other) => Ok(Some(format!(
            "Unknown topic '{other}'.\n\n{GUIDE_TOPICS}"
        ))),
        None => Ok(Some(GUIDE_OVERVIEW.to_string())),
    }
}

const GUIDE_TOPICS: &str = "\
Topics:  guide streets     — The four stages of a hand
         guide positions   — Table positions and why they matter
         guide preflop     — What OPEN, FOLD, and 3-BET mean
         guide hands       — Hand categories (suited connector, etc.)
         guide outs        — Outs, draws, and equity estimation
         guide pot         — Pot odds and when to call
         guide made        — Made hand rankings (pair through flush)";

const GUIDE_OVERVIEW: &str = "\
Poker Concepts Guide
====================

This tool helps you study Texas Hold'em by giving real-time analysis
as a hand plays out. Here's what it covers:

  Streets      The four stages of a hand (preflop, flop, turn, river)
  Positions    Where you sit and why it matters
  Preflop      What OPEN, FOLD, and 3-BET mean
  Hands        Hand categories like suited connector, offsuit broadway
  Outs         Counting outs and estimating equity
  Pot odds     When calling is profitable
  Made hands   Hand rankings from high card to straight flush

Type 'guide <topic>' for details on any topic. Example: guide outs";

const GUIDE_STREETS: &str = "\
Streets
=======

A Texas Hold'em hand has four stages, called streets:

1. Preflop   Each player gets two private cards (hole cards).
              A round of betting follows.

2. Flop      Three community cards are dealt face-up.
              Another round of betting.

3. Turn      A fourth community card is dealt.
              Another round of betting.

4. River     A fifth and final community card.
              Final round of betting, then showdown.

Your best five-card hand is made from any combination of your
two hole cards and the five community cards on the board.

In this tool, the prompt changes to show the current street
(preflop>, flop>, turn>, river>).";

const GUIDE_POSITIONS: &str = "\
Positions
=========

Position is where you sit relative to the dealer button. It determines
when you act in a betting round. Acting later is a huge advantage
because you see what others do first.

From earliest (worst) to latest (best):

  UTG  (Under the Gun)  First to act preflop. Play very tight.
  UTG+1, UTG+2          Still early. Play tight.
  MP   (Middle)         Can loosen up slightly.
  HJ   (Hijack)         Two before the button. Ranges open up.
  CO   (Cutoff)         One before the button. Very good position.
  BTN  (Button)         Best position. Acts last postflop. Widest range.
  SB   (Small Blind)    Posts a forced half-bet. Acts first postflop.
  BB   (Big Blind)      Posts a forced full bet. Gets a discount to see flops.

At smaller tables (e.g. 6-max), early positions are removed but the
relative advantage of later positions stays the same. The tool adjusts
automatically when you set the player count.";

const GUIDE_PREFLOP: &str = "\
Preflop Recommendations
=======================

When you 'deal', the tool gives one of three recommendations:

  OPEN   Raise to enter the pot. The standard play when no one
         has raised before you.

  FOLD   Don't play this hand from this position. It's not
         profitable long-term.

  3-BET  Re-raise. The blinds are the 1st bet, an open-raise
         is the 2nd bet, and a re-raise is the 3rd bet (hence
         '3-bet'). Reserved for premium hands.

These are based on GTO-approximate opening ranges — what game theory
suggests you should play from each position as a default. They assume
you're first to voluntarily enter the pot.

The same hand can be an OPEN in late position but a FOLD from early
position. For example, 87s is a clear open on the button but a fold
from UTG at a 9-handed table.

Note: These ranges are for No-Limit Hold'em cash games with ~100bb
stacks. They may not apply to other formats:

  Limit Hold'em     Ranges are wider — you can't be forced off a
                    hand with a large bet.
  Tournaments       Stack depth, blind levels, and ICM pressure all
                    shift ranges. Short-stacked play is very different
                    (push/fold).
  Deep-stacked      200bb+ — speculative hands (suited connectors)
                    go up in value; dominated hands go down.";

const GUIDE_HANDS: &str = "\
Hand Categories
===============

The tool classifies your two hole cards to help build vocabulary:

  Pocket pair        Same rank            (77, KK)
  Suited broadway    Both T+, same suit   (AKs, QJs)
  Offsuit broadway   Both T+, diff suits  (AKo, KJo)
  Suited ace         Ace + lower, suited  (A5s, A9s)
  Suited connector   Consecutive, suited  (87s, JTs)
  Suited one-gapper  One rank gap, suited (T8s, 97s)
  Offsuit connector  Consecutive, diff    (87o, JTo)

The 's' suffix means suited (same suit).
The 'o' suffix means offsuit (different suits).

Suited hands are stronger because they can make flushes.
Connected hands are stronger because they can make straights.
Broadway cards (T through A) make the highest pairs.";

const GUIDE_OUTS: &str = "\
Outs and Equity
===============

An 'out' is a card that will improve your hand. The tool detects:

  Flush draw      4 cards of one suit, need one more.   9 outs
  OESD            Open-ended straight draw.             8 outs
  Gutshot         Straight draw with an inside gap.     4 outs
  Overcards       Hole cards above all board cards.     3 outs each

Equity is your chance of winning. Estimated with the Rule of 2 and 4:

  Flop (2 cards to come):  outs x 4
  Turn (1 card to come):   outs x 2

Example: Flush draw on the flop = 9 x 4 = ~36% equity.

This is an approximation — slightly optimistic for high out counts
but accurate enough for in-game decisions.

When draws overlap (e.g. a card completes both a flush and a straight),
the tool lists each draw separately but deduplicates the total so no
card is counted twice.

  Combo draw example: flush draw (9) + straight draw (8)
  Some cards overlap, so total might be 15 instead of 17.";

const GUIDE_POT_ODDS: &str = "\
Pot Odds
========

Pot odds tell you the minimum equity needed to profitably call a bet:

  required equity = bet / (pot + bet)

Example: Someone bets $50 into a $200 pot.
  50 / (200 + 50) = 50 / 250 = 20%
  You need at least 20% equity to break even.

If your estimated equity exceeds this, calling is profitable long-term.
If not, folding is usually correct.

The 'pot' command does this math for you:
  pot 200 50       Calculate from manual values
  pot 50           Use the tracked pot from 'play' commands

When a hand is in progress, it also compares against your current
equity estimate and tells you whether the call is profitable.

Note: This is 'direct' pot odds only. Implied odds (money you might
win on later streets) can make some calls profitable even when direct
pot odds say fold — but that requires judgment the tool can't provide.";

const GUIDE_MADE_HANDS: &str = "\
Made Hand Rankings
==================

From weakest to strongest:

  High card           No pair or better
  Pair                One pair — tool shows top/overpair/middle/bottom
  Two pair            Two different pairs
  Three of a kind     Set (pocket pair hit) or trips (board pair hit)
  Straight            Five consecutive ranks
  Flush               Five cards of one suit
  Full house          Three of a kind + a pair
  Four of a kind      Four of the same rank
  Straight flush      Five consecutive cards of one suit
  Royal flush         A-K-Q-J-T of one suit (the best hand)

A 'set' (pocket pair + board card) is much stronger than 'trips'
(one hole card + board pair) because it's better disguised —
opponents are less likely to put you on three of a kind.

Top pair means one of your hole cards matches the highest board card.
Overpair means your pocket pair is above all board cards.";
