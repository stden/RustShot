//! `CliFormat` tests: conversion into `capture::Format` and derived-trait behaviour.

use super::*;

// --- CliFormat conversion tests ---

#[test]
fn cli_format_from_png_gives_capture_png() {
    assert!(matches!(
        capture::Format::from(CliFormat::Png),
        capture::Format::Png
    ));
}

#[test]
fn cli_format_from_jpg_gives_capture_jpg() {
    assert!(matches!(
        capture::Format::from(CliFormat::Jpg),
        capture::Format::Jpg
    ));
}

#[test]
fn cli_format_from_bmp_gives_capture_bmp() {
    assert!(matches!(
        capture::Format::from(CliFormat::Bmp),
        capture::Format::Bmp
    ));
}

#[test]
fn cli_format_clone_works() {
    let f = CliFormat::Jpg;
    let g = f.clone();
    assert!(matches!(g, CliFormat::Jpg));
}

#[test]
fn cli_format_debug_repr_is_non_empty() {
    assert!(!format!("{:?}", CliFormat::Png).is_empty());
    assert!(!format!("{:?}", CliFormat::Jpg).is_empty());
    assert!(!format!("{:?}", CliFormat::Bmp).is_empty());
}

// --- CliFormat trait tests ---

#[test]
fn cli_format_clone_produces_equal_value() {
    let f = CliFormat::Jpg;
    assert_eq!(f.clone(), CliFormat::Jpg);
}

#[test]
fn cli_format_partial_eq_works() {
    assert_eq!(CliFormat::Png, CliFormat::Png);
    assert_ne!(CliFormat::Png, CliFormat::Jpg);
    assert_ne!(CliFormat::Jpg, CliFormat::Bmp);
}

// --- CliFormat conversion exhaustiveness ---

#[test]
fn cli_format_to_capture_format_all_variants() {
    let conversions: Vec<(CliFormat, &str)> = vec![
        (CliFormat::Png, "png"),
        (CliFormat::Jpg, "jpg"),
        (CliFormat::Bmp, "bmp"),
    ];
    for (cli_fmt, expected_ext) in conversions {
        let capture_fmt: capture::Format = cli_fmt.into();
        assert_eq!(capture_fmt.extension(), expected_ext);
    }
}

// --- CliFormat conversion ---

#[test]
fn cli_format_into_capture_format_png() {
    let f: capture::Format = CliFormat::Png.into();
    assert!(matches!(f, capture::Format::Png));
}

#[test]
fn cli_format_into_capture_format_jpg() {
    let f: capture::Format = CliFormat::Jpg.into();
    assert!(matches!(f, capture::Format::Jpg));
}

#[test]
fn cli_format_into_capture_format_bmp() {
    let f: capture::Format = CliFormat::Bmp.into();
    assert!(matches!(f, capture::Format::Bmp));
}

// --- CliFormat PartialEq ---

#[test]
fn cli_format_equality() {
    assert_eq!(CliFormat::Png, CliFormat::Png);
    assert_eq!(CliFormat::Jpg, CliFormat::Jpg);
    assert_eq!(CliFormat::Bmp, CliFormat::Bmp);
    assert_ne!(CliFormat::Png, CliFormat::Jpg);
    assert_ne!(CliFormat::Png, CliFormat::Bmp);
    assert_ne!(CliFormat::Jpg, CliFormat::Bmp);
}

// --- CliFormat Clone ---

#[test]
fn cli_format_clone() {
    let f = CliFormat::Jpg;
    let g = f.clone();
    assert_eq!(f, g);
}

// -------------------------------------------------------------------------
// CliFormat derive trait tests (non-duplicate)
// -------------------------------------------------------------------------

#[test]
fn cli_format_partial_eq_all_pairs_differ() {
    assert_ne!(CliFormat::Png, CliFormat::Jpg);
    assert_ne!(CliFormat::Jpg, CliFormat::Bmp);
    assert_ne!(CliFormat::Png, CliFormat::Bmp);
}

#[test]
fn cli_format_eq_same_variant() {
    assert_eq!(CliFormat::Png, CliFormat::Png);
    assert_eq!(CliFormat::Jpg, CliFormat::Jpg);
    assert_eq!(CliFormat::Bmp, CliFormat::Bmp);
}
