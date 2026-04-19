use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;

use crate::components::Player;
use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::processing::SmelterState;
use crate::systems::net_events::{
    BuyToolRequest, CollectAllRequest, DigRequest, SellAllRequest, SmeltAllRequest,
};
use crate::tools::OwnedTools;

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RepliconPlugins);
        app.add_plugins(RepliconRenetPlugins);

        // Replicated components — host writes, all clients read.
        // Grid replicates as a Component on a singleton entity, spawned in
        // setup_world with Replicated marker (Task 9.5). Replication ships a
        // full Grid snapshot on every change (~MAP_W * MAP_H * sizeof(Tile)
        // bytes ≈ 16 KB at 80x200). Acceptable at current scale; revisit with
        // delta encoding if the map grows.
        app.replicate::<Player>()
            .replicate::<SmelterState>()
            .replicate::<Money>()
            .replicate::<Grid>()
            .replicate::<Inventory>()
            .replicate::<OwnedTools>()
            .replicate::<Transform>();

        // Client-fired events (client → server). 0.32 uses `Channel`, not `ChannelKind`.
        app.add_client_event::<DigRequest>(Channel::Ordered);
        app.add_client_event::<BuyToolRequest>(Channel::Ordered);
        app.add_client_event::<SmeltAllRequest>(Channel::Ordered);
        app.add_client_event::<CollectAllRequest>(Channel::Ordered);
        app.add_client_event::<SellAllRequest>(Channel::Ordered);

        // Mode-specific startup wiring lands in Task 12.
    }
}
