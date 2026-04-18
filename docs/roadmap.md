# MiningSim — Project Roadmap

## Vision

A top-down 2D online co-op ore mining and factory sim built in Godot, taking
strong influence from *Ore Factory Squad*. Players dig procedurally generated
underground terrain, extract ores, process them through a factory, and fulfill
contracts for profit. Up to 4 players per session, with the game fully playable
solo.

## Guiding Principles

- **Fun-first, vertical slices.** Every milestone ends in something playable.
  We do not build engines of systems with nothing to play.
- **Singleplayer first, networking retrofitted.** Netcode is vastly easier to
  add to a working game than to design around from day one. Milestone 4 is a
  deliberate, load-bearing retrofit — earlier milestones should still be
  written with multiplayer in mind (deterministic-ish systems, clear
  ownership of state) but will not pay networking costs until then.
- **YAGNI.** No feature gets built before its milestone. No abstractions for
  hypothetical future needs. Each milestone is allowed to invalidate earlier
  design assumptions.
- **One milestone = one spec + one plan.** Each milestone goes through its
  own brainstorming → spec → writing-plans → implementation cycle. This
  roadmap is the compass, not a spec.
- **Playtest before polish.** Each milestone should be played (solo is fine)
  before moving to the next, so the next milestone's design is informed by
  reality.

## Target Platform & Tech

