use super::Level;

#[test]
fn fields_are_publicly_constructible_and_readable() {
    let level = Level {
        price: 100_i64,
        quantity: 7_u64,
    };

    assert_eq!(level.price, 100);
    assert_eq!(level.quantity, 7);
}

#[test]
fn levels_compare_by_price_then_quantity() {
    let low = Level {
        price: 99_i64,
        quantity: 1_u64,
    };
    let same = Level {
        price: 99_i64,
        quantity: 1_u64,
    };
    let higher = Level {
        price: 100_i64,
        quantity: 1_u64,
    };

    assert_eq!(low, same);
    assert_ne!(low, higher);
    assert!(low < higher);
}

#[test]
fn a_level_is_copy() {
    let level = Level {
        price: 100_i64,
        quantity: 7_u64,
    };
    let copied = level;

    assert_eq!(copied, level);
}
