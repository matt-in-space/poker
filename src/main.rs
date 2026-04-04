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

fn main() {
    println!("{}", "Poker CLI — Texas Hold'em Study Tool".bold());
    println!("Type 'help' for commands, 'quit' to exit.\n");

    let mut rl = DefaultEditor::new().expect("Failed to initialize readline");
    let mut state = HandState::new();

    loop {
        let prompt = match state.street {
            hand_state::Street::Preflop => "preflop> ",
            hand_state::Street::Flop => "flop> ",
            hand_state::Street::Turn => "turn> ",
            hand_state::Street::River => "river> ",
        };

        match rl.readline(prompt) {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(&line);

                match commands::execute(&mut state, &line) {
                    Ok(Some(output)) => println!("{output}"),
                    Ok(None) => {}
                    Err(e) => eprintln!("{} {e}", "Error:".red().bold()),
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
