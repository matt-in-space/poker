use crate::card::{Card, Rank};
use crate::position::Position;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recommendation {
    Open,
    Fold,
    ThreeBet,
    IsoRaise,
    Call,
    Check,
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
                    if chars.len() > 3 && chars[3] == '+' {
                        // e.g. "A5s+" means A5s, A6s, A7s, ..., AKs (high card fixed, low goes up)
                        for v in low..high {
                            set.insert((high, v, true));
                        }
                    } else {
                        set.insert((high, low, true));
                    }
                }
                'o' | 'O' => {
                    if chars.len() > 3 && chars[3] == '+' {
                        // e.g. "A9o+" means A9o, ATo, AJo, AQo, AKo
                        for v in low..high {
                            set.insert((high, v, false));
                        }
                    } else {
                        set.insert((high, low, false));
                    }
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

// Preflop opening ranges, tightened for micro-stakes rake.
// Based on Upswing Poker simplified GTO charts, adjusted ~3-5% tighter.
// 3-bet ranges are polarized: premium value hands + low suited aces as bluffs
// (A5s-A2s block AA/AK and have decent suited equity when called).
//
// These assume ~100bb stacks, NL cash games.
// Source baseline: Upswing Poker / GTO Wizard simplified charts.

// --- 9-max ranges ---

const UTG_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 AKs AQs AJs ATs KQs KJs QJs JTs AKo AQo";
const UTG_3BET_9: &str = "AA KK QQ AKs";

const UTG1_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 AKs AQs AJs ATs A9s KQs KJs KTs QJs QTs JTs T9s AKo AQo AJo";
const UTG1_3BET_9: &str = "AA KK QQ AKs AKo";

const UTG2_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 AKs AQs AJs ATs A9s A8s KQs KJs KTs QJs QTs JTs J9s T9s 98s AKo AQo AJo ATo";
const UTG2_3BET_9: &str = "AA KK QQ AKs AKo";

const MP_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 AKs AQs AJs ATs A9s A8s A5s KQs KJs KTs K9s QJs QTs J9s T9s 98s 87s AKo AQo AJo ATo KQo";
const MP_3BET_9: &str = "AA KK QQ JJ AKs A5s AKo";

const HJ_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 AKs AQs AJs ATs A9s A8s A7s A5s A4s A3s A2s KQs KJs KTs K9s K8s QJs QTs Q9s JTs J9s T9s T8s 98s 87s 76s A9o+ KQo KJo QJo";
const HJ_3BET_9: &str = "AA KK QQ JJ TT AKs AQs A5s A4s AKo";

const CO_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 22 AKs AQs AJs ATs A9s A8s A7s A6s A5s A4s A3s A2s KQs KJs KTs K9s K8s K7s K6s QJs QTs Q9s Q8s JTs J9s J8s T9s T8s 98s 97s 87s 76s 65s 54s A8o+ KQo KJo KTo QJo QTo JTo";
const CO_3BET_9: &str = "AA KK QQ JJ TT AKs AQs AJs A5s A4s A3s AKo AQo";

const BTN_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 22 AKs AQs AJs ATs A9s A8s A7s A6s A5s A4s A3s A2s KQs KJs KTs K9s K8s K7s K6s K5s K4s K3s K2s QJs QTs Q9s Q8s Q7s Q6s JTs J9s J8s J7s T9s T8s T7s 98s 97s 87s 86s 76s 75s 65s 64s 54s 53s 43s AKo AQo AJo ATo A9o A8o A7o A5o A4o A3o A2o KQo KJo KTo K9o QJo QTo Q9o JTo J9o T9o 98o 87o";
const BTN_3BET_9: &str = "AA KK QQ JJ TT 99 AKs AQs AJs ATs A5s A4s A3s A2s AKo AQo";

const SB_OPEN_9: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 33 22 AKs AQs AJs ATs A9s A8s A7s A6s A5s A4s A3s A2s KQs KJs KTs K9s K8s K7s K6s K5s QJs QTs Q9s Q8s JTs J9s J8s T9s T8s 98s 87s 76s 65s 54s AKo AQo AJo ATo A9o A8o A5o KQo KJo KTo QJo QTo JTo";
const SB_3BET_9: &str = "AA KK QQ JJ TT 99 AKs AQs AJs A5s A4s A3s A2s AKo AQo";

// --- 6-max ranges ---
// 6-max UTG has 5 players behind, similar to 9-max MP.
// Positions from HJ onward are the same as 9-max equivalents.

const UTG_OPEN_6: &str = "AA KK QQ JJ TT 99 88 77 66 55 44 AKs AQs AJs ATs A9s A8s A5s KQs KJs KTs K9s QJs QTs J9s T9s 98s 87s AKo AQo AJo ATo KQo";
const UTG_3BET_6: &str = "AA KK QQ JJ AKs A5s AKo";

const HJ_OPEN_6: &str = HJ_OPEN_9;
const HJ_3BET_6: &str = HJ_3BET_9;
const CO_OPEN_6: &str = CO_OPEN_9;
const CO_3BET_6: &str = CO_3BET_9;
const BTN_OPEN_6: &str = BTN_OPEN_9;
const BTN_3BET_6: &str = BTN_3BET_9;
const SB_OPEN_6: &str = SB_OPEN_9;
const SB_3BET_6: &str = SB_3BET_9;

// Premium-only range for facing very large raises (>15x BB).
const PREMIUM_RANGE: &str = "AA KK QQ AKs";

// Tighter range for facing large raises (5-15x BB).
const LARGE_RAISE_3BET: &str = "AA KK";

pub fn recommend(
    hole: &HoleCardType,
    position: Position,
    num_players: u8,
    action: crate::hand_state::Action,
    raise_bb: Option<f64>,
) -> Recommendation {
    use crate::hand_state::Action;

    let is_6max = num_players <= 6;
    let key = hole.key();

    let (open_str, threebet_str) = get_range_strings(position, is_6max);
    let threebet_range = parse_range(threebet_str);
    let open_range = parse_range(open_str);

    let in_3bet = threebet_range.contains(&key);
    let in_open = open_range.contains(&key);

    let is_bb = position == Position::BB;

    match action {
        Action::FirstIn => {
            if in_3bet {
                Recommendation::ThreeBet
            } else if in_open {
                Recommendation::Open
            } else if is_bb {
                Recommendation::Check
            } else {
                Recommendation::Fold
            }
        }
        Action::FacingLimp => {
            if in_3bet || in_open {
                Recommendation::IsoRaise
            } else if is_bb {
                Recommendation::Check
            } else {
                Recommendation::Fold
            }
        }
        Action::FacingRaise => {
            match raise_bb {
                // Huge raise (>15x BB): only play premiums
                Some(bb_mult) if bb_mult > 15.0 => {
                    let premium = parse_range(PREMIUM_RANGE);
                    if premium.contains(&key) {
                        Recommendation::Call
                    } else {
                        Recommendation::Fold
                    }
                }
                // Large raise (5-15x BB): call with 3-bet hands, re-raise only AA/KK
                Some(bb_mult) if bb_mult > 5.0 => {
                    let top = parse_range(LARGE_RAISE_3BET);
                    if top.contains(&key) {
                        Recommendation::ThreeBet
                    } else if in_3bet {
                        Recommendation::Call
                    } else {
                        Recommendation::Fold
                    }
                }
                // Normal raise: standard ranges
                _ => {
                    if in_3bet {
                        Recommendation::ThreeBet
                    } else if in_open {
                        Recommendation::Call
                    } else {
                        Recommendation::Fold
                    }
                }
            }
        }
    }
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
    use crate::hand_state::Action;

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
        assert_eq!(recommend(&aa, Position::UTG, 9, Action::FirstIn, None), Recommendation::ThreeBet);
    }

    #[test]
    fn test_trash_utg() {
        let hand = make_hole("7h", "2d");
        assert_eq!(recommend(&hand, Position::UTG, 9, Action::FirstIn, None), Recommendation::Fold);
    }

    #[test]
    fn test_btn_wider() {
        // 87s should be open from BTN but fold from UTG
        let hand = make_hole("8h", "7h");
        assert_eq!(recommend(&hand, Position::BTN, 9, Action::FirstIn, None), Recommendation::Open);
        assert_eq!(recommend(&hand, Position::UTG, 9, Action::FirstIn, None), Recommendation::Fold);
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
    fn test_parse_range_plus_suffix() {
        // A9o+ should expand to A9o, ATo, AJo, AQo, AKo
        let range = parse_range("A9o+");
        assert!(range.contains(&(14, 9, false)));  // A9o
        assert!(range.contains(&(14, 10, false))); // ATo
        assert!(range.contains(&(14, 11, false))); // AJo
        assert!(range.contains(&(14, 12, false))); // AQo
        assert!(range.contains(&(14, 13, false))); // AKo
        assert!(!range.contains(&(14, 8, false))); // A8o not included
        assert!(!range.contains(&(14, 9, true)));  // A9s not included

        // A5s+ should expand suited hands
        let range2 = parse_range("A5s+");
        assert!(range2.contains(&(14, 5, true)));  // A5s
        assert!(range2.contains(&(14, 9, true)));  // A9s
        assert!(range2.contains(&(14, 13, true))); // AKs
        assert!(!range2.contains(&(14, 4, true))); // A4s not included
    }

    #[test]
    fn test_6max_wider_utg() {
        // T9s should be open from UTG in 6-max
        let hand = make_hole("Th", "9h");
        assert_eq!(recommend(&hand, Position::UTG, 6, Action::FirstIn, None), Recommendation::Open);
    }

    #[test]
    fn test_polarized_3bet() {
        // A5s should be a 3-bet bluff from MP+ in 9-max (polarized range)
        let a5s = make_hole("Ah", "5h");
        assert_eq!(recommend(&a5s, Position::MP, 9, Action::FirstIn, None), Recommendation::ThreeBet);
        // But A6s from MP should just be open (not in 3-bet range)
        let a6s = make_hole("Ah", "6h");
        assert_eq!(recommend(&a6s, Position::MP, 9, Action::FirstIn, None), Recommendation::Fold);
    }

    #[test]
    fn test_facing_limp() {
        // An open-range hand facing a limp should be ISO-RAISE
        let hand = make_hole("Ah", "Js");
        assert_eq!(recommend(&hand, Position::CO, 9, Action::FacingLimp, None), Recommendation::IsoRaise);
        // Trash facing a limp is still fold
        let trash = make_hole("7h", "2d");
        assert_eq!(recommend(&trash, Position::CO, 9, Action::FacingLimp, None), Recommendation::Fold);
    }

    #[test]
    fn test_facing_raise() {
        // A 3-bet hand facing a raise should still 3-bet
        let aa = make_hole("Ah", "Ad");
        assert_eq!(recommend(&aa, Position::CO, 9, Action::FacingRaise, None), Recommendation::ThreeBet);
        // An open-range hand facing a raise should call
        let hand = make_hole("8h", "7h");
        assert_eq!(recommend(&hand, Position::BTN, 9, Action::FacingRaise, None), Recommendation::Call);
        // Trash facing a raise is fold
        let trash = make_hole("7h", "2d");
        assert_eq!(recommend(&trash, Position::BTN, 9, Action::FacingRaise, None), Recommendation::Fold);
    }

    #[test]
    fn test_bb_check_first_in() {
        // BB with trash and no raise should check, not fold
        let trash = make_hole("7h", "2d");
        assert_eq!(recommend(&trash, Position::BB, 9, Action::FirstIn, None), Recommendation::Check);
        // BB with a good hand should still raise
        let good = make_hole("Ah", "Kh");
        assert_ne!(recommend(&good, Position::BB, 9, Action::FirstIn, None), Recommendation::Check);
    }

    #[test]
    fn test_bb_check_facing_limp() {
        // BB with trash facing a limp should check, not fold
        let trash = make_hole("7h", "2d");
        assert_eq!(recommend(&trash, Position::BB, 9, Action::FacingLimp, None), Recommendation::Check);
    }

    #[test]
    fn test_bb_fold_facing_raise() {
        // BB with trash facing a raise should still fold
        let trash = make_hole("7h", "2d");
        assert_eq!(recommend(&trash, Position::BB, 9, Action::FacingRaise, None), Recommendation::Fold);
    }

    #[test]
    fn test_large_raise_tightens_range() {
        // 87s is a call at normal sizing but should fold facing 10x BB
        let hand = make_hole("8h", "7h");
        assert_eq!(recommend(&hand, Position::BTN, 9, Action::FacingRaise, None), Recommendation::Call);
        assert_eq!(recommend(&hand, Position::BTN, 9, Action::FacingRaise, Some(10.0)), Recommendation::Fold);

        // AQs is in the 3-bet range at normal sizing, becomes a call at 10x BB
        let aqs = make_hole("Ah", "Qh");
        assert_eq!(recommend(&aqs, Position::BTN, 9, Action::FacingRaise, None), Recommendation::ThreeBet);
        assert_eq!(recommend(&aqs, Position::BTN, 9, Action::FacingRaise, Some(10.0)), Recommendation::Call);

        // AA still re-raises at 10x BB
        let aa = make_hole("Ah", "Ad");
        assert_eq!(recommend(&aa, Position::BTN, 9, Action::FacingRaise, Some(10.0)), Recommendation::ThreeBet);
    }

    #[test]
    fn test_huge_raise_only_premiums() {
        // At 33x BB, only AA/KK/QQ/AKs should play
        let aa = make_hole("Ah", "Ad");
        assert_eq!(recommend(&aa, Position::BTN, 9, Action::FacingRaise, Some(33.0)), Recommendation::Call);

        let kk = make_hole("Kh", "Kd");
        assert_eq!(recommend(&kk, Position::BTN, 9, Action::FacingRaise, Some(33.0)), Recommendation::Call);

        let aks = make_hole("Ah", "Kh");
        assert_eq!(recommend(&aks, Position::BTN, 9, Action::FacingRaise, Some(33.0)), Recommendation::Call);

        // JJ should fold at 33x BB
        let jj = make_hole("Jh", "Jd");
        assert_eq!(recommend(&jj, Position::BTN, 9, Action::FacingRaise, Some(33.0)), Recommendation::Fold);

        // 87s definitely folds
        let hand = make_hole("8h", "7h");
        assert_eq!(recommend(&hand, Position::BTN, 9, Action::FacingRaise, Some(33.0)), Recommendation::Fold);
    }
}
