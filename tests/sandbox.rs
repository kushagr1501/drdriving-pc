use drdriving_core::{cars::*, economy::*, save::SaveGame};

#[test]
fn sandbox_unlocks_every_selectable_car_and_rentals_never_expire() {
    let mut save = SaveGame::default();
    enable_local_sandbox(&mut save);
    assert!(save.has_car[..PLAYER_CARS].iter().all(|owned| *owned));
    let run = RunStats { car: PURCHASABLE_CARS, travel_distance: 45_000.0, ..Default::default() };
    finish_mission(&mut save, &run, Outcome::NoFuel);
    assert_eq!(save.car_rent[PURCHASABLE_CARS], i32::MAX);
    assert_eq!(save.coin, i32::MAX);
    assert_eq!(save.gold, i32::MAX);
}

#[test]
fn unlimited_balances_survive_spending_and_reward_overflow() {
    let mut save = SaveGame::default();
    save.coin = i32::MAX;
    save.gold = i32::MAX;
    buy_car(&mut save, 1).unwrap();
    buy_car(&mut save, 7).unwrap();
    buy_coin(&mut save, 1500, 10).unwrap();
    upgrade_car(&mut save, 1, 0).unwrap();
    save.car_damage[1] = 9000;
    repair_car(&mut save, 1).unwrap();
    apply_purchase(&mut save, "drdriving.gold1500");
    assert_eq!((save.coin, save.gold), (i32::MAX, i32::MAX));
}

#[test]
fn regular_gold_still_spends_normally() {
    let mut save = SaveGame::default();
    save.gold = 110;
    buy_car(&mut save, 7).unwrap();
    buy_coin(&mut save, 1500, 10).unwrap();
    assert_eq!((save.coin, save.gold), (1500, 0));
}

#[test]
fn wheel_rotation_reverses_when_reversing() {
    use drdriving_core::world::{World, Controls, mode, GEAR_R, GEAR_D};
    let save = SaveGame::default();
    for (gear, direction) in [(GEAR_D, 1.0), (GEAR_R, -1.0)] {
        let mut world = World::new(0, Some(17));
        world.prepare_race(&save);
        world.gear = gear;
        for _ in 0..60 { world.update(1.0/60.0, mode::PLAYING, Controls { accel:true,brake:false,steer:0.0 }); }
        assert!(world.player.ang_distance * direction > 0.0);
    }
}
