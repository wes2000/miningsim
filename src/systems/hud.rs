use bevy::prelude::*;
use crate::components::{MoneyText, CurrentToolText};
use crate::economy::Money;
use crate::inventory::Inventory;
use crate::items::{ItemKind, OreKind, ALL_ITEMS};
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

pub fn setup_hud(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|root| {
            for item in ALL_ITEMS {
                spawn_item_row(root, item);
            }
            // Money row
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(4.0)),
                ..default()
            }).with_children(|row| {
                row.spawn((
                    Node {
                        width: Val::Px(16.0),
                        height: Val::Px(16.0),
                        margin: UiRect::right(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 0.9, 0.3)),  // coin yellow
                ));
                row.spawn((
                    Text::new("0c"),
                    TextFont { font_size: 18.0, ..default() },
                    MoneyText,
                ));
            });
            // Current tool row
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(4.0)),
                ..default()
            }).with_children(|row| {
                row.spawn((
                    Node {
                        width: Val::Px(16.0),
                        height: Val::Px(16.0),
                        margin: UiRect::right(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.6, 0.6, 0.9)),  // tool slot bg
                ));
                row.spawn((
                    Text::new("Shovel"),
                    TextFont { font_size: 18.0, ..default() },
                    CurrentToolText,
                ));
            });
        });
}

fn spawn_item_row(root: &mut ChildBuilder, item: ItemKind) {
    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        margin: UiRect::all(Val::Px(4.0)),
        ..default()
    }).with_children(|row| {
        row.spawn((
            Node {
                width: Val::Px(16.0),
                height: Val::Px(16.0),
                margin: UiRect::right(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(item_color(item)),
        ));
        row.spawn((
            Text::new("0"),
            TextFont { font_size: 18.0, ..default() },
            ItemCountText(item),
        ));
    });
}

pub fn update_hud_system(
    inv: Res<Inventory>,
    money: Res<Money>,
    owned: Res<OwnedTools>,
    mut item_q: Query<(&mut Text, &ItemCountText), (Without<MoneyText>, Without<CurrentToolText>)>,
    mut money_q: Query<&mut Text, (With<MoneyText>, Without<ItemCountText>, Without<CurrentToolText>)>,
    mut tool_q: Query<&mut Text, (With<CurrentToolText>, Without<ItemCountText>, Without<MoneyText>)>,
) {
    if inv.is_changed() {
        for (mut text, marker) in item_q.iter_mut() {
            **text = inv.get(marker.0).to_string();
        }
    }
    if money.is_changed() {
        if let Ok(mut text) = money_q.get_single_mut() {
            **text = format!("{}c", money.0);
        }
    }
    if owned.is_changed() {
        if let Ok(mut text) = tool_q.get_single_mut() {
            // Strongest owned tool name
            let strongest = [Tool::Dynamite, Tool::Jackhammer, Tool::Pickaxe, Tool::Shovel]
                .into_iter()
                .find(|t| owned.0.contains(t))
                .unwrap_or(Tool::Shovel);
            **text = current_tool_display_name(strongest).to_string();
        }
    }
}
