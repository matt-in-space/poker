# Poker CLI Tool — Product Spec

A terminal-based Texas Hold 'em study aid. Designed to be run in a second terminal window while playing online, giving real-time preflop guidance, outs tracking, and pot odds calculation as a hand progresses.

---

## Overview

The tool is session-based and stateful. You open it at the start of a session and feed it information as each hand plays out using short, fast commands. It maintains the current hand state — your cards, the board, the pot, and player actions — and updates its output accordingly.

The primary goal is to build intuition, not replace thinking. The tool shows its work: it lists outs explicitly, explains preflop recommendations by reference to position and hand strength, and gives you numbers you can verify against your own read.

---

## Commands

### `new`
Resets all hand state. Run this at the start of each new hand.

---

### `deal <card1> <card2> <position> [players]`
Sets your hole cards and position. Optionally sets the number of players at the table (defaults to the last set value, or 9 if never set).

**Examples:**
- `deal js 4c co`
- `deal ah kh btn 6`

**Output:**
- Displays the table layout (see Table Visualizer below) with your seat highlighted
- Shows a preflop recommendation: `OPEN`, `FOLD`, `CALL`, or `3-BET`
- Recommendation is position-aware and adjusts for table size (e.g. ranges open up at 6-max vs 9-handed)
- Shows the hand category (e.g. "suited connector", "offsuit broadway", "pocket pair")

---

### `flop <card1> <card2> <card3>`
Sets the three flop cards. Requires `deal` to have been run first.

**Output:**
- Displays the current board
- Lists your outs explicitly by card, grouped by draw type
  - e.g. `Flush draw (9 outs): 2s 4s 5s 6s 7s 8s 9s Ts Qs`
  - e.g. `Gutshot straight draw (4 outs): 7h 7d 7c 7s`
  - e.g. `Overcards (6 outs): Ah Ad Ac Kh Kd Kc`
  - Combo draws are listed separately but total is deduplicated
- Shows total out count and equity % using rule of 4 (two cards to come)
- Flags any made hands (e.g. "You have top pair" or "You flopped a flush")

---

### `turn <card>`
Sets the turn card.

**Output:**
- Updates the board display
- Recalculates outs (removes any outs that appeared on the board as dead cards)
- Updates equity % using rule of 2 (one card to come)
- Notes any changes to draw status (e.g. "Flush draw bricked, straight draw still live")

---

### `river <card>`
Sets the river card.

**Output:**
- Updates the board display
- Shows your final made hand and its strength
- No equity calculation needed — hand is complete
- Optionally notes what you were drawing to and whether you got there

---

### `pot <pot_size> <bet_size>`
Calculates pot odds at any point in the hand. Can be called independently of hand state.

**Output:**
- Required equity % to break even: `Call $50 into $200 pot → need 20% equity`
- If a hand is in progress: compares required equity against your current estimated equity
  - e.g. `Your flush draw gives ~36% equity — profitable call`
  - e.g. `Your gutshot gives ~17% equity — fold or look for other factors`

---

### `table [players] [dealer_position]`
Displays or updates the table layout without starting a new hand. Useful for adjusting number of players or dealer position mid-session.

**Examples:**
- `table 6` — set to 6 players
- `table 9 btn` — 9 players, you are on the button

---

### `play <position> <action> [amount]`
Records an action taken by another player. Used to track pot size automatically and optionally in a future version for range inference.

**Actions:** `fold`, `check`, `call`, `raise <amount>`, `allin`

**Examples:**
- `play utg raise 50`
- `play sb call`
- `play bb fold`

**Output:**
- Updates the running pot total
- Displays a brief action log for the current street
- Pot total is then available for `pot` calculations without manual input (just `pot <bet>`)

**Note:** Range inference from actions is a future feature. For now this is purely for pot tracking and record-keeping.

---

### `pos <position>`
Updates your position without resetting hand state. Useful if you mistyped.

---

### `help`
Lists all commands with brief descriptions.

---

### `quit` / `exit`
Exits the program.

---

## Card Notation

