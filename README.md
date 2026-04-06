# Poker CLI

A terminal-based preflop study tool for Texas Hold'em. Run it alongside online play for real-time opening range advice based on your position and table size.

## Install

Requires Rust 1.85+.

```
cargo build --release
```

The binary will be at `target/release/poker`.

## Usage

```
cargo run
```

On startup you'll configure the number of players and your starting position. Then just deal cards and get recommendations.

### Example Session

```
$ poker
Poker CLI — Preflop Study Tool

How many players? (2-9): 6
Positions: UTG -> HJ -> CO -> BTN -> SB -> BB
Your position? btn

Ready! You're on the BTN (6 players). Type 'deal <c1> <c2>' to start.

BTN> deal 2h tc
  (table layout with your seat highlighted)
  Hand: 2h Tc  (T2o — Offsuit)
  Position: BTN (Button)
  Recommendation: FOLD

BTN> new
  Position advanced to SB (Small Blind).

SB> deal ah ks
  Hand: Ah Ks  (AKo — Offsuit broadway)
  Position: SB (Small Blind)
  Recommendation: 3-BET

SB> players 9
  Players set to 9.
  Position reset to UTG.

UTG> pos co
  Position set to CO (Cutoff).

CO>
```

## Commands

| Command | Description |
|---|---|
| `deal <c1> <c2>` | Deal hole cards, get preflop recommendation |
| `new` | Advance position and reset for the next hand |
| `players <2-9>` | Set number of players at the table |
| `pos <position>` | Set your current position |
| `help` | List all commands |
| `quit` / `exit` | Exit the program |

## Card Notation

Cards are rank + suit, case-insensitive:

- **Ranks:** `2 3 4 5 6 7 8 9 T J Q K A` (`10` is also accepted for `T`)
- **Suits:** `s` (spades) `h` (hearts) `d` (diamonds) `c` (clubs)

Examples: `As`, `td`, `2c`, `KH`, `10s`

## Positions

| Input | Position |
|---|---|
| `utg` | Under the Gun |
| `utg1` | UTG+1 |
| `utg2` | UTG+2 |
| `mp` | Middle Position |
| `hj` | Hijack |
| `co` | Cutoff |
| `btn` | Button |
| `sb` | Small Blind |
| `bb` | Big Blind |

Positions adjust automatically based on table size (2-9 players).

## Recommendations

The tool gives one of three recommendations based on your hand and position:

- **OPEN (raise)** — Raise to enter the pot. You're first in, so this is an open-raise, not a call.
- **FOLD** — Don't play this hand from this position.
- **3-BET** — Re-raise over an open. Reserved for premium hands.

These are GTO-approximate opening ranges for **No-Limit Hold'em cash games** at ~100bb stacks. They assume you're first to voluntarily enter the pot.
