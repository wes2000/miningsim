use bevy::prelude::*;
use crate::components::{
    InventoryPopupOpen, InventoryPopupRoot, LocalPlayer, MoneyText, ToolRowText,
};
use crate::economy::Money;
use crate::inventory::Inventory;
use crate::items::{ItemKind, OreKind, ALL_ORES};
use crate::tools::{OwnedTools, Tool};

#[derive(Component)]
pub struct ItemCountText(pub ItemKind);

pub fn item_color(item: ItemKind) -> Color {
    match item {
        ItemKind::Ore(OreKind::Copper) => Color::srgb(0.85, 0.45, 0.20),
        ItemKind::Ore(OreKind::Silver) => Color::srgb(0.85, 0.85, 0.92),
        ItemKind::Ore(OreKind::Gold)   => Color::srgb(0.95, 0.78, 0.25),
        ItemKind::Bar(OreKind::Copper) => Color::srgb(0.95, 0.55, 0.30),
        ItemKind::Bar(OreKind::Silver) => Color::srgb(0.95, 0.95, 1.00),
        ItemKind::Bar(OreKind::Gold)   => Color::srgb(1.00, 0.88, 0.40),
    }
}

/// Color helper for raw ore tiles in the world (terrain rendering, drop sprites
/// when only the OreKind is known).
pub fn ore_visual_color(o: OreKind) -> Color {
    item_color(ItemKind::Ore(o))
}

pub fn current_tool_display_name(t: Tool) -> &'static str {
    match t {
        Tool::Shovel     => "Shovel",
        Tool::Pickaxe    => "Pickaxe",
        Tool::Jackhammer => "Jackhammer",
        Tool::Dynamite   => "Dynamite",
    }
}

const TOOL_ORDER: [Tool; 4] = [Tool::Shovel, Tool::Pickaxe, Tool::Jackhammer, Tool::Dynamite];

const COIN_COLOR: Color = Color::srgb(1.0, 0.9, 0.3);
const SWATCH_PX: f32 = 16.0;
const PANEL_BG: Color = Color::srgba(0.10, 0.10, 0.12, 0.92);

/// Top-right HUD: just the coin counter (always visible).
pub fn setup_top_right_hud(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            right: Val::Px(8.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(SWATCH_PX),
                    height: Val::Px(SWATCH_PX),
                    margin: UiRect::right(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(COIN_COLOR),
            ));
            row.spawn((
                Text::new("0c"),
                TextFont { font_size: 22.0, ..default() },
                MoneyText,
            ));
        });
}

/// Tab-toggled inventory popup. Shown in the top-center, hidden by default.
/// Layout per ore: [ore swatch | ore count] -> [bar swatch | bar count]
/// Plus a Tools section listing the four tools with owned/locked state.
pub fn spawn_inventory_popup(mut commands: Commands) {
    commands
        .spawn((
            InventoryPopupRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-160.0)),   // center horizontally (~320 px wide)
                width: Val::Px(320.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("INVENTORY  (Tab)"),
                TextFont { font_size: 18.0, ..default() },
            ));

            for ore in ALL_ORES {
                spawn_pair_row(panel, ore);
            }

            // Spacer
            panel.spawn(Node { height: Val::Px(8.0), ..default() });

            panel.spawn((
                Text::new("Tools"),
                TextFont { font_size: 16.0, ..default() },
            ));
            for tool in TOOL_ORDER {
                spawn_tool_row(panel, tool);
            }
        });
}

fn spawn_pair_row(parent: &mut ChildBuilder, ore: OreKind) {
    let ore_item = ItemKind::Ore(ore);
    let bar_item = ItemKind::Bar(ore);
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            // Ore side
            row.spawn((
                Node {
                    width: Val::Px(SWATCH_PX),
                    height: Val::Px(SWATCH_PX),
                    margin: UiRect::right(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(item_color(ore_item)),
            ));
            row.spawn((
                Node {
                    width: Val::Px(50.0),
                    ..default()
                },
            )).with_children(|cell| {
                cell.spawn((
                    Text::new("0"),
                    TextFont { font_size: 18.0, ..default() },
                    ItemCountText(ore_item),
                ));
            });
            // Arrow
            row.spawn((
                Text::new("→"),
                TextFont { font_size: 18.0, ..default() },
                Node { margin: UiRect::horizontal(Val::Px(8.0)), ..default() },
            ));
            // Bar side
            row.spawn((
                Node {
                    width: Val::Px(SWATCH_PX),
                    height: Val::Px(SWATCH_PX),
                    margin: UiRect::right(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(item_color(bar_item)),
            ));
            row.spawn((
                Node { width: Val::Px(50.0), ..default() },
            )).with_children(|cell| {
                cell.spawn((
                    Text::new("0"),
                    TextFont { font_size: 18.0, ..default() },
                    ItemCountText(bar_item),
                ));
            });
        });
}

fn spawn_tool_row(parent: &mut ChildBuilder, tool: Tool) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(current_tool_display_name(tool)),
                TextFont { font_size: 16.0, ..default() },
                ToolRowText(tool),
            ));
        });
}

/// Refreshes the top-right coin counter when Money changes.
pub fn update_money_text_system(
    money_q: Query<&Money, (With<LocalPlayer>, Changed<Money>)>,
    mut text_q: Query<&mut Text, With<MoneyText>>,
) {
    let Ok(money) = money_q.get_single() else { return };
    if let Ok(mut text) = text_q.get_single_mut() {
        **text = format!("{}c", money.0);
    }
}

/// Refreshes the popup's item counts when Inventory changes and the tools
/// section when OwnedTools changes. Runs cheaply if neither did.
pub fn update_inventory_popup_system(
    local_player: Single<(Ref<Inventory>, Ref<OwnedTools>), With<LocalPlayer>>,
    mut item_q: Query<(&mut Text, &ItemCountText), Without<ToolRowText>>,
    mut tool_q: Query<(&mut Text, &ToolRowText), Without<ItemCountText>>,
) {
    let (inv, owned) = local_player.into_inner();
    if inv.is_changed() {
        for (mut text, marker) in item_q.iter_mut() {
            **text = inv.get(marker.0).to_string();
        }
    }
    if owned.is_changed() {
        // Strongest owned tool — used to mark the active row.
        let strongest = TOOL_ORDER
            .iter()
            .copied()
            .rev()
            .find(|t| owned.0.contains(t))
            .unwrap_or(Tool::Shovel);
        for (mut text, marker) in tool_q.iter_mut() {
            let name = current_tool_display_name(marker.0);
            **text = if marker.0 == strongest {
                format!("> {} (active)", name)
            } else if owned.0.contains(&marker.0) {
                format!("  {}", name)
            } else {
                format!("  {} (locked)", name)
            };
        }
    }
}

/// Tab toggles the popup; Esc force-closes.
pub fn toggle_inventory_popup_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut open: ResMut<InventoryPopupOpen>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        open.0 = false;
        return;
    }
    if keys.just_pressed(KeyCode::Tab) {
        open.0 = !open.0;
    }
}

/// Mirrors InventoryPopupOpen onto the panel's Visibility on change.
pub fn sync_inventory_popup_visibility_system(
    open: Res<InventoryPopupOpen>,
    mut q: Query<&mut Visibility, With<InventoryPopupRoot>>,
) {
    if !open.is_changed() { return; }
    if let Ok(mut vis) = q.get_single_mut() {
        *vis = if open.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}
