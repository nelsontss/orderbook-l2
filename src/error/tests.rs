use super::BookError;

const ALL: [BookError; 4] = [
    BookError::QuantityNotPositive,
    BookError::UnknownLevel,
    BookError::InsufficientQuantity,
    BookError::PriceOutsideGrid,
];

#[test]
fn every_variant_renders_a_message() {
    for error in ALL {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn variants_are_distinguishable_by_message() {
    let messages: Vec<String> = ALL.iter().map(ToString::to_string).collect();

    for (index, message) in messages.iter().enumerate() {
        for other in &messages[index + 1..] {
            assert_ne!(message, other);
        }
    }
}

#[test]
fn usable_as_a_std_error() {
    let error = BookError::InsufficientQuantity;
    let erased: &dyn std::error::Error = &error;

    assert_eq!(erased.to_string(), error.to_string());
}

#[test]
fn copy_and_equality_hold() {
    let error = BookError::UnknownLevel;
    let copied = error;

    assert_eq!(error, copied);
    assert_ne!(BookError::UnknownLevel, BookError::InsufficientQuantity);
}
