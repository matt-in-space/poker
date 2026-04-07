# Poker CLI

A terminal-based study tool for Texas Hold'em. Run it alongside online play for real-time preflop range advice, postflop equity analysis, and bet sizing guidance.

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

On startup you'll configure the number of players, your starting position, and optionally the big blind amount for sizing guidance. Then deal cards and get recommendations.

### Example Session

```
$ poker
Poker CLI — Preflop Study Tool

How many players? (2-9): 6
Positions: UTG -> HJ -> CO -> BTN -> SB -> BB
Your position? btn
Big blind amount? (enter to skip): 20

Ready! You're on the BTN (6 players, BB 20). Type 'deal <c1> <c2>' to start.

BTN> dAhKsr60
  Hand: Ah Ks  (AKo — Offsuit broadway)
  Position: BTN (Button)  (facing raise: 60 = 3.0x BB)
  Recommendation: 3-BET
  Sizing: re-raise to 180 (9x BB)

BTN> new
  Position advanced to SB (Small Blind).

SB> d7h2d
  Hand: 7h 2d  (72o — Offsuit)
  Position: SB (Small Blind)
  Recommendation: FOLD

SB> new
BB> d9s4c
  Hand: 9s 4c  (94o — Offsuit)
  Position: BB (Big Blind)
  Recommendation: CHECK
  You're in the big blind — check and see a free flop.

BB> f9h4d2c
  Board: 9h 4d 2c  (Two pair, Nines and Fours)
  6 outs · ~24% equity

BB> odds
  Street: flop
  Hand: Two pair, Nines and Fours
  ...
```

### Compact Shortcuts

Most commands have single-letter shortcuts for fast input during play:

| Full command | Shortcut | Example |
|---|---|---|
| `deal Ah Ks` | `dAhKs` | |
| `deal Ah Ks raise 60` | `dAhKsr60` | |
| `deal 2h 8s limp` | `d2h8sl` | |
| `flop 2h 3s 4c` | `f2h3s4c` | |
| `turn 5d` | `t5d` | |
| `river 6h` | `r6h` | |
| `odds 25 50` | `b25p50` | bet 25 into pot of 50 |
| `new` | `n` | |

## Commands

### Preflop

| Command | Description |
|---|---|
| `deal <c1> <c2> [limp\|raise [amount]]` | Deal hole cards, get preflop recommendation |
| `new` | Advance position and reset for the next hand |
| `limp` | Re-evaluate as if facing a limp |
| `raise [amount]` | Re-evaluate as if facing a raise (optionally with the raise size) |
| `first` | Reset to first-in action |

### Postflop

| Command | Description |
|---|---|
| `flop <c1> <c2> <c3>` | Deal the flop — shows made hand, outs, and equity |
| `turn <card>` | Deal the turn |
| `river <card>` | Deal the river |
| `odds` | Show outs, equity, and bet suggestion for the current board |
| `odds <bet> <pot>` | Calculate pot odds for a specific bet and pot size |

### Configuration

| Command | Description |
|---|---|
| `players <2-9>` | Set number of players at the table |
| `pos <position>` | Set your current position |
| `blinds <amount>` | Set big blind for sizing guidance (e.g. `blinds 20`) |
| `ranges` | Info about the ranges and how they work |
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

### Preflop Actions

- **RAISE** — Open-raise to enter the pot (first in) or isolate a limper.
- **3-BET** — Re-raise over an open. Premium hands or bluff candidates (like A5s).
- **CALL** — Call a raise. Good enough to see a flop but not to re-raise.
- **CHECK** — Big blind with no raise to face. See a free flop.
- **FOLD** — Don't play this hand from this position.

### Bet Sizing

When blinds are set, recommendations include concrete sizing guidance:

- **Open:** 2.5–3x BB
- **3-bet:** 3x the raise (or 7–10x BB if no raise amount given)
- **Iso-raise:** 3–4x BB + 1 BB per limper
- **Call:** Shows cost in BB multiples

### Raise-Adjusted Ranges

When facing a raise with both blinds and raise amount set, ranges tighten based on the raise size:

| Raise Size | Call Range | Re-raise Range |
|---|---|---|
| Normal (up to 5x BB) | Opening range | 3-bet range |
| Large (5–15x BB) | 3-bet hands only | AA, KK |
| Huge (>15x BB) | AA, KK, QQ, AKs | — |

### About the Ranges

GTO-approximate opening ranges for **No-Limit Hold'em cash games** at ~100bb stacks. Based on Upswing Poker simplified charts, tightened for micro-stakes rake.

Key caveats:

- Ranges are simplified — real solvers use mixed strategies (e.g. open A9o from UTG 35% of the time). This tool gives a binary raise-or-fold.
- 3-bet ranges are polarized: premium hands for value + low suited aces (A5s–A2s) as bluffs. At micros, lean toward value since opponents call too much.
- Facing a raise, the right play depends on who raised and from where. This tool uses your position only.
