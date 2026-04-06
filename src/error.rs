use std::fmt;

use crate::card::Card;

#[derive(Debug)]
pub enum PokerError {
    InvalidCard(String),
    DuplicateCard(Card),
    InvalidPosition(String),
    WrongArgCount {
        command: &'static str,
        usage: &'static str,
    },
    NotConfigured,
    NoDeal,
    WrongStreet { expected: &'static str },
}

impl fmt::Display for PokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PokerError::InvalidCard(s) => {
                write!(f, "Unknown card: '{s}' — use rank+suit like 'As', 'Td', '2c'")
            }
            PokerError::DuplicateCard(card) => {
                write!(f, "Card {card} is already in play")
            }
            PokerError::InvalidPosition(s) => {
                write!(
                    f,
                    "Unknown position '{s}' — try 'utg', 'mp', 'co', 'btn', 'sb', 'bb'"
                )
            }
            PokerError::WrongArgCount { command, usage } => {
                write!(f, "Usage: {command} {usage}")
            }
            PokerError::NotConfigured => {
                write!(f, "Not configured yet — use 'players' and 'pos' first")
            }
            PokerError::NoDeal => {
                write!(f, "No hand in progress — use 'deal' first")
            }
            PokerError::WrongStreet { expected } => {
                write!(f, "{expected}")
            }
        }
    }
}
