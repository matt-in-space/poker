use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

mod card;
mod commands;
mod error;
mod eval;
mod hand_state;
mod outs;
mod position;
mod pot;
mod preflop;
mod table_display;

use hand_state::HandState;
use position::Position;

fn main() {
    println!("{}", "Poker CLI — Preflop Study Tool".bold());
    println!();

    let mut rl = DefaultEditor::new().expect("Failed to initialize readline");
    let mut state = HandState::new();

    // Setup: ask for number of players
    loop {
        match rl.readline("How many players? (2-9): ") {
            Ok(line) => {
                let line = line.trim().to_string();
                if let Ok(n) = line.parse::<u8>() {
                    if (2..=9).contains(&n) {
                        state.num_players = n;
                        break;
                    }
                }
                println!("Enter a number between 2 and 9.");
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => return,
            Err(err) => {
                eprintln!("Error: {err}");
                return;
            }
        }
    }

    // Show position train
    let positions = position::positions_for_table_size(state.num_players);
    let train: Vec<&str> = positions.iter().map(|p| p.short_name()).collect();
    println!("Positions: {}", train.join(" -> "));

    // Ask for position
    loop {
        match rl.readline("Your position? ") {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                match Position::parse(&line) {
                    Ok(pos) => {
                        if state.set_position(pos) {
                            state.configured = true;
                            break;
                        } else {
                            println!(
                                "Position {} is not valid for a {}-player table.",
                                pos.short_name(),
                                state.num_players
                            );
                            println!("Valid positions: {}", train.join(", "));
                        }
                    }
                    Err(e) => eprintln!("{} {e}", "Error:".red().bold()),
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => return,
            Err(err) => {
                eprintln!("Error: {err}");
                return;
            }
        }
    }

    let pos = state.position().unwrap();
    println!(
        "\nReady! You're on the {} ({} players). Type 'deal <c1> <c2>' to start.\n",
        pos.short_name().bold(),
        state.num_players
    );

    // Main REPL
    loop {
        let pos_name = state
            .position()
            .map(|p| p.short_name().to_string())
            .unwrap_or_else(|| "?".to_string());
        let prompt = if state.street != hand_state::Street::Preflop {
            format!("{pos_name} {street}> ", street = state.street)
        } else {
            format!("{pos_name}> ")
        };

        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(&line);

                match commands::execute(&mut state, &line) {
                    Ok(Some(output)) => {
                        print!("\x1B[2J\x1B[H"); // clear screen, cursor to top
                        println!("{}", commands::format_status(&state).dimmed());
                        println!("{}", "─".repeat(60).dimmed());
                        println!("{output}");
                    }
                    Ok(None) => {}
                    Err(e) => {
                        print!("\x1B[2J\x1B[H");
                        println!("{}", commands::format_status(&state).dimmed());
                        println!("{}", "─".repeat(60).dimmed());
                        eprintln!("{} {e}", "Error:".red().bold());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Use 'quit' or 'exit' to leave.");
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("Error: {err}");
                break;
            }
        }
    }
}
