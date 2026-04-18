# Milestone 2 — Tool Progression + Tiered Ores (Design Spec)

**Date:** 2026-04-18
**Status:** Draft (awaiting spec review)
**Parent roadmap:** [../../roadmap.md](../../roadmap.md)
**Prior milestone:** [2026-04-18-milestone-1-core-dig-prototype-design.md](./2026-04-18-milestone-1-core-dig-prototype-design.md)

## Purpose

Turn the M1 "dig any tile in one click" prototype into the tool-progression
arc the roadmap promises: **I can barely scratch this layer → I can tear
through it.** Each layer deeper requires a better tool; better tools earn
faster mining across layers already unlocked. A surface shop closes the
loop — sell ore for coins, buy the next tool, descend farther.

## Scope

### In scope

- Four tools, forming a linear progression: **Shovel → Pickaxe → Jackhammer → Dynamite**.
- Four breakable tile tiers mapped 1:1 to tools: **Dirt → Stone → Deep → Core**.
- Map-boundary bedrock remains unbreakable (`Layer::Bedrock` reserved for the outermost ring).
- Tier-gate + graduated click counts:
  - Under-tier tool → no damage (clunk SFX, dig aborts).
  - At-tier tool → 3 strikes to break.
  - +1-tier tool → 2 strikes.
  - +2-or-more-tier tool → 1 strike.
- Per-tile damage state persists between strikes; partially damaged tiles show a semi-transparent dark overlay proportional to damage.
- Auto-tool selection: each dig picks the player's strongest owned tool that can break the target layer; no manual tool switching UI.
- Money resource with a HUD coin counter.
- Surface shop entity with a Bevy UI panel: Sell All Ore + three tool-buy buttons.
- Ore sell prices: copper=1, silver=5, gold=20 coins.
- Tool buy prices: Shovel=0 (owned at start), Pickaxe=30, Jackhammer=100, Dynamite=300.

### Out of scope (deferred)

- Save/load (M3).
- Processing/machines/recipes (M3).
- Networking (M4).
- Conveyors, pallets, forklifts (M5).
- Multiple properties, contracts, licenses (M6).
- Character/warehouse customization, art pass, audio polish (M7).
- Natural caves, cellular-automata carving (candidate for M3 or later).
- AoE dynamite (Dynamite in M2 is a stronger single-tile tool; real AoE is a later polish decision).
- Tool manual-switching UI.
- Shop keyboard/gamepad navigation (mouse only).
- Per-ore sell buttons (Sell All only).

### Explicitly not designed for

- Persistent tool/money state across sessions (see save/load above).
- Scaling prices or shop inventory per-property (M6 concern).
- Inventory capacity limits (still unlimited flat list).

## Target platform & tech

Unchanged from M1:

- **Engine:** Bevy 0.15.x (pinned). Rust stable.
- **Perspective:** Top-down 2D.
- **Platforms:** Desktop (Windows / macOS / Linux), single player.
- **Art:** Placeholder — flat-color sprites, semi-transparent damage overlays. Art pass is M7.

## Key design decisions

| Decision | Choice | Why |
|---|---|---|
| Tool-vs-hardness interaction | Tier gate + graduated clicks (option C from brainstorm) | Delivers the "barely scratch → tear through" arc with a natural progression wall. Gate enforces buy-the-next-tool loop; gradient rewards upgrading. |
| Tool/tier count | 4 tools, 4 diggable tiers (option B) | Symmetric, delivers the full roadmap vision. Bedrock splits into "Core" (deepest diggable band) and map-boundary "Bedrock" (unbreakable). |
| Progression source | Shop on the surface, buy with coins earned from selling ore | User preference over buried crates or ore-threshold auto-unlock. Introduces the money system M3 was going to need anyway. |
| Selling UX | One "Sell All" button, no per-ore selling | Minimal UI, minimal decisions. Per-ore selling is M6 contract-tuning territory. |
| Tool selection | Auto-select strongest applicable per strike | No manual switching needed when tools upgrade monotonically; keeps controls identical to M1. |
| Damage visibility | Yes — semi-transparent dark overlay per damage point (α = damage × 0.2) | Playtest feedback value; cheap to render. Invisible damage would obscure the progression feel. |
| Architecture fit | Hybrid (option C from brainstorm) | Extend `Tile` and `Layer` in place (tile-level data); put new concepts (`tools`, `economy`, shop systems) in new modules. Keeps pure-module boundary intact. |

