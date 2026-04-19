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

## Playtest Results — Milestone 2 (2026-04-18)

Exit-criteria met: tool progression Shovel → Pickaxe → Jackhammer →
Dynamite all unlock cleanly through the on-surface shop; tile damage
overlay reads correctly across the 1/2/3-strike gradient; Core (the new
deepest band) only breaks under Dynamite; bedrock boundary remains
unbreakable. Click-or-spacebar dig, sell-all + buy-tool loop, money +
current-tool HUD rows all functioning.

**What felt good:**
- "Barely scratch → tear through" lands. Each tool purchase produces a
  visible drop in clicks-per-tile across already-accessible layers, in
  addition to opening up the next layer. The progression arc reads.
- Auto-tool selection (no manual switching) keeps the controls identical
  to M1 — the player never has to think about which tool to wield.
- The shop UI's binary `Buy ... → OWNED` state is enough for M2; no need
  for confirmation dialogs or refund flows.

**What felt off (and was fixed mid-playtest):**
- Mouse-only dig made hold-to-dig feel awkward when the cursor drifted.
  **Added spacebar-dig that targets the tile in front of the player**
  (facing direction, snapped to dominant WASD axis). Mouse still aims
  with cursor; mouse wins if both are held.
- Walking vertically into a ceiling tile produced a sideways shove of
  ~12 px because the X-axis collision pass would fire for any tile that
  overlapped both axes. **Switched to minimum-translation-vector
  resolution** — each axis pass only resolves tiles whose overlap on
  that axis is the smaller one. Movement now feels clean in all
  directions.

**Decisions for milestone 3:**
- Save/load lands in M3 (per roadmap). Money, OwnedTools, Inventory, and
  the Grid (with damage state) all need to be serializable. The pure-data
  modules already make this trivial — Inventory and Money are HashMaps /
  scalars; Grid is a Vec<Tile>; OwnedTools is a HashSet<Tool>. Plan to
  use serde + ron or json; revisit when we get there.
- The five Bevy systems wired between dig and ore_drop are starting to
  add up — twelve in the Update chain now. M3 will add machine systems;
  consider grouping into named SystemSets (e.g. `InputSet`, `WorldSet`,
  `UiSet`) before the chain becomes unreadable.
- The HUD's per-row spawn code in `setup_hud` is mildly duplicated across
  ore-rows and the new Money / CurrentTool rows. A single helper
  `spawn_status_row(parent, swatch_color, label_text, marker)` would
  dedupe ~30 lines. Cosmetic; do when M3 adds more rows.
- `OreType::None` keeps appearing in match arms with `unreachable!()` /
  `Color::WHITE` fallbacks. The reviewer flagged this in M2's
  components review as a real type-smell — recommend revisiting in M3
  with `Tile { ore: Option<OreKind> }` so `OreSprite`/`OreDrop` cannot
  hold the sentinel variant. Touches grid, dig, inventory, terrain_gen,
  hud, player, chunk_render — sizeable refactor but mechanical.
- Dig SFX/VFX still missing (carry-over from M1 playtest notes). Tool
  upgrades amplify the felt absence — a pickaxe should sound different
  from a shovel. Pencil in for M7 polish unless it bothers us during M3.

## Playtest Results — Milestone 3 (2026-04-18)

Exit-criteria met: full **dig → smelt → collect → sell → upgrade → dig deeper**
loop works on the single property. Smelter takes raw ore, processes it on a
2-second-per-item timer with a queue + output buffer; player can walk away
mid-process. Bars sell at 5× ore prices. M2 cleanups (extract `coords::*`
helpers; affordability state on Shop Buy buttons) shipped as M3 Tasks 1–2 and
the `OreType::None` sentinel was retired in favour of `Option<OreKind>` plus a
clean `ItemKind { Ore(OreKind), Bar(OreKind) }` partition.

**What felt good:**
- The processing loop reads as a real factory step. "Drop in 5 copper, walk
  off to mine more, come back to a stack of bars" closes the gameplay loop
  the spec promised, and the 5× bar markup makes the trip feel worthwhile
  rather than busywork.
- The big atomic refactor (Task 4) landed without behavioural regressions
  thanks to the pure-data layer being well-isolated from Bevy systems.
  Swapping `Tile.ore` from `OreType` to `Option<OreKind>` and
  `Inventory<OreType>` to `Inventory<ItemKind>` was almost entirely
  mechanical once the test bodies were rewritten first.
