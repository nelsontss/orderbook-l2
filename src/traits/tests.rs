use super::{Price, Quantity};

fn requires_price<P: Price>() {}

fn requires_quantity<Q: Quantity>() {}

#[test]
fn primitive_prices_implement_price() {
    requires_price::<i64>();
    requires_price::<u64>();
    requires_price::<i128>();
    requires_price::<u128>();
}

#[test]
fn primitive_quantities_implement_quantity() {
    requires_quantity::<i64>();
    requires_quantity::<u64>();
    requires_quantity::<i128>();
    requires_quantity::<u128>();
}

#[test]
fn zero_is_the_default_value() {
    assert!(0_u64.is_zero());
    assert!(0_i64.is_zero());
    assert!(!1_u64.is_zero());
    assert!(!(-1_i64).is_zero());
}

#[test]
fn positive_excludes_zero_and_negatives() {
    assert!(3_u64.is_positive());
    assert!(3_i64.is_positive());
    assert!(!0_i64.is_positive());
    assert!(!(-3_i64).is_positive());
}

#[test]
fn midpoint_of_an_even_sum_is_exact() {
    assert_eq!(100_i64.mid(104), 102);
    assert_eq!(100_u64.mid(102), 101);
}

#[test]
fn midpoint_floors_when_the_sum_is_odd() {
    assert_eq!(100_i64.mid(101), 100);
    assert_eq!(101_i64.mid(100), 100);
    assert_eq!(100_u64.mid(101), 100);
}

#[test]
fn midpoint_floors_toward_the_lower_value_for_negatives() {
    assert_eq!((-100_i64).mid(-101), -101);
    assert_eq!((-101_i64).mid(-100), -101);
    assert_eq!((-1_i64).mid(0), -1);
}

#[test]
fn midpoint_is_symmetric() {
    for (left, right) in [(-7_i64, 4_i64), (0, 0), (-1, 1), (5, 5), (i64::MIN, 0)] {
        assert_eq!(left.mid(right), right.mid(left));
    }
}

#[test]
fn midpoint_does_not_overflow_at_the_extremes() {
    assert_eq!(i64::MIN.mid(i64::MAX), -1);
    assert_eq!(i64::MAX.mid(i64::MIN), -1);
    assert_eq!(i64::MIN.mid(i64::MIN), i64::MIN);
    assert_eq!(i64::MAX.mid(i64::MAX), i64::MAX);

    assert_eq!(i128::MIN.mid(i128::MAX), -1);
    assert_eq!(u64::MIN.mid(u64::MAX), u64::MAX / 2);
    assert_eq!(u128::MIN.mid(u128::MAX), u128::MAX / 2);
}

#[test]
fn midpoint_never_escapes_the_input_range() {
    let samples = [
        i64::MIN,
        i64::MIN / 2,
        -1_000_000_000,
        -1,
        0,
        1,
        1_000_000_000,
        i64::MAX / 2,
        i64::MAX,
    ];

    for left in samples {
        for right in samples {
            let midpoint = left.mid(right);
            let low = left.min(right);
            let high = left.max(right);
            assert!(low <= midpoint && midpoint <= high);
        }
    }
}