## Architecture

### Module / file layout

```
src/
  grid.rs                          # MODIFY: Tile gains `damage: u8`; Layer gains `Core`, keeps `Bedrock`
  dig.rs                           # MODIFY: try_dig takes tool; new DigStatus variants; new pure helper dig_target_valid
  inventory.rs                     # unchanged
  terrain_gen.rs                   # MODIFY: deepest band writes Layer::Core; boundary ring writes Bedrock
  tools.rs                         # NEW: Tool enum, OwnedTools resource, clicks_required, best_applicable_tool
  economy.rs                       # NEW: Money resource, ore_sell_price, tool_buy_price, sell_all, try_buy
  components.rs                    # MODIFY: add Shop, ShopUiRoot, ShopButtonKind components; ShopUiOpen resource
  systems/
    setup.rs                       # MODIFY: spawn Shop entity + Money + OwnedTools + ShopUiOpen resources
    player.rs                      # MODIFY: dig_input_system uses dig_target_valid + tools::best_applicable_tool
    shop.rs                        # NEW: shop_interact_system, close_shop_on_walk_away_system
    shop_ui.rs                     # NEW: spawn_shop_ui, sync_shop_visibility_system, update_shop_labels_system, handle_shop_buttons_system
    hud.rs                         # MODIFY: add Money row + CurrentTool row; extract ore_visual_color helper
    chunk_render.rs                # MODIFY: spawn damage-overlay sprite when tile.damage > 0
  app.rs                           # MODIFY: register new resources + systems
  lib.rs                           # MODIFY: pub mod tools; pub mod economy;
tests/
  grid.rs                          # MODIFY: cover Core variant + damage field round-trip
  inventory.rs                     # unchanged
  terrain_gen.rs                   # MODIFY: update depth-layers test; assert Core/Bedrock split
  dig.rs                           # MODIFY: tool-aware tests; add dig_target_valid coverage
  tools.rs                         # NEW
  economy.rs                       # NEW
```

### Module boundary

Pure modules (no Bevy systems/components): `grid`, `dig`, `inventory`, `tools`, `economy`, `terrain_gen`. Each is unit-testable headless. Bevy Resources may derive `Resource` but carry no ECS logic.

Dependency direction: `main → app → systems → pure modules`. No upward reach.

## Components / modules in detail

### `grid.rs` (modified)

```rust
pub enum Layer { Dirt, Stone, Deep, Core, Bedrock }   // Core is new; Bedrock unchanged but now semantically "boundary only"

pub struct Tile {
    pub solid: bool,
    pub layer: Layer,
    pub ore: OreType,
    pub damage: u8,                                    // NEW: strikes accumulated; 0 on fresh / broken tile
}
```

`Default` for `Tile` unchanged except `damage: 0`.

### `tools.rs` (new, pure)

```rust
pub enum Tool { Shovel, Pickaxe, Jackhammer, Dynamite }

pub fn tool_tier(t: Tool) -> u8 { /* Shovel=1, Pickaxe=2, Jackhammer=3, Dynamite=4 */ }

pub fn layer_tier(l: Layer) -> Option<u8> {
    // Some(1..=4) for Dirt/Stone/Deep/Core; None for Bedrock (unbreakable).
}

pub fn clicks_required(tool: Tool, layer: Layer) -> Option<u8> {
    // None if layer is Bedrock, or if tool_tier < layer_tier.
    // Else: gap = tool_tier - layer_tier; Some(3 - gap.min(2)).
}

#[derive(Resource)]
pub struct OwnedTools(pub HashSet<Tool>);
// Default: just Shovel.

pub fn best_applicable_tool(owned: &OwnedTools, layer: Layer) -> Option<Tool> {
    // Highest-tier tool in `owned` that has clicks_required(t, layer) = Some(_).
}
```

