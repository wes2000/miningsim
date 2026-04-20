use bevy::prelude::*;
use crate::systems::{belt as belt_sys, belt_ui, camera, chunk_lifecycle, chunk_render, hud, ore_drop, player, setup, shop, shop_ui, smelter};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InputSet { ReadInput, ApplyInput }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum WorldSet { Collide, ChunkLifecycle, ChunkRender, Drops }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MachineSet {
    ShopProximity,
    SmelterProximity,
    BeltPickup,
    BeltSpillage,
    BeltTick,
    SmelterBeltIo,
    SmelterTick,
    ShopUi,
    SmelterUi,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UiSet { Hud, SaveLoad, Camera }

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        // Requires Res<NetMode> to be inserted before this plugin is added (see main.rs).
        app.add_systems(Startup, (
                setup::setup_world,
                hud::setup_top_right_hud,
                hud::spawn_inventory_popup,
                shop_ui::spawn_shop_ui,
                smelter::spawn_smelter_ui,
            ).chain())
            // Order matches M2's chained-tuple invariant:
            //   input -> collide -> machine interactions/UI -> drops -> chunks -> hud -> save_load -> camera.
            //   Drops fires BEFORE chunk lifecycle/render so a tile broken this frame
            //   has its drop already in inventory before the HUD reads it.
            .configure_sets(Update, (
                InputSet::ReadInput,
                InputSet::ApplyInput,
                WorldSet::Collide,
                MachineSet::ShopProximity,
                MachineSet::SmelterProximity,
                MachineSet::BeltPickup,
                MachineSet::BeltSpillage,
                MachineSet::BeltTick,
                MachineSet::SmelterBeltIo,
                MachineSet::SmelterTick,
                MachineSet::ShopUi,
                MachineSet::SmelterUi,
                WorldSet::Drops,
                WorldSet::ChunkLifecycle,
                WorldSet::ChunkRender,
                UiSet::Hud,
                UiSet::SaveLoad,
                UiSet::Camera,
            ).chain())
            // Note: split across multiple `add_systems` calls because Bevy's tuple
            // impls for `IntoSystemConfigs` only go up to 20 elements. Ordering is
            // preserved entirely by `configure_sets` above + per-system `.in_set(...)`.
            .add_systems(Update, (
                player::read_input_system.in_set(InputSet::ReadInput),
                player::apply_velocity_system.in_set(InputSet::ApplyInput),
                player::dig_input_system.in_set(InputSet::ApplyInput),
                player::collide_player_with_grid_system.in_set(WorldSet::Collide),
                shop::shop_interact_system.in_set(MachineSet::ShopProximity),
                shop::close_shop_on_walk_away_system.in_set(MachineSet::ShopProximity),
                smelter::smelter_interact_system.in_set(MachineSet::SmelterProximity),
                smelter::close_smelter_on_walk_away_system.in_set(MachineSet::SmelterProximity),
                smelter::smelter_tick_system.in_set(MachineSet::SmelterTick),
                shop_ui::sync_shop_visibility_system.in_set(MachineSet::ShopUi),
                shop_ui::update_shop_labels_system.in_set(MachineSet::ShopUi),
                shop_ui::handle_shop_buttons_system.in_set(MachineSet::ShopUi),
                smelter::sync_smelter_visibility_system.in_set(MachineSet::SmelterUi),
                smelter::update_smelter_panel_system.in_set(MachineSet::SmelterUi),
                smelter::handle_smelter_buttons_system.in_set(MachineSet::SmelterUi),
                ore_drop::ore_drop_system.in_set(WorldSet::Drops),
                chunk_lifecycle::chunk_lifecycle_system.in_set(WorldSet::ChunkLifecycle),
                chunk_render::chunk_remesh_system.in_set(WorldSet::ChunkRender),
            ))
            .add_systems(Update, (
                hud::update_money_text_system.in_set(UiSet::Hud),
                hud::update_inventory_popup_system.in_set(UiSet::Hud),
                hud::toggle_inventory_popup_system.in_set(UiSet::Hud),
                hud::sync_inventory_popup_visibility_system.in_set(UiSet::Hud),
                camera::camera_follow_system.in_set(UiSet::Camera),
            ))
            // Build-mode UI (M5a Task 3). Local per-peer state; not gated on
            // NetMode here — placement/removal branch internally on Client mode
            // to fire request events instead of mutating directly (Task 10).
            .insert_resource(belt_sys::BeltTickTimer::default())
            .add_systems(Update, belt_sys::belt_tick_system.in_set(MachineSet::BeltTick))
            .insert_resource(belt_ui::BeltBuildMode::default())
            .add_systems(Update, (
                belt_ui::belt_build_toggle_system,
                belt_ui::belt_build_rotate_system,
                belt_ui::belt_ghost_render_system,
                belt_ui::belt_place_system,
                belt_ui::belt_remove_system,
                belt_ui::belt_visual_recompute_system,
            ).in_set(UiSet::Hud));

        // Mode-conditional plugin loading
        let net_mode = app.world().resource::<crate::net::NetMode>().clone();
        match net_mode {
            crate::net::NetMode::SinglePlayer => {
                app.add_plugins(crate::systems::save_load::SaveLoadPlugin);
            }
            crate::net::NetMode::Host { .. } | crate::net::NetMode::Client { .. } => {
                app.add_plugins(crate::systems::net_plugin::MultiplayerPlugin);
            }
        }
    }
}
