use reactor_safety_sim as rss;

#[test]
fn overtemp_trips_scram() {
    let dt_s = 0.05;
    let steps = 400;

    let p = rss::PlantParams::default();
    let mut x = rss::PlantState::default();
    x.coolant = 0.15;

    let mut s_cfg = rss::SafetyConfig::default();
    s_cfg.trip_temp_c = 420.0;

    let mut s_state = rss::SafetyState::default();

    let mut s1 = rss::Sensor::new(1);
    let mut s2 = rss::Sensor::new(2);
    let mut s3 = rss::Sensor::new(3);

    // Force heating
    x.power = 1.0;

    for _ in 0..steps {
        let y1 = s1.read_temp(x.temp_c, dt_s);
        let y2 = s2.read_temp(x.temp_c, dt_s);
        let y3 = s3.read_temp(x.temp_c, dt_s);

        rss::evaluate(&s_cfg, &mut s_state, [y1, y2, y3]);
        if s_state.scram {
            break;
        }

        x.step(&p, dt_s);
    }

    assert!(s_state.scram, "Expected SCRAM to be triggered");
    assert_eq!(s_state.reason, Some(rss::TripReason::OverTemp));
}
