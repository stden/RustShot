use super::*;

#[test]
fn full_monitor_region_comes_from_graphics_capture_item_size() {
    let region = resolve_capture_region(
        None,
        SizeInt32 {
            Width: 2880,
            Height: 1800,
        },
    )
    .unwrap();

    assert_eq!(
        region,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 2880,
            physical_height: 1800,
        }
    );
}

#[test]
fn a_capture_item_with_no_pixels_is_rejected() {
    // A monitor that is being torn down reports a zero-sized capture item.
    // Accepting it would size the staging texture and the frame buffer to
    // nothing, and the failure would surface much later as a copy error.
    for size in [
        SizeInt32 {
            Width: 0,
            Height: 1080,
        },
        SizeInt32 {
            Width: 1920,
            Height: 0,
        },
        SizeInt32 {
            Width: -1920,
            Height: -1080,
        },
    ] {
        assert_eq!(
            resolve_capture_region(None, size),
            Err("Monitor capture item has zero or negative size".to_string()),
            "accepted {}x{}",
            size.Width,
            size.Height
        );
    }
}

#[test]
fn a_requested_region_outside_the_capture_item_is_rejected() {
    // The region comes from a selection made against an earlier monitor
    // layout, so it can be stale by the time the capture item exists.
    let error = resolve_capture_region(
        Some(Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 4000,
            physical_height: 1800,
        }),
        SizeInt32 {
            Width: 2880,
            Height: 1800,
        },
    )
    .expect_err("a region wider than the monitor cannot be captured");
    assert!(!error.is_empty(), "the rejection must be explained");
}

#[test]
fn each_capture_format_describes_itself_consistently() {
    // The staging texture is allocated from `bytes_per_pixel` and filled by
    // the GPU according to `dxgi_format`. If the two ever disagreed, an HDR
    // monitor would read half a frame past the end of its buffer.
    assert_eq!(CaptureFormat::Bgra8.bytes_per_pixel(), 4);
    assert_eq!(
        CaptureFormat::ScRgb16 {
            peak_luminance_nits: 1_000.0,
        }
        .bytes_per_pixel(),
        8
    );

    let sdr = CaptureFormat::Bgra8;
    let hdr = CaptureFormat::ScRgb16 {
        peak_luminance_nits: 1_000.0,
    };
    assert_ne!(sdr.pixel_format(), hdr.pixel_format());
    assert_ne!(sdr.dxgi_format(), hdr.dxgi_format());
    // The peak luminance describes the content, not the layout: two HDR
    // formats that differ only in brightness share one texture format.
    assert_eq!(
        hdr.dxgi_format(),
        CaptureFormat::ScRgb16 {
            peak_luminance_nits: 600.0,
        }
        .dxgi_format()
    );
}

#[test]
fn hdr_format_transition_requires_source_rebuild() {
    assert!(!capture_format_changed(
        CaptureFormat::Bgra8,
        CaptureFormat::Bgra8
    ));
    assert!(capture_format_changed(
        CaptureFormat::Bgra8,
        CaptureFormat::ScRgb16 {
            peak_luminance_nits: 1_000.0,
        }
    ));
    assert!(capture_format_changed(
        CaptureFormat::ScRgb16 {
            peak_luminance_nits: 1_000.0,
        },
        CaptureFormat::ScRgb16 {
            peak_luminance_nits: 600.0,
        }
    ));
    // Below the one-nit threshold nothing is rebuilt: the reported peak
    // luminance jitters slightly between frames, and treating that as a
    // format change would tear down and recreate the capture session over
    // and over for the whole recording.
    assert!(!capture_format_changed(
        CaptureFormat::ScRgb16 {
            peak_luminance_nits: 1_000.0,
        },
        CaptureFormat::ScRgb16 {
            peak_luminance_nits: 1_000.4,
        }
    ));
}
