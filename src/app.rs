use bevy::prelude::*;
use crate::systems::{camera, chunk_lifecycle, chunk_render, hud, ore_drop, player, setup, shop, shop_ui, smelter};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InputSet { ReadInput, ApplyInput }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum WorldSet { Collide, ChunkLifecycle, ChunkRender, Drops }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MachineSet { ShopProximity, ShopUi, SmelterProximity, SmelterTick, SmelterUi }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UiSet { Hud, Camera }

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
                setup::setup_world,
                hud::setup_top_right_hud,
                hud::spawn_inventory_popup,
                shop_ui::spawn_shop_ui,
                smelter::spawn_smelter_ui,
            ).chain())
            // Order matches M2's chained-tuple invariant:
            //   input -> collide -> machine interactions/UI -> drops -> chunks -> hud -> camera.
            //   Drops fires BEFORE chunk lifecycle/render so a tile broken this frame
            //   has its drop already in inventory before the HUD reads it.
            .configure_sets(Update, (
                InputSet::ReadInput,
                InputSet::ApplyInput,
                WorldSet::Collide,
                MachineSet::ShopProximity,
                MachineSet::SmelterProximity,
                MachineSet::SmelterTick,
                MachineSet::ShopUi,
                MachineSet::SmelterUi,
                WorldSet::Drops,
                WorldSet::ChunkLifecycle,
                WorldSet::ChunkRender,
                UiSet::Hud,
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
            ));
    }
}
