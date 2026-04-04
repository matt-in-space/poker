use std::fmt;

use crate::card::{Card, Rank};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MadeHand {
    HighCard(Rank),
    Pair {
        rank: Rank,
        quality: PairQuality,
    },
    TwoPair {
        high: Rank,
        low: Rank,
    },
    ThreeOfAKind {
        rank: Rank,
        is_set: bool,
    },
    Straight(Rank),
    Flush,
    FullHouse {
        trips: Rank,
        pair: Rank,
    },
    FourOfAKind(Rank),
    StraightFlush(Rank),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairQuality {
    Top,
    Second,
    Middle,
    Bottom,
    Overpair,
    Pocket,
}

impl fmt::Display for MadeHand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MadeHand::HighCard(r) => write!(f, "High card ({r})"),
            MadeHand::Pair { quality, rank, .. } => {
                let q = match quality {
                    PairQuality::Top => "top pair",
                    PairQuality::Overpair => "overpair",
                    PairQuality::Second => "second pair",
                    PairQuality::Middle => "middle pair",
                    PairQuality::Bottom => "bottom pair",
                    PairQuality::Pocket => "pocket pair",
                };
                write!(f, "{q} ({rank}s)")
            }
            MadeHand::TwoPair { high, low } => write!(f, "Two pair ({high}s and {low}s)"),
            MadeHand::ThreeOfAKind { rank, is_set } => {
                if *is_set {
                    write!(f, "Set of {rank}s")
                } else {
                    write!(f, "Three of a kind ({rank}s)")
                }
            }
            MadeHand::Straight(high) => write!(f, "Straight ({high} high)"),
            MadeHand::Flush => write!(f, "Flush"),
            MadeHand::FullHouse { trips, pair } => {
                write!(f, "Full house ({trips}s full of {pair}s)")
            }
            MadeHand::FourOfAKind(r) => write!(f, "Four of a kind ({r}s)"),
            MadeHand::StraightFlush(high) => {
                if *high == Rank::Ace {
                    write!(f, "Royal flush!")
                } else {
                    write!(f, "Straight flush ({high} high)")
                }
            }
        }
    }
}

pub fn evaluate(hole: &[Card; 2], board: &[Card]) -> MadeHand {
    let all: Vec<Card> = hole.iter().chain(board.iter()).copied().collect();
    let hole_ranks: Vec<Rank> = hole.iter().map(|c| c.rank).collect();

    // Count ranks and suits
    let mut rank_counts: [u8; 15] = [0; 15];
    let mut suit_counts: [u8; 4] = [0; 4];
    for &card in &all {
        rank_counts[card.rank.value() as usize] += 1;
        suit_counts[card.suit as usize] += 1;
    }

    // Check for flush
    let flush_suit = suit_counts.iter().position(|&c| c >= 5);

    // Check for straights
    let has_rank: Vec<bool> = (0..15).map(|i| rank_counts[i] > 0).collect();
    let straight_high = find_straight_high(&has_rank);

    // Straight flush check
    if let (Some(suit_idx), Some(_)) = (flush_suit, straight_high) {
        let flush_ranks: Vec<bool> = {
            let mut fr = vec![false; 15];
            for &card in &all {
                if card.suit as usize == suit_idx {
                    fr[card.rank.value() as usize] = true;
                }
            }
            // Ace low
            if fr[14] {
                fr[1] = true;
            }
            fr
        };
        if let Some(sf_high) = find_straight_high(&flush_ranks) {
            return MadeHand::StraightFlush(sf_high);
        }
    }

    // Four of a kind
    for v in (2..=14).rev() {
        if rank_counts[v] == 4 {
            return MadeHand::FourOfAKind(Rank::from_value(v as u8).unwrap());
        }
    }

    // Full house
    let mut trips_rank = None;
    let mut pair_rank = None;
    for v in (2..=14).rev() {
        if rank_counts[v] >= 3 && trips_rank.is_none() {
            trips_rank = Some(v);
        } else if rank_counts[v] >= 2 && pair_rank.is_none() {
            pair_rank = Some(v);
        }
    }
    if let (Some(t), Some(p)) = (trips_rank, pair_rank) {
        return MadeHand::FullHouse {
            trips: Rank::from_value(t as u8).unwrap(),
            pair: Rank::from_value(p as u8).unwrap(),
        };
    }

    // Flush
    if flush_suit.is_some() {
        return MadeHand::Flush;
    }

    // Straight
    if let Some(high) = straight_high {
        return MadeHand::Straight(high);
    }

    // Three of a kind
    if let Some(t) = trips_rank {
        let rank = Rank::from_value(t as u8).unwrap();
        let is_set = hole_ranks.iter().filter(|&&r| r == rank).count() == 2;
        return MadeHand::ThreeOfAKind { rank, is_set };
    }

    // Two pair / pair
    let mut pairs: Vec<Rank> = Vec::new();
    for v in (2..=14).rev() {
        if rank_counts[v] >= 2 {
            pairs.push(Rank::from_value(v as u8).unwrap());
        }
    }

    if pairs.len() >= 2 {
        return MadeHand::TwoPair {
            high: pairs[0],
            low: pairs[1],
        };
    }

    if pairs.len() == 1 {
        let pair_rank = pairs[0];
        let quality = if board.is_empty() {
            PairQuality::Pocket
        } else {
            classify_pair(pair_rank, &hole_ranks, board)
        };
        return MadeHand::Pair {
            rank: pair_rank,
            quality,
        };
    }

    // High card
    let high = hole_ranks.iter().max().copied().unwrap();
    MadeHand::HighCard(high)
}

