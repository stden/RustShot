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