- Named `SystemSet`s organise the now-23-system Update chain readably.
  `InputSet → WorldSet → MachineSet → UiSet` reads top-down like a frame
  pipeline.
- Coords helpers immediately paid for themselves — adding the Smelter
  entity took one `tile_center_world(IVec2::new(sp.0 - 3, sp.1))` call
  instead of four lines of inline Y-inverting math.

**What felt off (and was fixed mid-flight):**
- The HUD's six-item ore+bar list plus money + current-tool stack
  overflowed the top-left and pushed against game world. **Redesigned**:
  slim coin counter top-right (always visible), Tab toggles a popup
  showing `[ore swatch | count → bar swatch | count]` rows + a Tools
  section with active/owned/locked status per tool. Reads cleaner and
  scales when M6 adds processed-good families.

**Decisions for the next milestone:**
- **Save/load is the natural next mini-milestone** — focused, contained,
  unblocks netcode. All persistent state (`Grid`, `Inventory`, `Money`,
  `OwnedTools`, `SmelterState`) is plain Rust types in pure modules, so a
  serde derive + a single load/save system is most of the work. Should be
  a 4–5 task milestone, not a full M-sized effort.
- **M4 (networking) brings serialization too.** If save/load lands first,
  the netcode work inherits the serde wiring. If not, both can land
  together. Lean toward save/load first because it gives us a
  fast-iteration playtest tool (load a saved end-game state to validate
  M4 changes against a populated world).
- The smelter UI's panel layout uses the M2 shop pattern (Bevy `Node` +
  `with_children` + per-button `BackgroundColor` toggle for enabled
  state). It works but is verbose. M5 (conveyors) will spawn many more
  panel-style entities — worth investigating a small UI helper crate or
  in-house DSL if conveyor configuration UIs balloon.
- The smelter's `update_smelter_panel_system` "always refresh" approach
  (rebuilding label strings every frame while busy) is fine at our scale
  but a known scaling tax. If M5 produces dozens of running machines,
  consider gating per-machine refresh on the entity's `Changed<>` filter
  individually.
- Dig SFX/VFX remain unaddressed (carry-over from M1, M2 notes). Smelter
  is also silent — finishing a bar should have audible feedback. Group
  all audio into a single M7 polish task; not worth interleaving now.
- The 23-system Update chain is at the high end of comfortable. M5 will
  add at least conveyor-tick + per-conveyor render systems. Re-evaluate
  whether `MachineSet` should split into `MachineUpdate` / `MachineUi`
  sub-sets when that lands.

## Playtest Results — Save/Load Mini-Milestone (2026-04-18)

Exit-criteria met: F5 saves to `./save.ron`, F9 loads, closing the window
auto-saves, restarting the game loads any existing save automatically.
Mid-process smelter state (active recipe + queue + output buffer) survives
save/load. Hand-corrupting the save or bumping the `version` field falls
back gracefully to a fresh world without crashing. Schema version field
is `1`; future schema changes bump and silently discard old saves (no
migration logic — by design for now).

**What felt good:**
- The pure-module discipline from M1–M3 paid off. Adding `serde` derives
  to `Grid`, `Tile`, `Layer`, `OreKind`, `ItemKind`, `Inventory`, `Money`,
  `Tool`, `OwnedTools`, `SmelterState` was mechanical — every persistent
  type was already a plain Rust value type with no Bevy lifecycle
  surface to navigate.
- Hand-rolled `SaveData` struct + collect/apply was the right call over
  Bevy reflection. Pure-Rust round-trip tests cover the serialization
  layer headlessly without spinning up an `App`.
- `apply()` being idempotent (proven by a dedicated test) means F9-spam
  is safe and the function composes cleanly with Bevy's resource
  change-detection — every `Changed<...>` ripple fires exactly once per
  load.
- The startup-load path (`setup_world` always runs unconditionally → load
  overwrites if a save exists) means a missing or corrupt save can never
  block the game from starting. Defensive but cheap.
- 4-system shape (F5 / F9 / startup-load / AppExit) parallels the M2/M3
  shop+smelter UI pattern (interact + sync + handle), so the code reads
  consistently with the rest of the systems directory.

**What felt off (no fixes needed for this milestone):**
- The `auto_save_on_exit_system` correctly wires `EventReader<AppExit>`
  but in practice Bevy's window-close path emits the event one frame
  before final shutdown, which gives our system exactly one tick to
  serialize and write. Works fine at our save sizes (~80–250 KB) but
  worth re-examining if save sizes ever grow into the megabytes (M5
  conveyor automation, M6 multi-property).
