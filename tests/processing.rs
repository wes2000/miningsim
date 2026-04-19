use miningsim::items::OreKind;
use miningsim::processing::{self, SmelterState, SmeltTickEvent, SMELT_DURATION_S};

#[test]
fn default_state_is_idle() {
    let s = SmelterState::default();
    assert_eq!(s.recipe, None);
    assert_eq!(s.queue, 0);
    assert!(s.output.is_empty());
    assert!(!processing::is_busy(&s));
}

#[test]
fn start_smelting_sets_recipe_and_timer() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 5);
    assert_eq!(s.recipe, Some(OreKind::Copper));
    assert_eq!(s.queue, 5);
    assert_eq!(s.time_left, SMELT_DURATION_S);
    assert!(processing::is_busy(&s));
}

#[test]
fn start_smelting_with_zero_count_is_noop() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 0);
    assert_eq!(s.recipe, None);
    assert_eq!(s.queue, 0);
}

#[test]
fn start_smelting_while_busy_is_noop() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 3);
    processing::start_smelting(&mut s, OreKind::Silver, 7);
    assert_eq!(s.recipe, Some(OreKind::Copper));
    assert_eq!(s.queue, 3);
}

#[test]
fn tick_decrements_timer() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 1);
    let ev = processing::tick_smelter(&mut s, 0.5);
    assert_eq!(ev, SmeltTickEvent::None);
    assert_eq!(s.time_left, SMELT_DURATION_S - 0.5);
}

#[test]
fn full_tick_completes_one_item() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 2);
    let ev = processing::tick_smelter(&mut s, SMELT_DURATION_S);
    assert_eq!(ev, SmeltTickEvent::BarFinished(OreKind::Copper));
    assert_eq!(s.queue, 1);
    assert_eq!(*s.output.get(&OreKind::Copper).unwrap_or(&0), 1);
    assert_eq!(s.recipe, Some(OreKind::Copper));   // queue not empty -> still smelting
    assert_eq!(s.time_left, SMELT_DURATION_S);     // reset for next item
}

#[test]
fn last_item_in_queue_returns_to_idle() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Silver, 1);
    let _ = processing::tick_smelter(&mut s, SMELT_DURATION_S);
    assert_eq!(s.recipe, None);
    assert_eq!(s.queue, 0);
    assert!(!processing::is_busy(&s));
    assert_eq!(*s.output.get(&OreKind::Silver).unwrap_or(&0), 1);
}

#[test]
fn tick_overshoot_completes_exactly_one_item() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 5);
    let ev = processing::tick_smelter(&mut s, 100.0);
    assert_eq!(ev, SmeltTickEvent::BarFinished(OreKind::Copper));
    assert_eq!(s.queue, 4);
    assert_eq!(*s.output.get(&OreKind::Copper).unwrap_or(&0), 1);
    assert_eq!(s.time_left, SMELT_DURATION_S);
}

#[test]
fn tick_when_idle_is_noop() {
    let mut s = SmelterState::default();
    let ev = processing::tick_smelter(&mut s, 5.0);
    assert_eq!(ev, SmeltTickEvent::None);
    assert_eq!(s.recipe, None);
    assert!(s.output.is_empty());
}

#[test]
fn collect_output_drains_and_returns() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Gold, 3);
    for _ in 0..3 { let _ = processing::tick_smelter(&mut s, SMELT_DURATION_S); }
    assert_eq!(*s.output.get(&OreKind::Gold).unwrap_or(&0), 3);
    let drained = processing::collect_output(&mut s);
    assert_eq!(*drained.get(&OreKind::Gold).unwrap_or(&0), 3);
    assert!(s.output.is_empty());
}

#[test]
fn collect_output_on_empty_returns_empty() {
    let mut s = SmelterState::default();
    let d = processing::collect_output(&mut s);
    assert!(d.is_empty());
}
