use super::Side;

#[test]
fn opposite_swaps_the_side() {
    assert_eq!(Side::Bid.opposite(), Side::Ask);
    assert_eq!(Side::Ask.opposite(), Side::Bid);
}

#[test]
fn opposite_is_its_own_inverse() {
    for side in [Side::Bid, Side::Ask] {
        assert_eq!(side.opposite().opposite(), side);
    }
}

#[test]
fn opposite_never_returns_the_same_side() {
    for side in [Side::Bid, Side::Ask] {
        assert_ne!(side.opposite(), side);
    }
}
