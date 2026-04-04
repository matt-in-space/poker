use std::fmt;

pub struct PotOdds {
    pub pot_size: u64,
    pub bet_size: u64,
    pub required_equity: f64,
}

impl PotOdds {
    pub fn calculate(pot_size: u64, bet_size: u64) -> Self {
        let required_equity = if pot_size + bet_size == 0 {
            0.0
        } else {
            (bet_size as f64 / (pot_size + bet_size) as f64) * 100.0
        };
        PotOdds {
            pot_size,
            bet_size,
            required_equity,
        }
    }
}

impl fmt::Display for PotOdds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Call ${} into ${} pot → need {:.1}% equity to break even",
            self.bet_size, self.pot_size, self.required_equity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pot_odds_basic() {
        let odds = PotOdds::calculate(200, 50);
        assert!((odds.required_equity - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_pot_odds_half_pot() {
        let odds = PotOdds::calculate(100, 50);
        assert!((odds.required_equity - 33.3).abs() < 0.1);
    }

    #[test]
    fn test_pot_odds_full_pot() {
        let odds = PotOdds::calculate(100, 100);
        assert!((odds.required_equity - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_pot_odds_zero() {
        let odds = PotOdds::calculate(0, 0);
        assert_eq!(odds.required_equity, 0.0);
    }
}