### `economy.rs` (new, pure)

```rust
#[derive(Resource, Default)]
pub struct Money(pub u32);

pub fn ore_sell_price(o: OreType) -> u32 {
    // None=0, Copper=1, Silver=5, Gold=20.
}

pub fn tool_buy_price(t: Tool) -> u32 {
    // Shovel=0, Pickaxe=30, Jackhammer=100, Dynamite=300.
}

pub fn sell_all(inv: &mut Inventory, money: &mut Money) {
    // Sum counts * prices, add to money, zero every ore count.
}

pub enum BuyResult { Ok, AlreadyOwned, NotEnoughMoney }

pub fn try_buy(tool: Tool, money: &mut Money, owned: &mut OwnedTools) -> BuyResult {
    // Check already-owned → AlreadyOwned.
    // Check money.0 < price → NotEnoughMoney.
    // Else: deduct price, insert tool, return Ok.
}
```

### `dig.rs` (modified, pure)

```rust
pub enum DigStatus {
    Broken,           // tile broke this strike
    Damaged,          // tile took damage but didn't break
    OutOfBounds,
    AlreadyEmpty,
    Blocked,          // line-of-sight / cardinal-only violation
    UnderTier,        // no tool of sufficient tier, OR tile is Bedrock
}

pub struct DigResult {
    pub status: DigStatus,
    pub ore: OreType,  // only meaningful on Broken; None otherwise
}

pub fn try_dig(grid: &mut Grid, tile: IVec2, tool: Tool) -> DigResult {
    // Validate bounds / solidity / layer-unbreakable / tool-tier adequacy.
    // Increment tile.damage. If damage >= clicks_required → clear tile, return Broken with captured ore.
    // Else return Damaged.
}

pub fn dig_target_valid(player_tile: IVec2, target: IVec2, reach: i32, grid: &Grid) -> bool {
    // Cardinal-only: exactly one of delta.x / delta.y is zero AND the other is nonzero.
    // |delta| <= reach on the nonzero axis.
    // Line-of-sight: every tile STRICTLY between player and target is non-solid.
    // Returns true iff all three hold.
}
```

### `inventory.rs` (unchanged)

Flat ore count map; no money, no tools. M2 does not repurpose it.

### `components.rs` (modified)

Added:
- `#[derive(Component)] struct Shop;`
- `#[derive(Component)] struct ShopUiRoot;`
- `#[derive(Component)] enum ShopButtonKind { SellAll, Buy(Tool) }`
- `#[derive(Component)] struct MoneyText;` and `#[derive(Component)] struct CurrentToolText;` for HUD change-detection binding.
- `#[derive(Resource, Default)] struct ShopUiOpen(pub bool);`

### `systems/setup.rs` (modified)

