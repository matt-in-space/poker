use std::fmt;

use crate::error::PokerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Position {
    UTG,
    UTG1,
    UTG2,
    MP,
    HJ,
    CO,
    BTN,
    SB,
    BB,
}

impl Position {
    pub fn parse(s: &str) -> Result<Position, PokerError> {
        match s.to_lowercase().as_str() {
            "utg" => Ok(Position::UTG),
            "utg1" | "utg+1" => Ok(Position::UTG1),
            "utg2" | "utg+2" => Ok(Position::UTG2),
            "mp" => Ok(Position::MP),
            "hj" => Ok(Position::HJ),
            "co" => Ok(Position::CO),
            "btn" | "bu" | "button" => Ok(Position::BTN),
            "sb" => Ok(Position::SB),
            "bb" => Ok(Position::BB),
            _ => Err(PokerError::InvalidPosition(s.to_string())),
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Position::UTG => "UTG",
            Position::UTG1 => "UTG+1",
            Position::UTG2 => "UTG+2",
            Position::MP => "MP",
            Position::HJ => "HJ",
            Position::CO => "CO",
            Position::BTN => "BTN",
            Position::SB => "SB",
            Position::BB => "BB",
        }
    }

    pub fn long_name(self) -> &'static str {
        match self {
            Position::UTG => "Under the Gun",
            Position::UTG1 => "UTG+1",
            Position::UTG2 => "UTG+2",
            Position::MP => "Middle Position",
            Position::HJ => "Hijack",
            Position::CO => "Cutoff",
            Position::BTN => "Button",
            Position::SB => "Small Blind",
            Position::BB => "Big Blind",
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

/// Returns the active positions for a given table size (2-9 players),
/// ordered clockwise from UTG (first to act preflop) through BB.
pub fn positions_for_table_size(n: u8) -> Vec<Position> {
    // Always have SB, BB, BTN. Fill remaining from CO backward toward UTG.
    // 2: SB, BB
    // 3: BTN, SB, BB
    // 4: CO, BTN, SB, BB
    // 5: HJ, CO, BTN, SB, BB
    // 6: UTG, HJ, CO, BTN, SB, BB
    // 7: UTG, MP, HJ, CO, BTN, SB, BB
    // 8: UTG, UTG1, MP, HJ, CO, BTN, SB, BB
    // 9: UTG, UTG1, UTG2, MP, HJ, CO, BTN, SB, BB
    let n = n.clamp(2, 9);
    match n {
        2 => vec![Position::SB, Position::BB],
        3 => vec![Position::BTN, Position::SB, Position::BB],
        4 => vec![Position::CO, Position::BTN, Position::SB, Position::BB],
        5 => vec![Position::HJ, Position::CO, Position::BTN, Position::SB, Position::BB],
        6 => vec![Position::UTG, Position::HJ, Position::CO, Position::BTN, Position::SB, Position::BB],
        7 => vec![Position::UTG, Position::MP, Position::HJ, Position::CO, Position::BTN, Position::SB, Position::BB],
        8 => vec![Position::UTG, Position::UTG1, Position::MP, Position::HJ, Position::CO, Position::BTN, Position::SB, Position::BB],
        _ => vec![Position::UTG, Position::UTG1, Position::UTG2, Position::MP, Position::HJ, Position::CO, Position::BTN, Position::SB, Position::BB],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_positions() {
        assert_eq!(Position::parse("utg").unwrap(), Position::UTG);
        assert_eq!(Position::parse("UTG").unwrap(), Position::UTG);
        assert_eq!(Position::parse("btn").unwrap(), Position::BTN);
        assert_eq!(Position::parse("BTN").unwrap(), Position::BTN);
        assert_eq!(Position::parse("co").unwrap(), Position::CO);
        assert_eq!(Position::parse("utg1").unwrap(), Position::UTG1);
        assert_eq!(Position::parse("utg+2").unwrap(), Position::UTG2);
        assert!(Position::parse("co2").is_err());
    }

    #[test]
    fn table_sizes() {
        let p9 = positions_for_table_size(9);
        assert_eq!(p9.len(), 9);
        assert_eq!(p9[0], Position::UTG);
        assert_eq!(p9[8], Position::BB);

        let p6 = positions_for_table_size(6);
        assert_eq!(p6.len(), 6);
        assert_eq!(p6[0], Position::UTG);
        assert_eq!(p6[1], Position::HJ);
        assert_eq!(p6[5], Position::BB);

        let p2 = positions_for_table_size(2);
        assert_eq!(p2.len(), 2);
        assert_eq!(p2[0], Position::SB);
        assert_eq!(p2[1], Position::BB);
    }

    #[test]
    fn table_size_clamped() {
        assert_eq!(positions_for_table_size(1).len(), 2);
        assert_eq!(positions_for_table_size(10).len(), 9);
    }
}