- No save backup / `.bak` rotation. If a write is interrupted mid-flush
  the player loses both the prior save and the in-progress one. Standard
  game-jam-tier behaviour; revisit before any public build.

**Decisions for milestone 4 (networking):**
- `serde` derives now exist on every persistent type — `bevy_replicon`
  or `lightyear` should be able to consume them with minimal additional
  glue.
- `SaveData` doubles as a viable "snapshot" type for late-join sync in
  authoritative-server netcode. Worth comparing against `bevy_replicon`'s
  per-component replication model before committing.
- The 87-test pure-module suite is now the de facto contract for
  serialization; M4 can extend `tests/save.rs` with networking-specific
  scenarios (snapshot delta, replication conflict) without restructuring.
- `LoadError` enum (Io / Parse / VersionMismatch) is the obvious base for
  M4's "join failed" / "schema mismatch" / "host disconnected" error
  surface. Likely renamed to `WorldLoadError` and extended.

## Playtest Results — Milestone 4 (2026-04-19)

Exit-criteria met: 2-player direct-IP co-op via `bevy_replicon = "0.32"` +
`bevy_replicon_renet = "0.9"`. Authoritative-host model. Per-player
`Money`/`Inventory`/`OwnedTools` migrated from Resources to Components on
the Player entity, with `LocalPlayer`/`RemotePlayer` markers driving HUD
queries and sprite color (blue local, orange remote). Shared `Grid` and
`SmelterState` replicate from host to clients. Smoke-tested four launch
modes: `cargo run` (single-player + save/load), `cargo run -- host`,
`cargo run -- join 127.0.0.1:5000`, and `cargo run -- garbage`
(graceful fallback). Two-window co-op: movement, mining, ore drops,
per-player coin/inv/tool state, shared smelter, shop. Host-disconnect →
client logs error and emits `AppExit::Success` cleanly; client-disconnect
→ host despawns the gone player and continues.

`SAVE_VERSION` bumped 1 → 2 because internal collections moved from
`HashMap`/`HashSet` (non-deterministic iteration) to `BTreeMap`/`BTreeSet`
to satisfy replicon's diff-based replication. Old v1 saves are silently
discarded on load. `OreKind`/`ItemKind`/`Tool` variant order is now
load-bearing for replicated diff shape and is documented inline.

Test count: **101 passing** (87 pre-M4 + 9 net.rs CLI parser + 5
net_events serde round-trip).

**What felt good:**
- The phased structure (Phase A: Resource→Component refactor; Phase B:
  CLI + plugin selection; Phase C: replicon integration) kept gameplay
  playable after every Phase A commit. Smoke checkpoints after Tasks 4
  and 6 caught a class of regressions that would otherwise have surfaced
  only in the two-window test.
- `Single<&T, With<LocalPlayer>>` is the right idiom for "the local
  client's view of a per-player component." HUD/shop/smelter UI
  consumers all use it identically; the same code path serves
  single-player, host-mode-with-no-clients, and host-mode-with-clients
  without further branching.
- The `OwningClient(Entity)` (server-side only) + `NetOwner(u64)`
  (replicated) split was the correct response to replicon 0.32's "no
  `ClientId` type" reality. `OwningClient` carries a host-side `Entity`
  for request routing; `NetOwner` carries a renet `client_id` that
  survives wire serialization for client-side "is this Player mine?"
  identification.
- Branching the UI handlers on `NetMode::Client` (NOT
  `Host | Client`) keeps the host's path identical to single-player —
  the host has no `OwningClient` on its own Player, so its events would
  be silently dropped by `handle_*_requests`. Mutating directly avoids
  that and avoids a wasted serialization round-trip for the host's
  own actions.
- Pure modules (`grid`, `dig`, `economy`, `inventory`, `tools`,
  `processing`, `coords`, `save`, `net`, `net_events`) all stayed
  Bevy-system-free. Networking lives entirely in `systems/net_plugin.rs`
  and `systems/net_player.rs`. Pure-data tests still cover the
  authoritative gameplay logic — replicon just routes who calls them.