- **Engine:** [Bevy](https://bevyengine.org/) — code-first ECS engine. Plan
  authored against Bevy 0.15.x; engineer adapts to current stable.
- **Language:** Rust.
- **Perspective:** Top-down 2D.
- **Why this stack:** 100% code-driven (no GUI editor in the loop), strong
  type system catches a class of bugs at compile time, ECS is a natural
  model for the factory/conveyor systems coming in milestone 5, and the
  Bevy networking ecosystem (`bevy_replicon`, `lightyear`) is built around
  replicated game state — well-aligned with milestone 4.
- **Collision:** DIY tile-grid AABB checks for milestone 1 (no physics
  crate). A heavier physics dependency can be introduced later if a
  specific milestone needs it.
- **Tilemap & marching squares:** custom — written against the explicit
  Grid data structure rather than relying on a community crate, since the
  Grid is load-bearing for save/load and netcode later.
- **Networking target:** Up to 4 players, co-op only (no adversarial).
  Specific netcode crate (`bevy_replicon` is the leading candidate) and
  authority model decided in milestone 4's brainstorm.

## Milestone Sequence

Each milestone is its own project with its own spec. Do not start the next
one until the current one is playable and its lessons are absorbed.

### Milestone 1 — Core Dig Prototype (singleplayer)

**Goal:** Answer "is it fun to dig?"

- Single player, single small map.
- Procedurally generated destructible 2D underground terrain.
- One mining tool.
- Ores drop and land in a simple inventory.
- No factory, no shop, no networking, no progression.

**Exit criteria:** A player can load the game, dig tunnels, break ore tiles,
and see their inventory fill up. Digging feels satisfying moment-to-moment.

### Milestone 2 — Tool Progression + Tiered Ores

**Goal:** Give digging depth and pacing.

- Shovel → pickaxe → jackhammer → dynamite (or equivalent set).
- Ore hardness layers — deeper/harder tiles require better tools.
- Multiple ore types with different rarity/value.
- Minimal "get a better tool" loop (a crate, a debug button, or a placeholder
  shop — anything that lets tools be earned).

**Exit criteria:** The player feels forward motion from "I can barely scratch
this layer" to "I can tear through it."

### Milestone 3 — Surface Base + First Processing Loop

**Goal:** Close the end-to-end core gameplay loop on one property.

- Surface warehouse / base area above the dig area.
- One or two simple machines (e.g., crusher, smelter).
- Sell processed goods for money.
- Spend money on tool upgrades.
- Day/session structure if it helps pacing (optional).

**Exit criteria:** A full loop of dig → process → sell → upgrade → dig deeper
works on one property.

### Milestone 4 — Online Co-op Networking

**Goal:** Up to 4 players in one session, playing milestone 3's game
together.

- Retrofit multiplayer onto the working singleplayer game.
- Host/join flow (Steam/LAN/direct IP — decided at brainstorm time).
- Synchronized terrain destruction, player positions, inventories, machine
  state, money.
- Player customization deferred to milestone 7 — placeholder visuals are
  fine.

**Exit criteria:** Four players can join a session, dig together, share or
split resources per design, process together, and the session stays coherent
across disconnects/reconnects.

### Milestone 5 — Factory Automation

**Goal:** Factories, not just machines.

- Conveyor belts connecting machines.
- Multi-stage recipes.
- Pallets, possibly forklifts for in-base logistics.
- Warehouse robots (stretch, may be pushed to polish).

**Exit criteria:** A player can set up an unattended production line where
raw ore enters and a finished product exits.

### Milestone 6 — Property System + Contracts + Meta-Progression

**Goal:** The game becomes a business.

- Multiple property types (backyard, construction site, forest, plateau) with
  different ore distributions and sizes.
- Buying new properties with earned money.
- Contracts: specific product + quantity + reward, with loading/delivery.
- Licenses unlocking new machines and recipes.
- Market/stock selling alongside contracts.

**Exit criteria:** A player has a reason to own more than one property and
can choose between contracts vs. spot sales.

### Milestone 7 — Polish & Customization

**Goal:** The game feels finished.

- Character customization (outfits, faces, gloves, helmets).
- Warehouse customization (walls, floors, signs, decorations).
- Audio polish, VFX polish, UI polish.
- Tutorial / onboarding pass.
- Balance pass across the full progression curve.

**Exit criteria:** A new player can start the game, understand what to do,
and enjoy themselves without external guidance.

## Cross-Cutting Concerns (Decided Per Milestone, Not Up Front)

These get their first real answer when the milestone that needs them
arrives. Listing them here only so we don't forget they exist:

- **Save/load format.** Likely introduced in milestone 3.
- **Netcode architecture.** Decided in milestone 4.
- **World coordinate + chunking strategy.** Decided in milestone 1 but may
  be revisited when networking lands.
- **Art direction and asset pipeline.** Placeholder art until milestone 7;
  a deliberate art pass happens there.
- **Audio architecture.** Placeholder SFX until milestone 7.
- **Build / distribution pipeline.** Not addressed until a public build is
  actually wanted.

## Playtest Results — Milestone 1 (2026-04-18)

Exit-criteria met: game launches, terrain generates with visible layer bands,
WASD movement + collision work, click-and-spacebar dig both break tiles,
bedrock and out-of-reach clicks are rejected, ore drops vacuum into the HUD
inventory, the session runs without crashes.

**What felt good:**
- Blocky per-tile rendering is perfectly legible and reinforces the
  dig-one-tile-at-a-time feel. The marching-squares contour variant was
  implemented and tried; it made tunneling visually mushy and harder to
  navigate, so it was reverted. **Keep blocks for now** (see "Decisions for
  milestone 2").
- Click-per-hit with a 0.15 s cooldown reads as a clear "swing" per tile.
- Spacebar was added as an alternate dig trigger during playtest and felt
  natural for sustained digging.

**What felt off (and was fixed mid-playtest):**
- A Y-inversion bug in chunk-visibility math meant chunks around the player
  didn't spawn; fixed by normalizing chunk-space min/max componentwise.
- Diagonal tile clicks produced narrow staircase tunnels the player's AABB
  could get wedged on. **Restricted dig to cardinal directions only.**
- With cardinal-only at reach = 2, pushing into a wall let the player mine
  the tile *behind* the wall. **Added a line-of-sight check:** intermediate
  tiles between player and target must already be non-solid.

**Decisions for milestone 2:**
- Keep blocky per-tile rendering until a later polish pass. A future
  milestone can revisit marching squares (or a different smoothing
  approach) once tool tiers, SFX/VFX, and destruction juice give us more
  feedback signal to validate against.
- Tool tier pacing should cap clicks-per-tile at 2–3 across the weakest
  tool even on hardest rock — per the spec, to manage RSI given the
  click-per-hit choice.
- Dig VFX/SFX are the single biggest identified gap; swings currently
  feel slightly dry without a punchy hit sound or particle spray.
- Consider whether dig reach should remain 2 tiles or drop to 1. The
  line-of-sight gate makes reach = 2 meaningful only when mining into an
  already-open corridor; most practical digs are adjacent anyway.

## What This Document Is Not

- Not a spec. Specs live in `docs/superpowers/specs/` and are per-milestone.
- Not a schedule. No dates, no hours estimates.
- Not frozen. Milestones can be re-ordered, merged, or split as playtesting
  reveals what actually matters. Update this file when that happens.
