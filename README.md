# Poker CLI

A terminal-based Texas Hold'em study tool. Run it in a second terminal while playing online for real-time preflop guidance, outs tracking, and pot odds calculation.

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

The tool is session-based. Start each hand with `deal`, feed in board cards as they appear, and get analysis at every street.

### Quick Example

```
preflop> deal Ah Kh btn 6
  (table layout with your seat highlighted)
  Hand: Ah Kh  (AKs - Suited broadway)
  Position: BTN (Button)
  Recommendation: 3-BET

preflop> flop 2h 5h 9c
  Board: 2h 5h 9c
  Made hand: High card (A)
    Flush draw (9 outs): 3h 4h 6h 7h 8h 9h Th Jh Qh
    Overcards (6 outs): Ks Kd Kc As Ad Ac
  Total outs: 15 (~60% equity, rule of 4)

flop> pot 200 50
  Call $50 into $200 pot -> need 20.0% equity
  Profitable call (~60% equity vs 20.0% needed)

flop> turn 3d
  Board: 2h 5h 9c 3d
  Made hand: High card (A)
    Flush draw (9 outs) + Gutshot (4 outs) + Overcards (6 outs)
  Total outs: 18 (~36% equity, rule of 2)

turn> river 8h
  Board: 2h 5h 9c 3d 8h
  Final hand: Flush

river> new
  Hand reset. Ready for a new hand.
```

## Commands

| Command | Description |
|---|---|
| `new` | Reset hand state for a new hand |
| `deal <c1> <c2> <pos> [players]` | Deal hole cards and set position |
| `flop <c1> <c2> <c3>` | Set the three flop cards |
| `turn <card>` | Set the turn card |
| `river <card>` | Set the river card |
| `pot <pot> <bet>` | Calculate pot odds (should I call?) |
| `pot <bet>` | Pot odds using the tracked pot total |
| `bet <amount> [pot_size]` | Analyze a bet you're considering |
| `table [players] [position]` | Show or update the table layout |
| `play <pos> <action> [amount]` | Record a player action (fold, check, call, raise, allin) |
| `pos <position>` | Update your position |
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

Positions adjust automatically based on table size (2-9 players). Positions that don't exist at the current table size are excluded.

## Guide

This section explains the poker concepts the tool uses. You can also access this in-app by typing `guide`.

### Streets

A Texas Hold'em hand has four stages, called **streets**:

1. **Preflop** - Each player gets two private cards (hole cards). A round of betting follows.
2. **Flop** - Three community cards are dealt face-up. Another round of betting.
3. **Turn** - A fourth community card is dealt. Another round of betting.
4. **River** - A fifth and final community card is dealt. Final round of betting.

Your best five-card hand is made from any combination of your two hole cards and the five community cards.

### Positions

Position is where you sit relative to the dealer button. It determines when you act in a betting round. Acting later is a huge advantage because you get to see what other players do first.

Positions from earliest (worst) to latest (best):

- **UTG (Under the Gun)** - First to act preflop. The worst position. Play tight.
- **UTG+1, UTG+2** - Still early position. Play tight.
- **MP (Middle Position)** - Can start to loosen up slightly.
- **HJ (Hijack)** - Two seats before the button. Ranges open up.
- **CO (Cutoff)** - One seat before the button. Very good position.
- **BTN (Button)** - The best position. Acts last on every postflop street. Play the widest range.
- **SB (Small Blind)** - Posts a forced half-bet. Acts first postflop. Tricky spot.
- **BB (Big Blind)** - Posts a forced full bet. Gets a discount to see flops.

At smaller tables (e.g. 6-max), early positions are removed but the relative advantage of later positions stays the same.

### Preflop Recommendations

When you `deal`, the tool gives one of three recommendations based on your hand and position:

- **OPEN** - Raise to enter the pot. This is the standard play when no one has raised before you.
- **FOLD** - Don't play this hand from this position. It's not profitable long-term.
- **3-BET** - Re-raise. The blinds are the 1st bet, an open-raise is the 2nd bet, and a re-raise is the **3rd bet** (hence "3-bet"). Reserved for premium hands or as a bluff with specific hands.

