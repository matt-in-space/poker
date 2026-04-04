use crate::card::{Card, Rank};
use crate::position::Position;
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recommendation {
    Open,
    Fold,
    ThreeBet,
}

impl fmt::Display for Recommendation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Recommendation::Open => write!(f, "OPEN"),
            Recommendation::Fold => write!(f, "FOLD"),
            Recommendation::ThreeBet => write!(f, "3-BET"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HoleCardType {
    pub high: Rank,
    pub low: Rank,
    pub suited: bool,
}

impl HoleCardType {
    pub fn from_cards(a: Card, b: Card) -> Self {
        let (high, low) = if a.rank >= b.rank {
            (a.rank, b.rank)
        } else {
            (b.rank, a.rank)
        };
        let suited = a.suit == b.suit;
        HoleCardType { high, low, suited }
    }

    pub fn label(&self) -> String {
        if self.high == self.low {
            format!("{}{}", self.high, self.low)
        } else if self.suited {
            format!("{}{}s", self.high, self.low)
        } else {
            format!("{}{}o", self.high, self.low)
        }
    }

    pub fn category(&self) -> &'static str {
        let hv = self.high.value();
        let lv = self.low.value();
        let gap = hv - lv;

        if self.high == self.low {
            return "Pocket pair";
        }

        let broadway = hv >= 10 && lv >= 10;

        if self.suited {
            if broadway {
                "Suited broadway"
            } else if self.high == Rank::Ace {
                "Suited ace"
            } else if gap == 1 {
                "Suited connector"
            } else if gap == 2 {
                "Suited one-gapper"
            } else {
                "Suited hand"
            }
        } else if broadway {
            "Offsuit broadway"
        } else if self.high == Rank::Ace {
            "Offsuit ace"
        } else if gap == 1 {
            "Offsuit connector"
        } else {
            "Offsuit hand"
        }
    }

    fn key(&self) -> (u8, u8, bool) {
        (self.high.value(), self.low.value(), self.suited)
    }
}

/// Parse a range string like "AA KK QQ AKs AQs AKo" into a set of (high_val, low_val, suited) keys.
fn parse_range(range_str: &str) -> HashSet<(u8, u8, bool)> {
    let mut set = HashSet::new();
    for token in range_str.split_whitespace() {
        let chars: Vec<char> = token.chars().collect();
        if chars.len() < 2 {
            continue;
        }
        let r1 = rank_from_char(chars[0]);
        let r2 = rank_from_char(chars[1]);
        let (r1, r2) = match (r1, r2) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };

        let high = r1.max(r2);
        let low = r1.min(r2);

        if chars.len() == 2 {
            // Pair like "AA" or suited/offsuit ambiguous — treat as both if pair
            if high == low {
                set.insert((high, low, false));
            } else {
                set.insert((high, low, true));
                set.insert((high, low, false));
            }
        } else {
            match chars[2] {
                's' | 'S' => {
                    set.insert((high, low, true));
                }
                'o' | 'O' => {
                    set.insert((high, low, false));
                }
                '+' => {
                    // Pair+: e.g. "77+" means 77 through AA
                    if high == low {
                        for v in high..=14 {
                            set.insert((v, v, false));
                        }
                    }
                }
                _ => {
                    set.insert((high, low, true));
                    set.insert((high, low, false));
                }
            }
        }
    }
    set
}

fn rank_from_char(c: char) -> Option<u8> {
    match c {
        'A' | 'a' => Some(14),
        'K' | 'k' => Some(13),
        'Q' | 'q' => Some(12),
        'J' | 'j' => Some(11),
        'T' | 't' => Some(10),
        '9' => Some(9),
        '8' => Some(8),
        '7' => Some(7),
        '6' => Some(6),
        '5' => Some(5),
        '4' => Some(4),
        '3' => Some(3),
        '2' => Some(2),
        _ => None,
    }
}

// GTO-approximate opening ranges (first-in, raise or fold)
// Adapted from standard 9-max charts

const UTG_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 AKs AQs AJs ATs A9s A5s A4s KQs KJs KTs QJs QTs JTs AKo AQo";
const UTG_3BET_9: &str = "AA KK QQ AKs";

