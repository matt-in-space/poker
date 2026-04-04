use std::fmt;

use crate::error::PokerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub fn value(self) -> u8 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
            Rank::Ace => 14,
        }
    }

    pub fn from_value(v: u8) -> Option<Rank> {
        match v {
            2 => Some(Rank::Two),
            3 => Some(Rank::Three),
            4 => Some(Rank::Four),
            5 => Some(Rank::Five),
            6 => Some(Rank::Six),
            7 => Some(Rank::Seven),
            8 => Some(Rank::Eight),
            9 => Some(Rank::Nine),
            10 => Some(Rank::Ten),
            11 => Some(Rank::Jack),
            12 => Some(Rank::Queen),
            13 => Some(Rank::King),
            14 => Some(Rank::Ace),
            _ => None,
        }
    }

    pub const ALL: [Rank; 13] = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    fn char(self) -> char {
        match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        }
    }

    fn parse(c: char) -> Option<Rank> {
        match c {
            '2' => Some(Rank::Two),
            '3' => Some(Rank::Three),
            '4' => Some(Rank::Four),
            '5' => Some(Rank::Five),
            '6' => Some(Rank::Six),
            '7' => Some(Rank::Seven),
            '8' => Some(Rank::Eight),
            '9' => Some(Rank::Nine),
            'T' | 't' => Some(Rank::Ten),
            'J' | 'j' => Some(Rank::Jack),
            'Q' | 'q' => Some(Rank::Queen),
            'K' | 'k' => Some(Rank::King),
            'A' | 'a' => Some(Rank::Ace),
            _ => None,
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.char())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];

    fn char(self) -> char {
        match self {
            Suit::Spades => 's',
            Suit::Hearts => 'h',
            Suit::Diamonds => 'd',
            Suit::Clubs => 'c',
        }
    }

    fn parse(c: char) -> Option<Suit> {
        match c {
            's' | 'S' => Some(Suit::Spades),
            'h' | 'H' => Some(Suit::Hearts),
            'd' | 'D' => Some(Suit::Diamonds),
            'c' | 'C' => Some(Suit::Clubs),
            _ => None,
        }
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.char())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Card { rank, suit }
    }

    pub fn parse(s: &str) -> Result<Card, PokerError> {
        let s = s.trim();
        // Handle "10x" as alias for "Tx"
        if s.len() == 3 && s.starts_with("10") {
            let suit_ch = s.chars().nth(2).unwrap();
            let suit = Suit::parse(suit_ch).ok_or_else(|| PokerError::InvalidCard(s.to_string()))?;
            return Ok(Card::new(Rank::Ten, suit));
        }

        if s.len() != 2 {
            return Err(PokerError::InvalidCard(s.to_string()));
        }

        let mut chars = s.chars();
        let rank_ch = chars.next().unwrap();
        let suit_ch = chars.next().unwrap();

        let rank = Rank::parse(rank_ch).ok_or_else(|| PokerError::InvalidCard(s.to_string()))?;
        let suit = Suit::parse(suit_ch).ok_or_else(|| PokerError::InvalidCard(s.to_string()))?;

        Ok(Card::new(rank, suit))
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_cards() {
        let card = Card::parse("As").unwrap();
        assert_eq!(card.rank, Rank::Ace);
        assert_eq!(card.suit, Suit::Spades);

        let card = Card::parse("2c").unwrap();
        assert_eq!(card.rank, Rank::Two);
        assert_eq!(card.suit, Suit::Clubs);

        let card = Card::parse("Td").unwrap();
        assert_eq!(card.rank, Rank::Ten);
        assert_eq!(card.suit, Suit::Diamonds);
    }

    #[test]
    fn parse_case_insensitive() {
        let card = Card::parse("kH").unwrap();
        assert_eq!(card.rank, Rank::King);
        assert_eq!(card.suit, Suit::Hearts);

        let card = Card::parse("jS").unwrap();
        assert_eq!(card.rank, Rank::Jack);
        assert_eq!(card.suit, Suit::Spades);

        let card = Card::parse("tD").unwrap();
        assert_eq!(card.rank, Rank::Ten);
        assert_eq!(card.suit, Suit::Diamonds);
    }

    #[test]
    fn parse_ten_alias() {
        let card = Card::parse("10s").unwrap();
        assert_eq!(card.rank, Rank::Ten);
        assert_eq!(card.suit, Suit::Spades);

        let card = Card::parse("10H").unwrap();
        assert_eq!(card.rank, Rank::Ten);
        assert_eq!(card.suit, Suit::Hearts);
    }

    #[test]
    fn parse_invalid() {
        assert!(Card::parse("1s").is_err());
        assert!(Card::parse("Ax").is_err());
        assert!(Card::parse("").is_err());
        assert!(Card::parse("AsKd").is_err());
    }

    #[test]
    fn display() {
        let card = Card::new(Rank::Ace, Suit::Spades);
        assert_eq!(format!("{card}"), "As");

        let card = Card::new(Rank::Ten, Suit::Diamonds);
        assert_eq!(format!("{card}"), "Td");
    }

    #[test]
    fn rank_ordering() {
        assert!(Rank::Ace > Rank::King);
        assert!(Rank::Two < Rank::Three);
        assert!(Rank::Ten > Rank::Nine);
    }

    #[test]
    fn rank_values() {
        assert_eq!(Rank::Two.value(), 2);
        assert_eq!(Rank::Ace.value(), 14);
        assert_eq!(Rank::Ten.value(), 10);
    }
}