**What felt off (and was fixed mid-flight):**
- Plan pinned `bevy_replicon = "0.30"` + `bevy_replicon_renet = "0.5"`,
  but `0.5.x` of the transport adapter hard-depends on `bevy = "0.14"`
  and silently dragged a second copy of bevy into the build graph.
  Diagnosed via `cargo tree -i bevy`. Corrected to `0.32` + `0.9` and
  documented the verification command in the plan's resolved
  open-questions section so the next reader can avoid the same trap.
- The plan punted on `Grid` replication ("if replicon doesn't support
  resource replication, that's a small additional refactor"). The
  refactor was not small — `Grid` has six consumer files. We inserted
  Task 9.5 (Grid Resource → Component on a singleton entity with
  `Replicated` marker) between Tasks 9 and 10. Without it, host's dig
  mutations would not visually propagate to client peers. Replicon now
  ships a full Grid snapshot on every change (~16 KB at 80×200);
  documented as the bandwidth budget to revisit if the map grows.
- Task 10 attempted to treat `OwningClient(ClientId)` as a replicated
  marker for client-side player identification. Replicon 0.32 doesn't
  expose a `ClientId` type — connected clients are entities with a
  replicon-side `ConnectedClient` component. Task 12's implementer
  correctly split into `OwningClient(Entity)` (server-only routing) and
  `NetOwner(u64)` (replicated identification carrying the renet
  `client_id`).
- First Task 12 build of `start_net_mode_system` used
  `ConnectionConfig::default()`, which declares zero channels. Replicon
  needs the channels its registered components and events use; the
  default would have caused all replicated state to silently drop.
  Caught in code review before smoke test #3, fixed by deriving the
  config from `Res<RepliconChannels>` via
  `bevy_replicon_renet::RenetChannelsExt`.
- `HOST_NET_OWNER` was initially `0`, which would collide with any
  client whose renet `client_id` happened to be `0`. Switched to
  `u64::MAX` (well above the millis-derived range).

**What we deliberately deferred:**
- `Cargo.lock` is still gitignored. Convention for binary crates is to
  commit it; deferred per user call. Adds risk that multi-machine
  builds resolve different patch versions of replicon/renet.
- New joiners spawn at world `(0, 0)` rather than the host's spawn-tile
  helper. Visually fine but possibly inside terrain — they may need to
  dig out before being visible. Trivial to fix when the UX matters.
- The `Display` impl on `CliParseError` was never added — `main.rs`
  uses `{:?}` for the fallback error message. Output is readable
  (`UnknownCommand("garbage")`) but not user-facing-polished.

**Decisions for the next milestone:**
- **The bones are online-friendly.** `bevy_replicon` + `renet` are real
  UDP networking, not local-only. The 2-player cap is a config line in
  `setup_host` (`max_clients`); per-player components scale to N
  cleanly; the smelter UI/shop/HUD all already filter on
  `With<LocalPlayer>` so they automatically scale. The path to
  friction-free internet co-op is to swap `bevy_replicon_renet` for
  `bevy_replicon_steam` (Steam relay + lobbies + auth) or stand up a
  dedicated server with `bevy_replicon_quinnet`. The replicon API
  surface stays identical.
- **Trust model is pragmatic for friends, fragile for strangers.** Dig
  is server-validated (reach, line-of-sight, tool-tier). Buy/sell are
  host-mediated. Shared smelter is intentionally trust-based per the
  M4 spec — anyone can collect anyone's deposit. Acceptable for friend
  groups; would need per-deposit ownership tracking for stranger play.
- **Grid replication will not scale.** Full-snapshot replication is
  fine at 80×200 (~16 KB). At 200×500 (~100 KB) the per-frame
  bandwidth on dig-storms will become noticeable. The replacement is
  delta encoding (per-tile change events with `add_server_event`).
  Not blocking M5; flag for M6+.
- **Host-vs-client UI parity is a quiet win.** The same handler code
  serves both modes in single-player and Host (LocalPlayer mutation
  path). Only Client mode goes through the event system. Future UI
  systems can follow the same pattern without re-deriving the
  branching logic.
- **`OwningClient` + `NetOwner` is unusual but correct.** Document
  this pattern explicitly in CLAUDE.md or a short architecture note
  before M5 — the split would be easy for a future contributor to
  collapse into one component, breaking either server routing or
  client identification.

## What This Document Is Not

- Not a spec. Specs live in `docs/superpowers/specs/` and are per-milestone.
- Not a schedule. No dates, no hours estimates.
- Not frozen. Milestones can be re-ordered, merged, or split as playtesting
  reveals what actually matters. Update this file when that happens.
