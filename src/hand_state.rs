use crate::card::Card;
use crate::error::PokerError;
use crate::position::{self, Position};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FirstIn,
    FacingLimp,
    FacingRaise,
}

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
            Street::Preflop => write!(f, "preflop"),
            Street::Flop => write!(f, "flop"),
            Street::Turn => write!(f, "turn"),
            Street::River => write!(f, "river"),
        }
    }
}

pub struct HandState {
    pub hole_cards: Option<[Card; 2]>,
    pub num_players: u8,
    pub position_index: usize,
    pub configured: bool,
    pub action: Action,
    pub board: Vec<Card>,
    pub street: Street,
    pub big_blind: Option<u64>,
    pub raise_amount: Option<u64>,
}

impl HandState {
    pub fn new() -> Self {
        HandState {
            hole_cards: None,
            num_players: 9,
            position_index: 0,
            configured: false,
            action: Action::FirstIn,
            board: Vec::new(),
            street: Street::Preflop,
            big_blind: None,
            raise_amount: None,
        }
    }

    pub fn reset(&mut self) {
        self.hole_cards = None;
        self.action = Action::FirstIn;
        self.raise_amount = None;
        self.board.clear();
        self.street = Street::Preflop;
    }

    pub fn position(&self) -> Option<Position> {
        if !self.configured {
            return None;
        }
        let positions = position::positions_for_table_size(self.num_players);
        Some(positions[self.position_index % positions.len()])
    }

    pub fn advance_position(&mut self) {
        let positions = position::positions_for_table_size(self.num_players);
        let len = positions.len();
        self.position_index = (self.position_index + len - 1) % len;
    }

    pub fn set_position(&mut self, pos: Position) -> bool {
        let positions = position::positions_for_table_size(self.num_players);
        if let Some(idx) = positions.iter().position(|&p| p == pos) {
            self.position_index = idx;
            true
        } else {
            false
        }
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
}