const UTG1_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 AKs AQs AJs ATs A9s A8s A5s A4s A3s KQs KJs KTs K9s QJs QTs JTs T9s AKo AQo AJo";
const UTG1_3BET_9: &str = "AA KK QQ AKs";

const UTG2_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 AKs AQs AJs ATs A9s A8s A5s A4s A3s A2s KQs KJs KTs K9s QJs QTs Q9s JTs J9s T9s AKo AQo AJo ATo";
const UTG2_3BET_9: &str = "AA KK QQ AKs AKo";

const MP_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 AKs AQs AJs ATs A9s A8s A7s A5s A4s A3s A2s KQs KJs KTs K9s K8s QJs QTs Q9s JTs J9s T9s 98s AKo AQo AJo ATo KQo";
const MP_3BET_9: &str = "AA KK QQ JJ AKs AKo";

const HJ_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 AKs AQs AJs ATs A9s A8s A7s A6s A5s A4s A3s A2s KQs KJs KTs K9s K8s K7s QJs QTs Q9s Q8s JTs J9s T9s T8s 98s 87s AKo AQo AJo ATo A9o KQo KJo QJo";
const HJ_3BET_9: &str = "AA KK QQ JJ TT AKs AQs AKo";

const CO_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 22 AKs AQs AJs ATs A9s A8s A7s A6s A5s A4s A3s A2s KQs KJs KTs K9s K8s K7s K6s K5s QJs QTs Q9s Q8s JTs J9s J8s T9s T8s 98s 87s 76s 65s AKo AQo AJo ATo A9o A8o KQo KJo KTo QJo QTo JTo";
const CO_3BET_9: &str = "AA KK QQ JJ TT AKs AQs AJs AKo AQo";

const BTN_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 22 AKs AQs AJs ATs A9s A8s A7s A6s A5s A4s A3s A2s KQs KJs KTs K9s K8s K7s K6s K5s K4s K3s K2s QJs QTs Q9s Q8s Q7s Q6s JTs J9s J8s J7s T9s T8s T7s 98s 97s 87s 86s 76s 75s 65s 64s 54s 53s 43s AKo AQo AJo ATo A9o A8o A7o A6o A5o A4o A3o A2o KQo KJo KTo K9o QJo QTo Q9o JTo J9o T9o 98o 87o";
const BTN_3BET_9: &str = "AA KK QQ JJ TT 99 AKs AQs AJs ATs A5s AKo AQo";

const SB_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 22 AKs AQs AJs ATs A9s A8s A7s A6s A5s A4s A3s A2s KQs KJs KTs K9s K8s K7s K6s K5s K4s QJs QTs Q9s Q8s Q7s JTs J9s J8s T9s T8s 98s 97s 87s 76s 65s 54s AKo AQo AJo ATo A9o A8o A7o A5o A4o KQo KJo KTo K9o QJo QTo JTo";
const SB_3BET_9: &str = "AA KK QQ JJ TT 99 AKs AQs AJs A5s AKo AQo";

// 6-max ranges are wider — use BTN-like ranges shifted earlier
const UTG_OPEN_6: &str = "AA KK QQ JJ TT 99 88 77 66 55 AKs AQs AJs ATs A9s A8s A5s A4s A3s KQs KJs KTs K9s QJs QTs Q9s JTs J9s T9s 98s AKo AQo AJo ATo KQo";
const UTG_3BET_6: &str = "AA KK QQ JJ AKs AKo";

const HJ_OPEN_6: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 AKs AQs AJs ATs A9s A8s A7s A6s A5s A4s A3s A2s KQs KJs KTs K9s K8s K7s QJs QTs Q9s Q8s JTs J9s J8s T9s T8s 98s 87s 76s AKo AQo AJo ATo A9o KQo KJo KTo QJo";
const HJ_3BET_6: &str = "AA KK QQ JJ TT AKs AQs AKo";

const CO_OPEN_6: &str = CO_OPEN_9;
const CO_3BET_6: &str = CO_3BET_9;
const BTN_OPEN_6: &str = BTN_OPEN_9;
const BTN_3BET_6: &str = BTN_3BET_9;
const SB_OPEN_6: &str = SB_OPEN_9;
const SB_3BET_6: &str = SB_3BET_9;

