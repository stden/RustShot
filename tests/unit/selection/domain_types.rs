use super::*;

#[test]
fn scale_factor_rejects_non_positive_and_non_finite_values() {
    for value in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            ScaleFactor::try_from(value),
            Err("Display scale factor must be positive".to_string())
        );
    }
}

#[test]
fn scale_factor_returns_the_validated_value_unchanged() {
    // Validation must not also normalise: `get` feeds the raw factor back into
    // coordinate scaling, so rounding or clamping it here would mis-place every
    // physical coordinate on a fractional-DPI monitor.
    assert_eq!(ScaleFactor::try_from(1.25).unwrap().get(), 1.25);
    assert_eq!(ScaleFactor::try_from(2.5).unwrap().get(), 2.5);
    assert_eq!(ScaleFactor::ONE.get(), 1.0);
    // The smallest value validation accepts still survives intact.
    assert_eq!(
        ScaleFactor::try_from(f32::MIN_POSITIVE).unwrap().get(),
        f32::MIN_POSITIVE
    );
}

#[test]
fn logical_point_conversion_preserves_the_coordinate_space_boundary() {
    let scale = ScaleFactor::try_from(1.25).unwrap();
    let physical = LogicalPoint::new(-7, 10).to_physical(scale);

    assert_eq!(physical, PhysicalPoint::new(-9, 13));
}

#[test]
fn logical_size_conversion_rounds_each_physical_extent_up() {
    let scale = ScaleFactor::try_from(1.5).unwrap();
    let physical = LogicalSize::new(1921, 1079).to_physical(scale).unwrap();

    assert_eq!(physical, PhysicalSize::new(2882, 1619));
}

#[test]
fn region_exposes_typed_physical_parts() {
    let origin = PhysicalPoint::new(-50, 20);
    let size = PhysicalSize::new(100, 200);
    let region = Region::from_physical(origin, size);

    assert_eq!(region.origin(), origin);
    assert_eq!(region.size(), size);
}

#[test]
fn logical_and_physical_newtypes_round_trip_through_tuples() {
    // The newtypes exist to stop logical and physical coordinates being mixed
    // up; the tuple conversions are how callers get in and out of them, so a
    // round trip must be lossless in both directions.
    assert_eq!(<(i32, i32)>::from(LogicalPoint::from((-3, 7))), (-3, 7));
    assert_eq!(<(i32, i32)>::from(PhysicalPoint::from((5, -9))), (5, -9));
    assert_eq!(
        <(u32, u32)>::from(LogicalSize::from((1280, 720))),
        (1280, 720)
    );
    assert_eq!(
        <(u32, u32)>::from(PhysicalSize::from((1920, 1080))),
        (1920, 1080)
    );

    // The accessors agree with the tuple the value was built from.
    let point = LogicalPoint::from((11, -13));
    assert_eq!((point.x(), point.y()), (11, -13));
    let size = PhysicalSize::from((640, 480));
    assert_eq!((size.width(), size.height()), (640, 480));
}
