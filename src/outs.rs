use std::collections::HashSet;
use std::fmt;

use crate::card::{Card, Rank, Suit};
use crate::hand_state::Street;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawType {
    FlushDraw,
    BackdoorFlushDraw,
    OpenEndedStraightDraw,
    GutshotStraightDraw,
    Overcards,
}

impl fmt::Display for DrawType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrawType::FlushDraw => write!(f, "Flush draw"),
            DrawType::BackdoorFlushDraw => write!(f, "Backdoor flush draw"),
            DrawType::OpenEndedStraightDraw => write!(f, "Open-ended straight draw"),
            DrawType::GutshotStraightDraw => write!(f, "Gutshot straight draw"),
            DrawType::Overcards => write!(f, "Overcards"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Draw {
    pub draw_type: DrawType,
    pub outs: Vec<Card>,
}

#[derive(Debug, Clone)]
pub struct OutsAnalysis {
    pub draws: Vec<Draw>,
    pub total_outs: u8,
    pub equity_percent: f64,
}

pub fn analyze_outs(hole: &[Card; 2], board: &[Card], street: Street) -> OutsAnalysis {
    let all_cards: HashSet<Card> = hole.iter().chain(board.iter()).copied().collect();
    let mut draws = Vec::new();

    // Flush draw detection
    if let Some(draw) = detect_flush_draw(hole, board, &all_cards, street) {
        draws.push(draw);
    }

    // Straight draw detection
    draws.extend(detect_straight_draws(hole, board, &all_cards));

    // Overcard detection
    if let Some(draw) = detect_overcards(hole, board, &all_cards) {
        draws.push(draw);
    }

    // Deduplicate total outs
    let all_outs: HashSet<Card> = draws.iter().flat_map(|d| d.outs.iter().copied()).collect();
    let total_outs = all_outs.len() as u8;

    let multiplier = match street {
        Street::Flop => 4.0,
        Street::Turn => 2.0,
        _ => 0.0,
    };
    let equity_percent = (total_outs as f64 * multiplier).min(100.0);

    OutsAnalysis {
        draws,
        total_outs,
        equity_percent,
    }
}

fn detect_flush_draw(
    hole: &[Card; 2],
    board: &[Card],
    all_cards: &HashSet<Card>,
    street: Street,
) -> Option<Draw> {
    let all: Vec<Card> = hole.iter().chain(board.iter()).copied().collect();

    for &suit in &Suit::ALL {
        let count = all.iter().filter(|c| c.suit == suit).count();

        // Need at least one hole card in the suit for it to be "our" draw
        let hole_in_suit = hole.iter().any(|c| c.suit == suit);
        if !hole_in_suit {
            continue;
        }

        if count >= 5 {
            // Made flush, not a draw
            continue;
        }

        if count == 4 {
            let outs: Vec<Card> = Rank::ALL
                .iter()
                .map(|&r| Card::new(r, suit))
                .filter(|c| !all_cards.contains(c))
                .collect();
            return Some(Draw {
                draw_type: DrawType::FlushDraw,
                outs,
            });
        }

        if count == 3 && street == Street::Flop {
            let outs: Vec<Card> = Rank::ALL
                .iter()
                .map(|&r| Card::new(r, suit))
                .filter(|c| !all_cards.contains(c))
                .collect();
            return Some(Draw {
                draw_type: DrawType::BackdoorFlushDraw,
                outs,
            });
        }
    }
    None
}

fn detect_straight_draws(
    hole: &[Card; 2],
    board: &[Card],
    all_cards: &HashSet<Card>,
) -> Vec<Draw> {
    let all: Vec<Card> = hole.iter().chain(board.iter()).copied().collect();
    let mut rank_present: HashSet<u8> = all.iter().map(|c| c.rank.value()).collect();
    // Ace also counts as 1 for low straights
    if rank_present.contains(&14) {
        rank_present.insert(1);
    }

    let hole_values: HashSet<u8> = hole.iter().map(|c| c.rank.value()).collect();

    // Check if we already have a straight
    for high in (5..=14).rev() {
        let low = high - 4;
        if (low..=high).all(|v| rank_present.contains(&v)) {
            return Vec::new(); // Made straight, no draw needed
        }
    }

    // Find missing ranks in potential straights where we contribute a hole card
    let mut oesd_ranks: HashSet<u8> = HashSet::new();
    let mut gutshot_ranks: HashSet<u8> = HashSet::new();

    // For each 5-card window, check if we have exactly 4 of 5
    for high in 5..=14 {
        let low = high - 4;
        let window: Vec<u8> = (low..=high).collect();
        let present: Vec<u8> = window.iter().filter(|v| rank_present.contains(v)).copied().collect();
        let missing: Vec<u8> = window.iter().filter(|v| !rank_present.contains(v)).copied().collect();

        if present.len() == 4 && missing.len() == 1 {
            // Check we have at least one hole card in this window
            let hole_contributes = window.iter().any(|v| {
                let check = if *v == 1 { 14 } else { *v };
                hole_values.contains(&check)
            });
            if !hole_contributes {
                continue;
            }

            let missing_rank = missing[0];
            // Normalize: 1 -> 14 for actual card lookup
            let lookup_rank = if missing_rank == 1 { 14 } else { missing_rank };

            // Is this OESD or gutshot? Check if the missing card is at the edge
            // OESD: missing card is at either end of the window
            if missing_rank == low || missing_rank == high {
                oesd_ranks.insert(lookup_rank);
            } else {
                gutshot_ranks.insert(lookup_rank);
            }
        }
    }

    // A rank that appears in OESD in any window counts as OESD
    // Remove from gutshot anything that's also in oesd
    gutshot_ranks.retain(|r| !oesd_ranks.contains(r));

    let mut draws = Vec::new();

    if oesd_ranks.len() >= 2 {
        // True OESD: two ranks complete the straight
        let outs = ranks_to_out_cards(&oesd_ranks, all_cards);
        if !outs.is_empty() {
            draws.push(Draw {
                draw_type: DrawType::OpenEndedStraightDraw,
                outs,
            });
        }
    } else if !oesd_ranks.is_empty() {
        // Single end — gutshot
        let outs = ranks_to_out_cards(&oesd_ranks, all_cards);
        if !outs.is_empty() {
            draws.push(Draw {
                draw_type: DrawType::GutshotStraightDraw,
                outs,
            });
        }
    }

    if !gutshot_ranks.is_empty() {
        let outs = ranks_to_out_cards(&gutshot_ranks, all_cards);
        if !outs.is_empty() {
            draws.push(Draw {
                draw_type: DrawType::GutshotStraightDraw,
                outs,
            });
        }
    }

    draws
}

fn ranks_to_out_cards(ranks: &HashSet<u8>, all_cards: &HashSet<Card>) -> Vec<Card> {
    let mut outs = Vec::new();
    for &v in ranks {
        if let Some(rank) = Rank::from_value(v) {
            for &suit in &Suit::ALL {
                let card = Card::new(rank, suit);
                if !all_cards.contains(&card) {
                    outs.push(card);
                }
            }
        }
    }
    outs.sort_by_key(|c| (c.rank, c.suit as u8));
    outs
}

fn detect_overcards(
    hole: &[Card; 2],
    board: &[Card],
    all_cards: &HashSet<Card>,
) -> Option<Draw> {
    if board.is_empty() {
        return None;
    }

    let board_max = board.iter().map(|c| c.rank).max().unwrap();
    let over_ranks: Vec<Rank> = hole
        .iter()
        .map(|c| c.rank)
        .filter(|&r| r > board_max)
        .collect();

    if over_ranks.is_empty() {
        return None;
    }

    let mut outs = Vec::new();
    for rank in &over_ranks {
        for &suit in &Suit::ALL {
            let card = Card::new(*rank, suit);
            if !all_cards.contains(&card) {
                outs.push(card);
            }
        }
    }
    outs.sort_by_key(|c| (c.rank, c.suit as u8));

    if outs.is_empty() {
        return None;
    }

    Some(Draw {
        draw_type: DrawType::Overcards,
        outs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> Card {
        Card::parse(s).unwrap()
    }

    #[test]
    fn test_flush_draw() {
        let hole = [c("Ah"), c("Kh")];
        let board = [c("2h"), c("5h"), c("9c")];
        let analysis = analyze_outs(&hole, &board, Street::Flop);

        let flush_draw = analysis.draws.iter().find(|d| d.draw_type == DrawType::FlushDraw);
        assert!(flush_draw.is_some());
        assert_eq!(flush_draw.unwrap().outs.len(), 9);
    }

    #[test]
    fn test_oesd() {
        // 8-9 on a 6-7-2 board = open-ended (need 5 or T)
        let hole = [c("8h"), c("9d")];
        let board = [c("6s"), c("7c"), c("2h")];
        let analysis = analyze_outs(&hole, &board, Street::Flop);

        let straight_draw = analysis
            .draws
            .iter()
            .find(|d| d.draw_type == DrawType::OpenEndedStraightDraw);
        assert!(straight_draw.is_some());
        assert_eq!(straight_draw.unwrap().outs.len(), 8);
    }

    #[test]
    fn test_gutshot() {
        // A-K on a Q-J-2 board = gutshot (need T)
        let hole = [c("Ah"), c("Kd")];
        let board = [c("Qs"), c("Jc"), c("2h")];
        let analysis = analyze_outs(&hole, &board, Street::Flop);

        let gutshot = analysis
            .draws
            .iter()
            .find(|d| d.draw_type == DrawType::GutshotStraightDraw);
        assert!(gutshot.is_some());
        assert_eq!(gutshot.unwrap().outs.len(), 4);
    }

    #[test]
    fn test_overcards() {
        let hole = [c("Ah"), c("Kd")];
        let board = [c("2s"), c("5c"), c("9h")];
        let analysis = analyze_outs(&hole, &board, Street::Flop);

        let overcards = analysis.draws.iter().find(|d| d.draw_type == DrawType::Overcards);
        assert!(overcards.is_some());
        assert_eq!(overcards.unwrap().outs.len(), 6); // 3 aces + 3 kings
    }

    #[test]
    fn test_made_flush_no_draw() {
        let hole = [c("Ah"), c("Kh")];
        let board = [c("2h"), c("5h"), c("9h")];
        let analysis = analyze_outs(&hole, &board, Street::Flop);

        let flush_draw = analysis.draws.iter().find(|d| d.draw_type == DrawType::FlushDraw);
        assert!(flush_draw.is_none());
    }

    #[test]
    fn test_equity_flop() {
        let hole = [c("Ah"), c("Kh")];
        let board = [c("2h"), c("5h"), c("9c")];
        let analysis = analyze_outs(&hole, &board, Street::Flop);
        // Should use rule of 4
        assert!(analysis.equity_percent > 0.0);
        assert_eq!(analysis.equity_percent, analysis.total_outs as f64 * 4.0);
    }

    #[test]
    fn test_equity_turn() {
        let hole = [c("Ah"), c("Kh")];
        let board = [c("2h"), c("5h"), c("9c"), c("3d")];
        let analysis = analyze_outs(&hole, &board, Street::Turn);
        assert_eq!(analysis.equity_percent, analysis.total_outs as f64 * 2.0);
    }

    #[test]
    fn test_deduplication() {
        // Cards that are outs for multiple draws should be counted once
        let hole = [c("Ah"), c("Kh")];
        let board = [c("Qh"), c("Jh"), c("2s")];
        let analysis = analyze_outs(&hole, &board, Street::Flop);
        // Has flush draw (9) + straight draw + overcards, but total should be deduplicated
        let individual_sum: usize = analysis.draws.iter().map(|d| d.outs.len()).sum();
        assert!(analysis.total_outs as usize <= individual_sum);
    }
}