pub fn recommend(hole: &HoleCardType, position: Position, num_players: u8) -> Recommendation {
    let is_6max = num_players <= 6;
    let key = hole.key();

    let (open_str, threebet_str) = get_range_strings(position, is_6max);

    let threebet_range = parse_range(threebet_str);
    if threebet_range.contains(&key) {
        return Recommendation::ThreeBet;
    }

    let open_range = parse_range(open_str);
    if open_range.contains(&key) {
        return Recommendation::Open;
    }

    Recommendation::Fold
}

fn get_range_strings(position: Position, is_6max: bool) -> (&'static str, &'static str) {
    if is_6max {
        match position {
            Position::UTG | Position::UTG1 | Position::UTG2 | Position::MP => {
                (UTG_OPEN_6, UTG_3BET_6)
            }
            Position::HJ => (HJ_OPEN_6, HJ_3BET_6),
            Position::CO => (CO_OPEN_6, CO_3BET_6),
            Position::BTN => (BTN_OPEN_6, BTN_3BET_6),
            Position::SB => (SB_OPEN_6, SB_3BET_6),
            Position::BB => (BTN_OPEN_6, BTN_3BET_6), // BB has wide defend range
        }
    } else {
        match position {
            Position::UTG => (UTG_OPEN_9, UTG_3BET_9),
            Position::UTG1 => (UTG1_OPEN_9, UTG1_3BET_9),
            Position::UTG2 => (UTG2_OPEN_9, UTG2_3BET_9),
            Position::MP => (MP_OPEN_9, MP_3BET_9),
            Position::HJ => (HJ_OPEN_9, HJ_3BET_9),
            Position::CO => (CO_OPEN_9, CO_3BET_9),
            Position::BTN => (BTN_OPEN_9, BTN_3BET_9),
            Position::SB => (SB_OPEN_9, SB_3BET_9),
            Position::BB => (BTN_OPEN_9, BTN_3BET_9),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;

    fn make_hole(s1: &str, s2: &str) -> HoleCardType {
        HoleCardType::from_cards(Card::parse(s1).unwrap(), Card::parse(s2).unwrap())
    }

    #[test]
    fn test_hand_labels() {
        assert_eq!(make_hole("Ah", "Kh").label(), "AKs");
        assert_eq!(make_hole("Ah", "Kd").label(), "AKo");
        assert_eq!(make_hole("7h", "7d").label(), "77");
        assert_eq!(make_hole("9s", "8s").label(), "98s");
    }

    #[test]
    fn test_hand_categories() {
        assert_eq!(make_hole("Ah", "Ad").category(), "Pocket pair");
        assert_eq!(make_hole("Ah", "Kh").category(), "Suited broadway");
        assert_eq!(make_hole("Ah", "Kd").category(), "Offsuit broadway");
        assert_eq!(make_hole("Ah", "5h").category(), "Suited ace");
        assert_eq!(make_hole("9h", "8h").category(), "Suited connector");
        assert_eq!(make_hole("Th", "8h").category(), "Suited one-gapper");
    }

    #[test]
    fn test_premium_utg() {
        let aa = make_hole("Ah", "Ad");
        assert_eq!(recommend(&aa, Position::UTG, 9), Recommendation::ThreeBet);
    }

    #[test]
    fn test_trash_utg() {
        let hand = make_hole("7h", "2d");
        assert_eq!(recommend(&hand, Position::UTG, 9), Recommendation::Fold);
    }

    #[test]
    fn test_btn_wider() {
        // 87s should be open from BTN but fold from UTG
        let hand = make_hole("8h", "7h");
        assert_eq!(recommend(&hand, Position::BTN, 9), Recommendation::Open);
        assert_eq!(recommend(&hand, Position::UTG, 9), Recommendation::Fold);
    }

    #[test]
    fn test_parse_range() {
        let range = parse_range("AA KK AKs");
        assert!(range.contains(&(14, 14, false))); // AA
        assert!(range.contains(&(13, 13, false))); // KK
        assert!(range.contains(&(14, 13, true)));  // AKs
        assert!(!range.contains(&(14, 13, false))); // AKo not included
    }

    #[test]
    fn test_6max_wider_utg() {
        // Suited connectors like T9s should be open from UTG in 6-max
        let hand = make_hole("Th", "9h");
        assert_eq!(recommend(&hand, Position::UTG, 6), Recommendation::Open);
    }
}
