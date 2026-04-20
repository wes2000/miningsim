use bevy::prelude::*;
use bevy_replicon::prelude::FromClient;

use miningsim::components::{Facing, OwningClient, Player};
use miningsim::systems::net_events::ClientPositionUpdate;
use miningsim::systems::net_plugin::handle_client_position_updates;

#[test]
fn client_position_update_moves_player_even_without_facing() {
    let mut app = App::new();
    app.add_event::<FromClient<ClientPositionUpdate>>();
    app.add_systems(Update, handle_client_position_updates);

    let client = app.world_mut().spawn_empty().id();
    let player = app
        .world_mut()
        .spawn((
            Player,
            OwningClient(client),
            Transform::from_xyz(1.0, 2.0, 10.0),
        ))
        .id();

    app.world_mut()
        .resource_mut::<Events<FromClient<ClientPositionUpdate>>>()
        .send(FromClient {
            client_entity: client,
            event: ClientPositionUpdate {
                pos: Vec2::new(40.0, -24.0),
                facing: IVec2::new(1, 0),
            },
        });

    app.update();

    let xf = app.world().get::<Transform>(player).unwrap();
    assert_eq!(xf.translation, Vec3::new(40.0, -24.0, 10.0));
}

#[test]
fn client_position_update_refreshes_facing_when_present() {
    let mut app = App::new();
    app.add_event::<FromClient<ClientPositionUpdate>>();
    app.add_systems(Update, handle_client_position_updates);

    let client = app.world_mut().spawn_empty().id();
    let player = app
        .world_mut()
        .spawn((
            Player,
            OwningClient(client),
            Facing::default(),
            Transform::from_xyz(0.0, 0.0, 10.0),
        ))
        .id();

    app.world_mut()
        .resource_mut::<Events<FromClient<ClientPositionUpdate>>>()
        .send(FromClient {
            client_entity: client,
            event: ClientPositionUpdate {
                pos: Vec2::new(8.0, 16.0),
                facing: IVec2::new(-1, 0),
            },
        });

    app.update();

    let facing = app.world().get::<Facing>(player).unwrap();
    assert_eq!(facing.0, IVec2::new(-1, 0));
}
