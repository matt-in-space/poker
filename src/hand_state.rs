use crate::card::Card;
use crate::error::PokerError;
use crate::position::Position;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
}

impl std::fmt::Display for Street {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Street::Preflop => write!(f, "Preflop"),
            Street::Flop => write!(f, "Flop"),
            Street::Turn => write!(f, "Turn"),
            Street::River => write!(f, "River"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Fold,
    Check,
    Call,
    Raise(u64),
    AllIn,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Fold => write!(f, "fold"),
            Action::Check => write!(f, "check"),
            Action::Call => write!(f, "call"),
            Action::Raise(amt) => write!(f, "raise {amt}"),
            Action::AllIn => write!(f, "all-in"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActionEntry {
    pub position: Position,
    pub action: Action,
    pub street: Street,
}

pub struct HandState {
    pub hole_cards: Option<[Card; 2]>,
    pub position: Option<Position>,
    pub num_players: u8,
    pub board: Vec<Card>,
    pub street: Street,
    pub pot: u64,
    pub actions: Vec<ActionEntry>,
}

impl HandState {
    pub fn new() -> Self {
        HandState {
            hole_cards: None,
            position: None,
            num_players: 9,
            board: Vec::new(),
            street: Street::Preflop,
            pot: 0,
            actions: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        let players = self.num_players;
        *self = HandState::new();
        self.num_players = players;
    }

    pub fn cards_in_play(&self) -> HashSet<Card> {
        let mut set = HashSet::new();
        if let Some(hole) = &self.hole_cards {
            set.insert(hole[0]);
            set.insert(hole[1]);
        }
        for &c in &self.board {
            set.insert(c);
        }
        set
    }

    pub fn check_not_duplicate(&self, card: Card) -> Result<(), PokerError> {
        if self.cards_in_play().contains(&card) {
            Err(PokerError::DuplicateCard(card))
        } else {
            Ok(())
        }
    }

    pub fn check_duplicates(&self, cards: &[Card]) -> Result<(), PokerError> {
        let in_play = self.cards_in_play();
        let mut seen = HashSet::new();
        for &card in cards {
            if in_play.contains(&card) || !seen.insert(card) {
                return Err(PokerError::DuplicateCard(card));
            }
        }
        Ok(())
    }

    pub fn actions_on_street(&self, street: Street) -> Vec<&ActionEntry> {
        self.actions.iter().filter(|a| a.street == street).collect()
    }
}