The recommendations are based on GTO-approximate opening ranges — what game theory suggests you should play from each position as a default. They assume you're first to voluntarily enter the pot.

**Important:** These ranges are designed for **No-Limit Hold'em cash games** with ~100 big blind stacks. They may not be accurate for:

- **Limit Hold'em** - Ranges are wider in fixed-limit because you can't be forced off a hand with a large bet.
- **Tournaments** - Stack depth, blind levels, and ICM pressure all shift ranges. Short-stacked play especially is very different (push/fold).
- **Deep-stacked play** (200bb+) - Speculative hands like suited connectors go up in value; dominated hands go down.

### Hand Categories

The tool classifies your two hole cards into categories to help build vocabulary:

- **Pocket pair** - Two cards of the same rank (e.g. 77, KK)
- **Suited broadway** - Two cards T or higher of the same suit (e.g. AKs, QJs)
- **Offsuit broadway** - Two cards T or higher of different suits (e.g. AKo, KJo)
- **Suited ace** - An ace with a lower card of the same suit (e.g. A5s)
- **Suited connector** - Two consecutive cards of the same suit (e.g. 87s, JTs)
- **Suited one-gapper** - Same suit, one rank apart (e.g. T8s, 97s)

The "s" suffix means suited (same suit), "o" means offsuit (different suits).

### Outs and Equity

An **out** is a card that will improve your hand to what's likely the best hand. The tool detects these draw types:

- **Flush draw** - You have four cards of the same suit and need one more. **9 outs.**
- **Open-ended straight draw (OESD)** - Four consecutive cards that can be completed on either end. **8 outs.**
- **Gutshot straight draw** - Four to a straight with a gap in the middle. **4 outs.**
- **Overcards** - Your hole cards are higher than anything on the board. Pairing one gives you top pair. **3 outs per overcard**, up to 6 total.

**Equity** is your chance of winning the hand. The tool estimates it with the **rule of 2 and 4**:

- On the **flop** (two cards to come): multiply outs by **4**
- On the **turn** (one card to come): multiply outs by **2**

Example: A flush draw (9 outs) on the flop gives roughly 9 x 4 = 36% equity.

This is an approximation. It's slightly optimistic for high out counts but accurate enough for in-game decisions.

When draws overlap (e.g. a card that completes both a flush and a straight), the tool lists each draw separately but **deduplicates** the total count so no card is counted twice.

### Pot Odds

**Pot odds** tell you the minimum equity you need to profitably call a bet. The formula is:

```
required equity = bet / (pot + bet)
```

Example: Someone bets $50 into a $200 pot. You need 50 / 250 = 20% equity to break even on a call.

If your estimated equity (from outs) exceeds the required equity, calling is profitable in the long run. If not, folding is usually correct unless you expect to win more on later streets (implied odds).

### Made Hands

The tool identifies your current hand strength after the flop, turn, and river:

| Hand | Description |
|---|---|
| High card | No pair or better |
| Pair (top/overpair/middle/bottom) | One pair, with position relative to the board |
| Two pair | Two different pairs |
| Three of a kind (set vs trips) | "Set" = pocket pair hit the board; "trips" = one hole card + board pair |
| Straight | Five consecutive ranks |
| Flush | Five cards of the same suit |
| Full house | Three of a kind + a pair |
| Four of a kind | Four cards of the same rank |
| Straight flush / Royal flush | Five consecutive cards of the same suit |

A **set** (pocket pair matching a board card) is much stronger than **trips** (one hole card matching a board pair) because it's better disguised.

## Features

- **Preflop recommendations** - Position-aware, table-size-aware opening range advice (OPEN, FOLD, or 3-BET) based on GTO-approximate charts
- **Hand classification** - Labels your hand (pocket pair, suited connector, offsuit broadway, etc.)
- **Outs calculation** - Lists outs by draw type (flush draw, OESD, gutshot, overcards) with deduplication
- **Equity estimation** - Rule of 4 on the flop, rule of 2 on the turn
- **Pot odds** - Compares required equity against your current estimated equity
- **Made hand detection** - Identifies your hand strength (top pair, overpair, set, flush, etc.)
- **Table visualizer** - ASCII table layout showing all seats and your position
- **Pot tracking** - Use `play` to record actions and track the pot automatically
