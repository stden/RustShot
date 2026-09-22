//! Windows presentation must not expose the intermediate pieces of a repaint.

use super::*;
use std::num::NonZeroU32;

const WIDTH: usize = 32;
const HEIGHT: usize = 24;

fn assert_every_presented_frame_is_complete(previous: OverlayDecoration, next: OverlayDecoration) {
    assert_presentation_completes_frame(previous, next, None);
}

fn assert_presentation_completes_frame(
    previous: OverlayDecoration,
    next: OverlayDecoration,
    exposed_pixel: Option<(usize, usize)>,
) {
    let background: Vec<u32> = (0..WIDTH * HEIGHT).map(|pixel| pixel as u32).collect();
    let mut previous_frame = background.clone();
    assert!(compose_overlay_frame(
        &background,
        &mut previous_frame,
        WIDTH,
        HEIGHT,
        previous,
    ));
    let mut expected = background.clone();
    assert!(compose_overlay_frame(
        &background,
        &mut expected,
        WIDTH,
        HEIGHT,
        next,
    ));
    assert_ne!(
        previous_frame, expected,
        "the fixture must move a decoration"
    );

    let mut back = previous_frame.clone();
    let damage = redraw_overlay_frame(&background, &mut back, (WIDTH, HEIGHT), previous, next);
    assert_eq!(back, expected, "the prepared back frame must be complete");

    // Win32 softbuffer submits one BitBlt for each presentation rectangle. The
    // desktop can observe the result between those calls, even though the back
    // frame was fully composed before the first call. Keep a separate visible
    // frame and consume the production presentation policy in its actual order.
    let mut visible = previous_frame.clone();
    if let Some((x, y)) = exposed_pixel {
        assert!(x < WIDTH && y < HEIGHT);
        assert!(
            !damage.as_slice().iter().any(|rectangle| {
                x >= rectangle.x
                    && x < rectangle.x + rectangle.width
                    && y >= rectangle.y
                    && y < rectangle.y + rectangle.height
            }),
            "the exposed pixel must be outside the decoration's damage"
        );
        visible[y * WIDTH + x] ^= 0x00ff_ffff;
    }
    let before_presentation = visible.clone();
    let rectangles = damage_rects(
        &damage,
        NonZeroU32::new(WIDTH as u32).unwrap(),
        NonZeroU32::new(HEIGHT as u32).unwrap(),
    );
    for (index, rectangle) in rectangles.iter().enumerate() {
        let x = rectangle.x as usize;
        let y = rectangle.y as usize;
        let width = rectangle.width.get() as usize;
        let height = rectangle.height.get() as usize;
        assert!(x + width <= WIDTH && y + height <= HEIGHT);
        for row in y..y + height {
            let start = row * WIDTH + x;
            let end = start + width;
            visible[start..end].copy_from_slice(&back[start..end]);
        }
        assert!(
            visible == before_presentation || visible == expected,
            "presentation rectangle {index} exposed an incomplete frame: \
             {previous:?} -> {next:?}, invalidated pixel: {exposed_pixel:?}"
        );
    }
    assert_eq!(visible, expected, "presentation must reach the next frame");
}

#[test]
fn moving_crosshair_presents_only_complete_frames() {
    assert_every_presented_frame_is_complete(
        OverlayDecoration::new(None, Some((3, 4)), WIDTH, HEIGHT),
        OverlayDecoration::new(None, Some((25, 19)), WIDTH, HEIGHT),
    );
}

#[test]
fn moving_and_resizing_border_presents_only_complete_frames() {
    assert_every_presented_frame_is_complete(
        OverlayDecoration::new(Some((2, 2)), Some((13, 10)), WIDTH, HEIGHT),
        OverlayDecoration::new(Some((5, 4)), Some((24, 18)), WIDTH, HEIGHT),
    );
}

#[test]
fn moving_border_repaints_exposed_background_outside_its_damage() {
    // A window uncovered while the border moves can have invalid background
    // pixels beyond both border bounds. Win32 softbuffer validates the entire
    // window after presenting, so leaving those pixels unpainted loses the OS
    // repaint request even though the decoration itself looks complete.
    assert_presentation_completes_frame(
        OverlayDecoration::new(Some((2, 2)), Some((13, 10)), WIDTH, HEIGHT),
        OverlayDecoration::new(Some((5, 4)), Some((24, 18)), WIDTH, HEIGHT),
        Some((WIDTH - 1, HEIGHT - 1)),
    );
}

#[test]
fn crosshair_and_border_transitions_present_only_complete_frames() {
    let crosshair = OverlayDecoration::new(None, Some((8, 6)), WIDTH, HEIGHT);
    let border = OverlayDecoration::new(Some((8, 6)), Some((23, 17)), WIDTH, HEIGHT);
    assert_every_presented_frame_is_complete(crosshair, border);
    assert_every_presented_frame_is_complete(border, crosshair);
}
