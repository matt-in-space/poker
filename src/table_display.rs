use crate::position::{positions_for_table_size, Position};

pub fn render_table(num_players: u8, hero_position: Option<Position>) -> String {
    let positions = positions_for_table_size(num_players);
    let coords = seat_coords(positions.len());

    // Find grid dimensions
    let max_row = coords.iter().map(|&(r, _)| r).max().unwrap_or(0);
    let max_col = coords.iter().map(|&(_, c)| c).max().unwrap_or(0) + 12;

    // Build 2D character buffer
    let mut grid: Vec<Vec<char>> = vec![vec![' '; max_col + 1]; max_row + 2];

    let mut hero_row = 0;
    let mut hero_col = 0;

    for (i, &pos) in positions.iter().enumerate() {
        if i >= coords.len() {
            break;
        }
        let (row, col) = coords[i];
        let is_hero = hero_position == Some(pos);
        let is_btn = pos == Position::BTN;

        let label = if is_btn {
            format!("D:{}", pos.short_name())
        } else {
            pos.short_name().to_string()
        };

        let formatted = if is_hero {
            format!("( {} )", label)
        } else {
            format!("[ {} ]", label)
        };

        if is_hero {
            hero_row = row;
            hero_col = col + formatted.len() / 2;
        }

        for (j, ch) in formatted.chars().enumerate() {
            if col + j < grid[row].len() {
                grid[row][col + j] = ch;
            }
        }
    }

    // Add hero indicator
    if hero_position.is_some() && hero_row + 1 < grid.len() {
        let indicator = "^-- you";
        let start = if hero_col >= 1 { hero_col - 1 } else { 0 };
        for (j, ch) in indicator.chars().enumerate() {
            if start + j < grid[hero_row + 1].len() {
                grid[hero_row + 1][start + j] = ch;
            }
        }
    }

    // Convert to string, trimming trailing spaces
    grid.iter()
        .map(|row| row.iter().collect::<String>().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end_matches('\n')
        .to_string()
}

/// Returns (row, col) coordinates for each seat, arranged in an oval.
/// Seats are ordered to match the output of positions_for_table_size:
/// early positions at the top, BTN/SB/BB at the bottom-left.
fn seat_coords(num_seats: usize) -> Vec<(usize, usize)> {
    match num_seats {
        2 => vec![
            (0, 10), // SB
            (0, 22), // BB
        ],
        3 => vec![
            (0, 16),  // BTN (top center)
            (2, 6),   // SB
            (2, 24),  // BB
        ],
        4 => vec![
            (0, 16),  // CO
            (2, 28),  // BTN
            (2, 4),   // SB
            (0, 4),   // BB -- wait, order matters
        ],
        // For 4+, follow the clockwise pattern:
        // Top center, then clockwise
        5 => vec![
            (0, 14),  // HJ
            (2, 26),  // CO
            (4, 22),  // BTN
            (4, 6),   // SB
            (2, 2),   // BB
        ],
        6 => vec![
            (0, 14),  // UTG
            (0, 28),  // HJ
            (2, 30),  // CO
            (4, 22),  // BTN
            (4, 6),   // SB
            (2, 2),   // BB
        ],
        7 => vec![
            (0, 14),  // UTG
            (0, 28),  // MP
            (2, 32),  // HJ
            (4, 28),  // CO
            (4, 14),  // BTN
            (4, 2),   // SB
            (2, 2),   // BB
        ],
        8 => vec![
            (0, 14),  // UTG
            (0, 28),  // UTG1
            (2, 34),  // MP
            (4, 30),  // HJ
            (4, 18),  // CO
            (4, 6),   // BTN
            (2, 0),   // SB
            (0, 2),   // BB
        ],
        9 => vec![
            (0, 16),  // UTG
            (0, 30),  // UTG1
            (2, 34),  // UTG2
            (4, 32),  // MP
            (4, 20),  // HJ
            (6, 18),  // CO
            (6, 6),   // BTN
            (4, 0),   // SB
            (2, 0),   // BB
        ],
        _ => vec![(0, 10)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_6_players() {
        let output = render_table(6, Some(Position::BTN));
        assert!(output.contains("BTN"));
        assert!(output.contains("you"));
        assert!(output.contains("UTG"));
    }

    #[test]
    fn test_render_9_players() {
        let output = render_table(9, Some(Position::CO));
        assert!(output.contains("CO"));
        assert!(output.contains("you"));
        assert!(output.contains("UTG"));
        assert!(output.contains("BB"));
    }

    #[test]
    fn test_all_positions_shown() {
        let output = render_table(9, None);
        assert!(output.contains("UTG"));
        assert!(output.contains("UTG+1"));
        assert!(output.contains("UTG+2"));
        assert!(output.contains("MP"));
        assert!(output.contains("HJ"));
        assert!(output.contains("CO"));
        assert!(output.contains("BTN"));
        assert!(output.contains("SB"));
        assert!(output.contains("BB"));
    }
}