fn classify_pair(pair_rank: Rank, hole_ranks: &[Rank], board: &[Card]) -> PairQuality {
    let hole_has_pair_rank = hole_ranks.contains(&pair_rank);
    if !hole_has_pair_rank {
        // Pair is entirely on the board — effectively high card for us, but eval says pair
        return PairQuality::Bottom;
    }

    // Check if it's an overpair (pocket pair above all board cards)
    if hole_ranks[0] == hole_ranks[1] {
        let board_max = board.iter().map(|c| c.rank).max().unwrap();
        if pair_rank > board_max {
            return PairQuality::Overpair;
        }
        return PairQuality::Pocket;
    }

    // Pair made with a hole card + board card
    let mut board_ranks: Vec<Rank> = board.iter().map(|c| c.rank).collect();
    board_ranks.sort();
    board_ranks.dedup();
    board_ranks.sort_by(|a, b| b.cmp(a)); // descending

    if let Some(0) = board_ranks.iter().position(|&r| r == pair_rank) {
        PairQuality::Top
    } else if let Some(1) = board_ranks.iter().position(|&r| r == pair_rank) {
        PairQuality::Second
    } else {
        let len = board_ranks.len();
        if board_ranks.iter().position(|&r| r == pair_rank) == Some(len - 1) {
            PairQuality::Bottom
        } else {
            PairQuality::Middle
        }
    }
}

fn find_straight_high(has_rank: &[bool]) -> Option<Rank> {
    // has_rank[1] = ace-low, has_rank[2..=14] = 2 through ace
    // Check ace-low: A-2-3-4-5
    // We need has_rank to be at least 15 elements
    // Check windows from high to low
    for high in (5..=14).rev() {
        let low = high - 4;
        let mut all_present = true;
        for v in low..=high {
            let idx = if v == 1 { 14 } else { v }; // ace-low maps to 14
            if !has_rank[idx] {
                all_present = false;
                break;
            }
        }
        if all_present {
            return Some(Rank::from_value(high as u8).unwrap());
        }
    }

    // Check ace-low straight: A-2-3-4-5
    if has_rank[14] && has_rank[2] && has_rank[3] && has_rank[4] && has_rank[5] {
        return Some(Rank::Five); // 5-high straight
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;

    fn c(s: &str) -> Card {
        Card::parse(s).unwrap()
    }

    #[test]
    fn test_high_card() {
        let hand = evaluate(&[c("Ah"), c("Kd")], &[c("2s"), c("5c"), c("9h")]);
        assert!(matches!(hand, MadeHand::HighCard(Rank::Ace)));
    }

    #[test]
    fn test_top_pair() {
        let hand = evaluate(&[c("Ah"), c("Kd")], &[c("As"), c("5c"), c("9h")]);
        assert!(matches!(
            hand,
            MadeHand::Pair {
                rank: Rank::Ace,
                quality: PairQuality::Top
            }
        ));
    }

    #[test]
    fn test_overpair() {
        let hand = evaluate(&[c("Kh"), c("Kd")], &[c("Qs"), c("5c"), c("9h")]);
        assert!(matches!(
            hand,
            MadeHand::Pair {
                rank: Rank::King,
                quality: PairQuality::Overpair
            }
        ));
    }

    #[test]
    fn test_flush() {
        let hand = evaluate(
            &[c("Ah"), c("Kh")],
            &[c("2h"), c("5h"), c("9h")],
        );
        assert!(matches!(hand, MadeHand::Flush));
    }

    #[test]
    fn test_straight() {
        let hand = evaluate(
            &[c("9h"), c("8d")],
            &[c("7s"), c("6c"), c("5h")],
        );
        assert!(matches!(hand, MadeHand::Straight(Rank::Nine)));
    }

    #[test]
    fn test_ace_low_straight() {
        let hand = evaluate(
            &[c("Ah"), c("2d")],
            &[c("3s"), c("4c"), c("5h")],
        );
        assert!(matches!(hand, MadeHand::Straight(Rank::Five)));
    }

    #[test]
    fn test_full_house() {
        let hand = evaluate(
            &[c("Ah"), c("Ad")],
            &[c("As"), c("Kc"), c("Kh")],
        );
        assert!(matches!(
            hand,
            MadeHand::FullHouse {
                trips: Rank::Ace,
                pair: Rank::King
            }
        ));
    }

    #[test]
    fn test_set() {
        let hand = evaluate(
            &[c("7h"), c("7d")],
            &[c("7s"), c("Kc"), c("2h")],
        );
        assert!(matches!(
            hand,
            MadeHand::ThreeOfAKind {
                rank: Rank::Seven,
                is_set: true
            }
        ));
    }

    #[test]
    fn test_two_pair() {
        let hand = evaluate(
            &[c("Ah"), c("Kd")],
            &[c("As"), c("Kc"), c("2h")],
        );
        assert!(matches!(
            hand,
            MadeHand::TwoPair {
                high: Rank::Ace,
                low: Rank::King
            }
        ));
    }
}