- Inserts `Money::default()`, `OwnedTools::default()`, `ShopUiOpen::default()`.
- Spawns Shop entity (yellow placeholder sprite, ~16 px) at a fixed offset from the spawn tile on the surface strip (e.g. spawn_tile.x + 3, spawn_tile.y - 1 — laterally offset from the player pocket so the player doesn't spawn on top of the shop).
- Calls `shop_ui::spawn_shop_ui(commands)` to build the hidden panel.

### `systems/shop.rs` (new)

`shop_interact_system` — on `E` just-pressed, computes distance from player to Shop entity; if ≤ 2 tiles, toggles `ShopUiOpen.0`. On `Esc` just-pressed, forces `ShopUiOpen.0 = false`.

`close_shop_on_walk_away_system` — each frame, if `ShopUiOpen.0` and player distance to Shop > 2 tiles, set `ShopUiOpen.0 = false`.

### `systems/shop_ui.rs` (new)

- `spawn_shop_ui` (Startup, ordered `.after(hud::setup_hud)` via a shared `SystemSet` to guarantee stable spawn order of UI roots) — builds the panel as one `Node` root (`ShopUiRoot`) with `Visibility::Hidden`, children: money label, Sell All button, three tool rows (name + price + Buy button). Each button carries a `ShopButtonKind` component for dispatch.
- `sync_shop_visibility_system` (Update) — only runs on `Changed<ShopUiOpen>`; mirrors bool to `Visibility`.
- `update_shop_labels_system` (Update) — runs on `Changed<Money> | Changed<OwnedTools>`; refreshes money text, button labels (`[Buy XX c]` → `[OWNED]`), and enables/disables button interactivity.
- `handle_shop_buttons_system` (Update) — walks `Query<(&Interaction, &ShopButtonKind), Changed<Interaction>>`; on `Pressed`, dispatches `economy::sell_all` or `economy::try_buy` accordingly. Does nothing when the UI is hidden (defense-in-depth: Bevy does not send Interaction events for hidden UI, but guard anyway).

### `systems/player.rs` (modified)

`dig_input_system` changes:
1. Compute `target_tile` from cursor as before.
2. Call `dig::dig_target_valid(player_tile, target_tile, DIG_REACH_TILES as i32, &grid)`; reject if false.
3. Read the tile's layer.
4. Compute `tool = tools::best_applicable_tool(&owned, layer)`; if None, play clunk SFX and return without consuming cooldown.
5. Call `dig::try_dig(&mut grid, target_tile, tool)`.
6. On `Broken`: mark chunk dirty, spawn OreDrop if ore present, reset cooldown.
7. On `Damaged`: mark chunk dirty (for damage overlay), reset cooldown.
8. On any failure: do not reset cooldown (identical to M1 anti-punish behavior).

### `systems/chunk_render.rs` (modified)

When rebuilding a dirty chunk, for each solid tile:
- (existing) spawn layer-color sprite.
- (existing) spawn ore dot if `tile.ore != None`.
- **NEW**: if `tile.damage > 0`, spawn a dark semi-transparent sprite (color: `Color::srgba(0.0, 0.0, 0.0, tile.damage as f32 * 0.2)`), same size as the tile, z slightly above the layer sprite. Max possible alpha is 0.4 (damage=2, just before break); damage=3 would break the tile and clear it, so the overlay never reaches higher.

### `systems/hud.rs` (modified)

- Extract `ore_visual_color(OreType) -> Color` into a shared helper reused by HUD, shop UI, chunk render, and ore drops (resolves M1 final-review advisory on scattered RGB constants).
- Add Money row with `MoneyText` marker and a coin-icon swatch.
- Add Current Tool row with `CurrentToolText` marker — shows the strongest tool the player owns.
- Update `update_hud_system` to refresh Money on `Changed<Money>` and Current Tool on `Changed<OwnedTools>`.

### `app.rs` (modified)

Register new resources: `Money`, `OwnedTools`, `ShopUiOpen`.
Register new systems in the Update chain after existing ones, roughly:
- shop_interact_system
- close_shop_on_walk_away_system
- sync_shop_visibility_system
- update_shop_labels_system
- handle_shop_buttons_system

`spawn_shop_ui` added to Startup set.

## Data flow

> **Cooldown rule (applies to all dig paths below):** the 0.15 s dig cooldown
> is reset only when `try_dig` returns `Broken` or `Damaged`. Every other
> outcome (under-tier, bedrock, out-of-bounds, already-empty, cardinal/LoS
> rejection) leaves the cooldown unchanged so failed clicks don't punish
> the player. Restated per-step below for clarity.

### Dig (tool-aware, damaged)
1. LMB/Space + cooldown finished → `dig_input_system`.
2. Compute `target_tile`. `dig::dig_target_valid` filters cardinal + reach + LoS.
3. Look up tile's layer.
4. `tools::best_applicable_tool(&owned, layer)` → `None` means clunk, abort (cooldown NOT consumed).
5. `dig::try_dig(grid, target, tool)`:
   - `clicks_required = clicks_required(tool, layer)` (1/2/3).
   - `tile.damage += 1`. If `>= clicks_required`, clear tile, return `Broken { ore: <prior ore> }`. Else return `Damaged`.
6. Broken or Damaged → mark owning chunk dirty (`insert ChunkDirty`). Reset cooldown.
7. Broken + ore → spawn OreDrop.

### Shop open/close
1. `E` just-pressed → `shop_interact_system` toggles `ShopUiOpen` if player within 2 tiles of Shop entity.
2. `Esc` just-pressed → force `ShopUiOpen = false`.
3. Walk-away → `close_shop_on_walk_away_system` forces `ShopUiOpen = false`.
4. `sync_shop_visibility_system` mirrors `ShopUiOpen` onto the panel's `Visibility`.

### Sell / Buy
1. `handle_shop_buttons_system` observes `Changed<Interaction>` on buttons.
2. On `Pressed` + `ShopButtonKind::SellAll` → `economy::sell_all(&mut inventory, &mut money)`.
3. On `Pressed` + `ShopButtonKind::Buy(tool)` → `economy::try_buy(tool, &mut money, &mut owned_tools)`.
4. Money / OwnedTools changes propagate via `Changed<>` filters to `update_shop_labels_system` (relabel Buy buttons), `update_hud_system` (refresh Money and Current Tool rows).

## Cross-cutting invariants

Unchanged from M1 plus:

1. **Grid is the single source of truth for tile state, including damage.** Nothing outside `dig::try_dig` and `terrain_gen::generate` mutates `Tile`.
2. **Pure modules stay pure.** `tools` and `economy` take only plain Rust types (plus the `Inventory`, `Money`, `OwnedTools` RefCounted-ish structs), not queries or commands.
3. **Dig is idempotent on non-solid tiles.** Both for `AlreadyEmpty` (M1 invariant preserved) and for the new `Damaged` state — re-striking continues accumulating damage deterministically.
4. **Tool auto-select is per-strike.** No cached "current tool" state on the Player entity; the strongest applicable tool is recomputed each strike. Immediately reflects new purchases.
5. **Shop UI state is a single Resource (`ShopUiOpen`).** Visibility is derived; no other code reads panel visibility directly.

## Edge cases & error handling

- **No tool can break targeted tile.** `best_applicable_tool` returns `None`. Clunk SFX, no dig, no cooldown reset.
- **Bedrock (map boundary) click.** `layer_tier(Bedrock) = None` → `best_applicable_tool` returns `None` regardless of owned tools. Same no-dig path as above.
- **Tool upgrade mid-mining.** Damage persists on the tile. Next strike uses the new (stronger) tool; if `clicks_required` drops at or below current damage, tile breaks instantly.
- **Partial damage + walk away.** Damage persists in the Grid. Returning to the tile continues accumulation. Intended, not a bug.
- **Out-of-bounds dig.** `OutOfBounds`, no-op.
- **Damage `u8` overflow.** Not reachable in practice (break happens at 1/2/3), but `u8` caps at 255; behavior is mathematically safe since the comparison is `>=`.
- **Shop open, player disconnects from range.** Auto-closes via `close_shop_on_walk_away_system`.
- **Buying already-owned tool.** `try_buy → AlreadyOwned`. UI row is `[OWNED]` and visually non-interactive; back-end no-ops as a defense.
- **Affording tool exactly.** `try_buy` uses `>=`; exact-cost purchase succeeds and leaves `Money = 0`.
- **Sell All with empty inventory.** Sums to zero, no state change. Harmless.
- **Shop placed near surface digging.** Shop Transform is fixed; terrain carving around it does not move it. A player could dig under the shop and leave it floating in a hole — cosmetic, M7 concern.
- **Integer overflow on money.** `u32` can hold ~4B; well above any plausible M2 inventory value. No guard needed.

### Explicitly not handled in M2

- Saving money / owned tools across sessions.
- Multiple shops (one per property arrives in M6).
- Per-ore sell prices scaling by market pressure / contracts (M6).
- Damage reset on tool change (not needed — damage is tool-agnostic).
- Race conditions on `try_buy` (single-player, single system).

## Testing approach

### Headless unit tests

- **`grid`** — add coverage for `Layer::Core`; `damage` default 0; round-trip `damage: u8` field through `set`/`get`.
- **`terrain_gen`** — **update** the M1 `depth_layers_appear_in_order` test: the deepest-band assertion must flip from `Bedrock` to `Core`, and a new assertion should confirm the outer ring (boundary) is `Bedrock`. No `Bedrock` in the interior below the surface strip, across Dirt/Stone/Deep/Core bands. This is a behavior-flip, not an additive test — the existing assertion as written will fail under M2 terrain generation.
- **`dig`** — extend with tool-aware tests and damage accumulation; add dedicated tests for `dig_target_valid` (cardinal, reach, LoS cases).
- **`tools`** — `tool_tier`, `layer_tier`, `clicks_required` full matrix, `best_applicable_tool` (None / strongest-wins / respects Bedrock).
- **`economy`** — `ore_sell_price`, `tool_buy_price`, `sell_all` (mixed inventory round-trip), `try_buy` (Ok / NotEnoughMoney / AlreadyOwned / exact-cost).
- **`inventory`** — unchanged; existing 4 tests still pass.

Target test count at end of M2: ~37 (existing 22 + ~15 new).

### Bevy systems are not unit-tested

Consistent with M1 policy. Visual / interactive behavior is validated by manual playtest.

### Manual playtest exit-criteria

- [ ] Game launches; yellow shop visible near spawn on the surface.
- [ ] Shovel on dirt: 3 strikes per tile; damage overlay visibly darkens over strikes.
- [ ] Shovel on stone: clunk, no damage.
- [ ] Shovel on bedrock: clunk, no damage.
- [ ] Walk to shop + press `E` → panel opens. `E` again, `Esc`, or walking away all close it.
- [ ] Sell All converts all ore to coins; HUD coin counter updates.
- [ ] Buy Pickaxe at 30c; pickaxe row shows `[OWNED]`; mining dirt now 2 strikes, stone now 3.
- [ ] Buy Jackhammer; dirt 1 strike, stone 2, deep 3.
- [ ] Buy Dynamite; Core at-tier → 3 strikes; Deep (gap +1) → 2; Stone (gap +2) → 1; Dirt (gap +3, capped) → 1. (Formula: `3 - (tool_tier - layer_tier).min(2)`.)
- [ ] Break through Core to map floor; Bedrock boundary ring remains unbreakable.
- [ ] Current-tool indicator updates when the strongest owned tool changes.
- [ ] Partial damage persists when walking away and returning.
- [ ] No crashes over a 20-minute session.
- [ ] "Barely scratch → tear through" feel is evident — each tool purchase feels like a qualitative power-up.

### Explicitly not tested

- Pixel-perfect damage overlay rendering (placeholder).
- SFX balance (placeholder).
- Exact price tuning — first-pass numbers, revisited in playtest notes recorded in `docs/roadmap.md` M2 section.

## Open questions deferred to implementation planning

- Exact shop UI panel dimensions and layout ordering. Starting guess: panel ~300×200 px, centered horizontally, vertically near the top.
- Whether `Shop` entity should be z-sorted above or below terrain. Starting guess: z = 5 (between chunk tile sprites at z=0 and player at z=10, so the player draws over the shop when overlapping; revise if it feels wrong).
- Whether the clunk SFX uses M1's existing placeholder or a dedicated new sound. M1 has no SFX — placeholder for M2 can be a brief console log or an optional Bevy `AudioPlayer` on a built-in sine wave, TBD at implementation time.
- Whether `OwnedTools::default()` starting with Shovel should be explicit (`vec![Shovel].into_iter().collect()`) vs. a named constructor. Stylistic; decide during implementation.

---

**Spec end.** Implementation plan to follow in a separate document via the writing-plans skill once this spec is approved.
