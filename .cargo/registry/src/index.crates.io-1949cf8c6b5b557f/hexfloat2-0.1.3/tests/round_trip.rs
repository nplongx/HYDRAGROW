use std::ops::Deref;

use hexfloat2::HexFloat;

#[track_caller]
fn round_trip_f32(val: f32) {
    let val_bits = val.to_bits();
    let hf = HexFloat::new(val);
    let hf_string = hf.to_string();
    let hf2: HexFloat<f32> = hf_string.parse().unwrap();

    eprintln!("{val:?} ({val_bits:#08x}) -> {hf_string} -> {hf2:?} -> {hf2}");

    assert_eq!(hf2.deref(), &val);
}

#[test]
fn test_round_trip_f32() {
    let mut val = 1.0f32;
    loop {
        val = val * 2.0;
        if val.is_infinite() {
            break;
        };
        round_trip_f32(val);
    }

    let mut val = 1.0f32;
    loop {
        val = val * 0.5;
        if val == 0.0 {
            break;
        };
        round_trip_f32(val);
    }
}

#[track_caller]
fn round_trip_f64(val: f64) {
    let val_bits = val.to_bits();
    let hf = HexFloat::new(val);
    let hf_string = hf.to_string();
    let hf2: HexFloat<f64> = hf_string.parse().unwrap();

    eprintln!("{val:?} ({val_bits:#08x}) -> {hf_string} -> {hf2:?} -> {hf2}");

    assert_eq!(hf2.deref(), &val);
}

#[test]
fn test_round_trip_f64() {
    let mut val = 1.0f64;
    loop {
        val = val * 2.0;
        if val.is_infinite() {
            break;
        };
        round_trip_f64(val);
    }

    let mut val = 1.0f64;
    loop {
        val = val * 0.5;
        if val == 0.0 {
            break;
        };
        round_trip_f64(val);
    }
}
