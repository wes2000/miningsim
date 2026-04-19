use bevy::prelude::*;
use crate::components::{LocalPlayer, ShopButtonKind, ShopUiOpen, ShopUiRoot};
use crate::economy::{self, Money};
use crate::inventory::Inventory;
use crate::tools::{OwnedTools, Tool};
use crate::systems::hud::current_tool_display_name;

pub fn spawn_shop_ui(mut commands: Commands) {
    commands
        .spawn((
            ShopUiRoot,
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
            // Title
            panel.spawn((
                Text::new("SHOP"),
                TextFont { font_size: 24.0, ..default() },
            ));
            // Sell All button
            spawn_button(panel, "Sell All Ore", ShopButtonKind::SellAll);
            // Divider text
            panel.spawn((
                Text::new("Tools:"),
                TextFont { font_size: 18.0, ..default() },
            ));
            // Buy Pickaxe / Jackhammer / Dynamite
            spawn_buy_row(panel, Tool::Pickaxe);
            spawn_buy_row(panel, Tool::Jackhammer);
            spawn_buy_row(panel, Tool::Dynamite);
        });
}

fn spawn_button(parent: &mut ChildBuilder, label: &str, kind: ShopButtonKind) {
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
    )).with_children(|button| {
        button.spawn((
            Text::new(label),
            TextFont { font_size: 18.0, ..default() },
        ));
    });
}

fn spawn_buy_row(parent: &mut ChildBuilder, tool: Tool) {
    let price = economy::tool_buy_price(tool);
    let label = format!("Buy {} - {}c", current_tool_display_name(tool), price);
    spawn_button(parent, &label, ShopButtonKind::Buy(tool));
}

pub fn sync_shop_visibility_system(
    ui_open: Res<ShopUiOpen>,
    mut q: Query<&mut Visibility, With<ShopUiRoot>>,
) {
    if !ui_open.is_changed() { return; }
    if let Ok(mut vis) = q.get_single_mut() {
        *vis = if ui_open.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}

pub fn update_shop_labels_system(
    local_player: Single<(Ref<Money>, Ref<OwnedTools>), With<LocalPlayer>>,
    buttons_q: Query<(&ShopButtonKind, &Children, Entity)>,
    mut bg_q: Query<&mut BackgroundColor>,
    mut texts_q: Query<&mut Text>,
) {
    let (money, owned) = local_player.into_inner();
    if !money.is_changed() && !owned.is_changed() { return; }
    for (kind, children, entity) in buttons_q.iter() {
        match kind {
            ShopButtonKind::SellAll => { /* static label, static color */ }
            ShopButtonKind::Buy(tool) => {
                let owned_already = owned.0.contains(tool);
                let price = economy::tool_buy_price(*tool);
                let affordable = money.0 >= price;

                let new_label = if owned_already {
                    format!("{} - OWNED", current_tool_display_name(*tool))
                } else {
                    format!("Buy {} - {}c", current_tool_display_name(*tool), price)
                };
                for c in children.iter() {
                    if let Ok(mut text) = texts_q.get_mut(*c) {
                        **text = new_label.clone();
                    }
                }

                // Background color signals interactability:
                //   normal (affordable, not owned) — slightly lit
                //   dimmed (broke or already owned) — darker
                let new_bg = if owned_already || !affordable {
                    Color::srgb(0.16, 0.16, 0.18)
                } else {
                    Color::srgb(0.22, 0.22, 0.28)
                };
                if let Ok(mut bg) = bg_q.get_mut(entity) {
                    *bg = BackgroundColor(new_bg);
                }
            }
        }
    }
}

pub fn handle_shop_buttons_system(
    ui_open: Res<ShopUiOpen>,
    interaction_q: Query<(&Interaction, &ShopButtonKind), Changed<Interaction>>,
    local_player: Single<(&mut Money, &mut Inventory, &mut OwnedTools), With<LocalPlayer>>,
) {
    // Defense-in-depth: Bevy does not deliver Interaction events for hidden UI,
    // but guard here in case system ordering changes or the UI is force-hidden
    // mid-frame.
    if !ui_open.0 { return; }
    let (mut money, mut inv, mut owned) = local_player.into_inner();
    for (interaction, kind) in interaction_q.iter() {
        if *interaction != Interaction::Pressed { continue; }
        match kind {
            ShopButtonKind::SellAll => {
                economy::sell_all(&mut inv, &mut money);
            }
            ShopButtonKind::Buy(tool) => {
                let _ = economy::try_buy(*tool, &mut money, &mut owned);
                // BuyResult::Ok / NotEnoughMoney / AlreadyOwned handled silently;
                // UI labels update via Changed<Money> / Changed<OwnedTools>.
            }
        }
    }
}
