use bevy::prelude::*;
use crate::components::{
    LocalPlayer, Smelter, SmelterButtonKind, SmelterStatusText, SmelterUiOpen, SmelterUiRoot,
};
use crate::coords::TILE_SIZE_PX;
use crate::inventory::Inventory;
use crate::items::{ItemKind, OreKind, ALL_ORES};
use crate::processing::{self, is_busy, SmelterState};
use crate::systems::net_events::{CollectAllRequest, SmeltAllRequest};

pub const SMELTER_INTERACT_RADIUS_TILES: f32 = 2.0;

pub fn smelter_interact_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_open: ResMut<SmelterUiOpen>,
    player_q: Query<&Transform, With<LocalPlayer>>,
    smelter_q: Query<&Transform, (With<Smelter>, Without<LocalPlayer>)>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        ui_open.0 = false;
        return;
    }
    if !keys.just_pressed(KeyCode::KeyE) { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(smelter) = smelter_q.get_single() else { return };
    let dist = player.translation.truncate().distance(smelter.translation.truncate());
    if dist / TILE_SIZE_PX <= SMELTER_INTERACT_RADIUS_TILES {
        ui_open.0 = !ui_open.0;
    }
}

pub fn close_smelter_on_walk_away_system(
    mut ui_open: ResMut<SmelterUiOpen>,
    player_q: Query<&Transform, With<LocalPlayer>>,
    smelter_q: Query<&Transform, (With<Smelter>, Without<LocalPlayer>)>,
) {
    if !ui_open.0 { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(smelter) = smelter_q.get_single() else { return };
    let dist = player.translation.truncate().distance(smelter.translation.truncate());
    if dist / TILE_SIZE_PX > SMELTER_INTERACT_RADIUS_TILES {
        ui_open.0 = false;
    }
}

pub fn smelter_tick_system(
    time: Res<Time>,
    mut q: Query<&mut SmelterState>,
) {
    let dt = time.delta_secs();
    for mut state in q.iter_mut() {
        let _ = processing::tick_smelter(&mut state, dt);
        // Event return value is unused for M3; M4 (events bus) or M7 (audio) may consume it.
    }
}

pub fn spawn_smelter_ui(mut commands: Commands) {
    commands
        .spawn((
            SmelterUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(30.0),
                top: Val::Percent(20.0),
                width: Val::Px(420.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.10, 0.12, 0.92)),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("SMELTER"),
                TextFont { font_size: 24.0, ..default() },
            ));
            // Status line — updated by update_smelter_panel_system
            panel.spawn((
                Text::new("IDLE"),
                TextFont { font_size: 18.0, ..default() },
                SmelterStatusText,
            ));
            // SmeltAll buttons — one per ore
            for ore in ALL_ORES {
                spawn_smelter_button(panel, &smelt_button_label(ore, 0), SmelterButtonKind::SmeltAll(ore));
            }
            // CollectAll
            spawn_smelter_button(panel, "Collect All", SmelterButtonKind::CollectAll);
        });
}

fn smelt_button_label(ore: OreKind, count: u32) -> String {
    format!("Smelt All {} ({})", ore_display(ore), count)
}

fn ore_display(o: OreKind) -> &'static str {
    match o {
        OreKind::Copper => "Copper",
        OreKind::Silver => "Silver",
        OreKind::Gold   => "Gold",
    }
}

