use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;

use crate::components::Player;
use crate::economy::Money;
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
        // NOTE: Grid is intentionally NOT replicated here. It's a Resource (not a
        // Component), so `replicate::<Grid>()` won't compile, and converting it to a
        // Component would require touching every Res<Grid>/ResMut<Grid> consumer.
        // For now we rely on each peer regenerating the world from the same seed.
        // Tasks 10/12 may need to revisit this if dig-by-other-player must propagate
        // byte-for-byte (vs. via DigRequest events that mutate each peer's local Grid).
        app.replicate::<Player>()
            .replicate::<SmelterState>()
            .replicate::<Money>()
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
