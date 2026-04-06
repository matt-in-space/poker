use crate::card::Card;
use crate::position::{self, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FirstIn,
    FacingLimp,
    FacingRaise,
}

pub struct HandState {
    pub hole_cards: Option<[Card; 2]>,
    pub num_players: u8,
    pub position_index: usize,
    pub configured: bool,
    pub action: Action,
}

impl HandState {
    pub fn new() -> Self {
        HandState {
            hole_cards: None,
            num_players: 9,
            position_index: 0,
            configured: false,
            action: Action::FirstIn,
        }
    }

    pub fn reset(&mut self) {
        self.hole_cards = None;
        self.action = Action::FirstIn;
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
}