Cards are entered as rank + suit, case-insensitive, no space.

**Ranks:** `2 3 4 5 6 7 8 9 T J Q K A`  
**Suits:** `s` (spades), `h` (hearts), `d` (diamonds), `c` (clubs)

**Examples:** `As`, `td`, `2c`, `KH`, `jS`

The parser should be lenient — `10s` should be accepted as an alias for `Ts`.

---

## Position Notation

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

Positions that don't exist at the current table size are automatically excluded. For example, at a 6-max table there is no UTG+2 or MP.

---

## Table Visualizer

Displayed after `deal` and `table` commands. An ASCII oval showing all seats, labeled by position. The dealer button (`D`) is shown at the BTN seat. Your seat is highlighted (e.g. with `[ ]` vs `( )` brackets or an arrow).

**Example — 9 players, you are in the CO:**

```
            [ UTG ]
     [ BB ]          [ UTG+1 ]
  [ SB ]                  [ UTG+2 ]
     [D:BTN]          [ MP ]
            ( CO )  [ HJ ]
               ^-- you
```

**Example — 6 players, you are on the BTN:**

```
          [ UTG ]
   [ BB ]          [ UTG+1 ]
  [ SB ]                [ MP ]
         (D:BTN)   [ CO ]
            ^-- you
```

The layout should scale cleanly to 2–9 players, removing positions that don't exist rather than leaving empty seats.

---

## Preflop Recommendations

Recommendations are sourced from standard range charts (e.g. GTO-approximate opening ranges) and are position- and table-size-aware.

Each recommendation is one of:

- **OPEN** — raise to standard sizing (first in)
- **FOLD** — don't play this hand from this position
- **CALL** — only applies in BB facing a raise, or in specific spots
- **3-BET** — re-raise; shown for premium hands facing an open

The tool does not yet account for what other players have done preflop (that's for the `play` command future range feature). It gives the default first-in recommendation for the position.

The hand is also labeled by type to build vocabulary:
- Pocket pair (`77`)
- Suited connectors (`87s`)
- Suited broadway (`AQs`)
- Offsuit broadway (`KJo`)
- Suited ace (`A5s`)
- Offsuit connector (`76o`)
- etc.

---

## Outs Calculation

On the flop and turn, the tool enumerates draws and calculates outs with the following logic:

- **Flush draw:** 9 outs (4 if backdoor on flop)
- **Open-ended straight draw (OESD):** 8 outs
- **Gutshot straight draw:** 4 outs
- **Two overcards:** up to 6 outs (3 per overcard rank)
- **One overcard:** up to 3 outs
- **Combo draws:** outs are listed per draw type, then deduplicated for the total
- **Made hands:** flagged clearly; no outs needed

Outs that appear on the board are removed. The equity estimate uses:
- **Flop:** outs × 4
- **Turn:** outs × 2

This is an approximation (rule of 2 and 4). The tool should note it's an estimate, not exact.

---

## State Model

The tool holds the following state for the current hand:

- Your hole cards (2 cards)
- Your position
- Number of players
- Flop cards (0–3)
- Turn card (0–1)
- River card (0–1)
- Running pot total (updated by `play` commands or manual `pot` input)
- Action log for current street

State persists until `new` is run.

---

## Error Handling

The tool should handle bad input gracefully:

- Invalid card notation → `"Unknown card: '1s' — did you mean 'As'?"`
- Duplicate card (same card entered twice) → `"Card Jh is already in play"`
- Command run out of order (e.g. `turn` before `flop`) → `"No flop set yet — use 'flop' first"`
- Unknown position → `"Unknown position 'co2' — try 'co' or 'hj'"`
- Wrong number of arguments → show usage for that command

---

## Future Features (Out of Scope for v1)

- **Range inference from actions:** use `play` command history to narrow opponent ranges and refine equity estimates
- **Quiz mode:** randomly deal a hand + position, prompt for action, reveal correct answer and explanation
- **Session stats:** track how many preflop decisions you got right in quiz mode
- **Hand history log:** write completed hands to a file for later review