fn spawn_smelter_button(parent: &mut ChildBuilder, label: &str, kind: SmelterButtonKind) {
    parent.spawn((
        kind,
        Button,
        Node {
            padding: UiRect::all(Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            width: Val::Px(280.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.22, 0.22, 0.28)),
        BorderColor(Color::srgb(0.35, 0.35, 0.42)),
    )).with_children(|b| {
        b.spawn((
            Text::new(label),
            TextFont { font_size: 18.0, ..default() },
        ));
    });
}

pub fn sync_smelter_visibility_system(
    ui_open: Res<SmelterUiOpen>,
    mut q: Query<&mut Visibility, With<SmelterUiRoot>>,
) {
    if !ui_open.is_changed() { return; }
    if let Ok(mut vis) = q.get_single_mut() {
        *vis = if ui_open.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}

pub fn update_smelter_panel_system(
    local_inv: Option<Single<&Inventory, With<LocalPlayer>>>,
    state_q: Query<&SmelterState>,
    status_q: Query<Entity, With<SmelterStatusText>>,
    buttons_q: Query<(&SmelterButtonKind, &Children, Entity)>,
    mut bg_q: Query<&mut BackgroundColor>,
    mut texts_q: Query<&mut Text>,
) {
    let Some(local_inv) = local_inv else { return };
    let Ok(state) = state_q.get_single() else { return };
    let inv = local_inv.into_inner();
    // Always refresh — SmelterState may have changed (tick mutates time_left every frame)
    // and inventory may have changed; combined gating is messy and the work is cheap.

    // Status line
    if let Ok(status_entity) = status_q.get_single() {
        if let Ok(mut text) = texts_q.get_mut(status_entity) {
            **text = match state.recipe {
                None => "IDLE".to_string(),
                Some(ore) => format!(
                    "Smelting {} Bar ({:.1}s, queue: {})",
                    ore_display(ore), state.time_left.max(0.0), state.queue
                ),
            };
        }
    }

    // Buttons
    for (kind, children, entity) in buttons_q.iter() {
        let (label, enabled) = match kind {
            SmelterButtonKind::SmeltAll(ore) => {
                let count = inv.get(ItemKind::Ore(*ore));
                let label = smelt_button_label(*ore, count);
                let enabled = count > 0 && !is_busy(state);
                (label, enabled)
            }
            SmelterButtonKind::CollectAll => {
                let total: u32 = state.output.values().sum();
                let label = format!("Collect All ({})", total);
                let enabled = total > 0;
                (label, enabled)
            }
        };
        // Update child Text label
        for c in children.iter() {
            if let Ok(mut text) = texts_q.get_mut(*c) {
                **text = label.clone();
            }
        }
        // Update background per enabled state
        let new_bg = if enabled {
            Color::srgb(0.22, 0.22, 0.28)
        } else {
            Color::srgb(0.16, 0.16, 0.18)
        };
        if let Ok(mut bg) = bg_q.get_mut(entity) {
            *bg = BackgroundColor(new_bg);
        }
    }
}

pub fn handle_smelter_buttons_system(
    ui_open: Res<SmelterUiOpen>,
    interaction_q: Query<(&Interaction, &SmelterButtonKind), Changed<Interaction>>,
    local_inv: Option<Single<&mut Inventory, With<LocalPlayer>>>,
    mut state_q: Query<&mut SmelterState>,
    net_mode: Res<crate::net::NetMode>,
    mut smelt_writer: EventWriter<SmeltAllRequest>,
    mut collect_writer: EventWriter<CollectAllRequest>,
) {
    if !ui_open.0 { return; }

    if matches!(*net_mode, crate::net::NetMode::Client { .. }) {
        // Client: fire request events for the host to validate. Local Inventory
        // and SmelterState borrows intentionally not used; the host's mutations
        // replicate back and drive UI refresh via Changed<…>.
        for (interaction, kind) in interaction_q.iter() {
            if *interaction != Interaction::Pressed { continue; }
            match kind {
                SmelterButtonKind::SmeltAll(ore) => { smelt_writer.send(SmeltAllRequest { ore: *ore }); }
                SmelterButtonKind::CollectAll => { collect_writer.send(CollectAllRequest); }
            }
        }
        return;
    }

    // SinglePlayer / Host: mutate local inventory + smelter directly.
    let Some(local_inv) = local_inv else { return };
    let Ok(mut state) = state_q.get_single_mut() else { return };
    let mut inv = local_inv.into_inner();
    for (interaction, kind) in interaction_q.iter() {
        if *interaction != Interaction::Pressed { continue; }
        match kind {
            SmelterButtonKind::SmeltAll(ore) => {
                let count = inv.get(ItemKind::Ore(*ore));
                if count == 0 || is_busy(&state) { continue; }
                inv.remove(ItemKind::Ore(*ore), count);
                processing::start_smelting(&mut state, *ore, count);
            }
            SmelterButtonKind::CollectAll => {
                let drained = processing::collect_output(&mut state);
                for (ore, n) in drained {
                    inv.add(ItemKind::Bar(ore), n);
                }
            }
        }
    }
}
