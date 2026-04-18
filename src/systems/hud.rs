use bevy::prelude::*;
use crate::components::{MoneyText, CurrentToolText};
use crate::economy::Money;
use crate::grid::OreType;
use crate::inventory::Inventory;
use crate::tools::{OwnedTools, Tool};

#[derive(Component)]
pub struct OreCountText(pub OreType);

pub fn ore_visual_color(o: OreType) -> Color {
    match o {
        OreType::None   => Color::WHITE,
        OreType::Copper => Color::srgb(0.85, 0.45, 0.20),
        OreType::Silver => Color::srgb(0.85, 0.85, 0.92),
        OreType::Gold   => Color::srgb(0.95, 0.78, 0.25),
    }
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
            // Existing ore rows
            spawn_ore_row(root, OreType::Copper);
            spawn_ore_row(root, OreType::Silver);
            spawn_ore_row(root, OreType::Gold);
            // New: Money row
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
            // New: Current tool row
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

fn spawn_ore_row(root: &mut ChildBuilder, ore: OreType) {
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
            BackgroundColor(ore_visual_color(ore)),
        ));
        row.spawn((
            Text::new("0"),
            TextFont { font_size: 18.0, ..default() },
            OreCountText(ore),
        ));
    });
}

pub fn update_hud_system(
    inv: Res<Inventory>,
    money: Res<Money>,
    owned: Res<OwnedTools>,
    mut ore_q: Query<(&mut Text, &OreCountText), (Without<MoneyText>, Without<CurrentToolText>)>,
    mut money_q: Query<&mut Text, (With<MoneyText>, Without<OreCountText>, Without<CurrentToolText>)>,
    mut tool_q: Query<&mut Text, (With<CurrentToolText>, Without<OreCountText>, Without<MoneyText>)>,
) {
    if inv.is_changed() {
        for (mut text, marker) in ore_q.iter_mut() {
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
