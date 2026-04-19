use std::collections::BTreeSet;
use miningsim::grid::Layer;
use miningsim::tools::{self, Tool, OwnedTools};

#[test]
fn tool_tiers_are_1_through_4() {
    assert_eq!(tools::tool_tier(Tool::Shovel), 1);
    assert_eq!(tools::tool_tier(Tool::Pickaxe), 2);
    assert_eq!(tools::tool_tier(Tool::Jackhammer), 3);
    assert_eq!(tools::tool_tier(Tool::Dynamite), 4);
}

#[test]
fn layer_tier_assigns_diggable_tiers_and_bedrock_is_none() {
    assert_eq!(tools::layer_tier(Layer::Dirt), Some(1));
    assert_eq!(tools::layer_tier(Layer::Stone), Some(2));
    assert_eq!(tools::layer_tier(Layer::Deep), Some(3));
    assert_eq!(tools::layer_tier(Layer::Core), Some(4));
    assert_eq!(tools::layer_tier(Layer::Bedrock), None);
}

#[test]
fn clicks_required_at_tier_is_three() {
    assert_eq!(tools::clicks_required(Tool::Shovel, Layer::Dirt), Some(3));
    assert_eq!(tools::clicks_required(Tool::Pickaxe, Layer::Stone), Some(3));
    assert_eq!(tools::clicks_required(Tool::Jackhammer, Layer::Deep), Some(3));
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Core), Some(3));
}

#[test]
fn clicks_required_one_above_tier_is_two() {
    assert_eq!(tools::clicks_required(Tool::Pickaxe, Layer::Dirt), Some(2));
    assert_eq!(tools::clicks_required(Tool::Jackhammer, Layer::Stone), Some(2));
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Deep), Some(2));
}

#[test]
fn clicks_required_two_or_more_above_tier_is_one() {
    assert_eq!(tools::clicks_required(Tool::Jackhammer, Layer::Dirt), Some(1));
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Stone), Some(1));
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Dirt), Some(1));
}

#[test]
fn clicks_required_under_tier_is_none() {
    assert_eq!(tools::clicks_required(Tool::Shovel, Layer::Stone), None);
    assert_eq!(tools::clicks_required(Tool::Pickaxe, Layer::Deep), None);
    assert_eq!(tools::clicks_required(Tool::Jackhammer, Layer::Core), None);
}

#[test]
fn clicks_required_bedrock_is_always_none() {
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Bedrock), None);
    assert_eq!(tools::clicks_required(Tool::Shovel, Layer::Bedrock), None);
}

#[test]
fn default_owned_tools_has_only_shovel() {
    let owned = OwnedTools::default();
    assert!(owned.0.contains(&Tool::Shovel));
    assert_eq!(owned.0.len(), 1);
}

#[test]
fn best_applicable_tool_picks_strongest() {
    let owned = OwnedTools(BTreeSet::from([Tool::Shovel, Tool::Pickaxe, Tool::Jackhammer]));
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Dirt), Some(Tool::Jackhammer));
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Stone), Some(Tool::Jackhammer));
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Deep), Some(Tool::Jackhammer));
}

#[test]
fn best_applicable_tool_returns_none_when_no_owned_tool_can_break() {
    let owned = OwnedTools(BTreeSet::from([Tool::Shovel]));
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Stone), None);
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Core), None);
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Bedrock), None);
}
