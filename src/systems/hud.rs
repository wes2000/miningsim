use bevy::prelude::*;
use crate::grid::OreType;
use crate::inventory::Inventory;

#[derive(Component)]
pub struct OreCountText(pub OreType);

pub fn setup_hud(mut commands: Commands) {
    // NOTE: Bevy 0.15.x in this workspace does not expose the `children!` macro
    // used by the plan; we use `with_children` callbacks instead. Semantically
    // identical — just spawns each child under the parent.
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|root| {
            spawn_row(root, OreType::Copper, Color::srgb(0.85, 0.45, 0.20));
            spawn_row(root, OreType::Silver, Color::srgb(0.85, 0.85, 0.92));
            spawn_row(root, OreType::Gold,   Color::srgb(0.95, 0.78, 0.25));
        });
}

fn spawn_row(parent: &mut ChildBuilder, ore: OreType, color: Color) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            margin: UiRect::all(Val::Px(4.0)),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    margin: UiRect::right(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(color),
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
    mut q: Query<(&mut Text, &OreCountText)>,
) {
    if !inv.is_changed() { return; }
    for (mut text, marker) in q.iter_mut() {
        **text = inv.get(marker.0).to_string();
    }
}
