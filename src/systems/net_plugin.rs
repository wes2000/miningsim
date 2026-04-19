use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RepliconPlugins);
        app.add_plugins(RepliconRenetPlugins);
        // Replication registrations + event handlers added in subsequent tasks.
    }
}
