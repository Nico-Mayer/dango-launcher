//! The moves, as pure arithmetic on a display's work area. No platform, no
//! window handle: a `Region` plus a work-area rectangle yields the target
//! rectangle, so every rounding decision is made and tested once.

use crate::platform::Rect;

/// A fixed region of a display's work area a window can be tiled to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter,
    LeftThird,
    CenterThird,
    RightThird,
    Maximize,
}

impl Region {
    /// The rectangle this region occupies inside `area`.
    ///
    /// Splits use the remainder for the far side (`w - w / 2`, `w - 2 * (w / 3)`)
    /// so paired regions tile the whole width or height with no lost or doubled
    /// pixel column, whatever the parity of the work area.
    pub fn rect(self, area: Rect) -> Rect {
        let (x, y, w, h) = (area.x, area.y, area.width, area.height);
        let half_w = w / 2;
        let half_h = h / 2;
        let third_w = w / 3;
        match self {
            Region::LeftHalf => Rect { x, y, width: half_w, height: h },
            Region::RightHalf => Rect { x: x + half_w, y, width: w - half_w, height: h },
            Region::TopHalf => Rect { x, y, width: w, height: half_h },
            Region::BottomHalf => Rect { x, y: y + half_h, width: w, height: h - half_h },
            Region::TopLeftQuarter => Rect { x, y, width: half_w, height: half_h },
            Region::TopRightQuarter => {
                Rect { x: x + half_w, y, width: w - half_w, height: half_h }
            }
            Region::BottomLeftQuarter => {
                Rect { x, y: y + half_h, width: half_w, height: h - half_h }
            }
            Region::BottomRightQuarter => Rect {
                x: x + half_w,
                y: y + half_h,
                width: w - half_w,
                height: h - half_h,
            },
            Region::LeftThird => Rect { x, y, width: third_w, height: h },
            Region::CenterThird => Rect { x: x + third_w, y, width: third_w, height: h },
            Region::RightThird => {
                Rect { x: x + 2 * third_w, y, width: w - 2 * third_w, height: h }
            }
            Region::Maximize => area,
        }
    }
}

/// Centre a window of the given size on the work area, keeping its size. The
/// origin is never placed above or left of the work area, so an oversized window
/// keeps its title bar reachable rather than sliding off the top.
pub fn center(area: Rect, size: (i32, i32)) -> Rect {
    let (width, height) = size;
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    Rect {
        x: x.max(area.x),
        y: y.max(area.y),
        width,
        height,
    }
}

/// Map the target window onto the next display, keeping the same relative region.
///
/// The frame's position and size are taken as fractions of its current work area
/// and reapplied to the destination's, so a left-half window stays a left-half
/// window and a window crossing into a display of a different size or DPI fills
/// the intended region rather than keeping stale pixels. `None` when there is no
/// other display or the current one cannot be identified.
pub fn next_display(frame: Rect, current: Rect, displays: &[Rect]) -> Option<Rect> {
    if displays.len() < 2 {
        return None;
    }
    let index = display_index(frame, current, displays)?;
    let destination = displays[(index + 1) % displays.len()];

    let fx = fraction(frame.x - current.x, current.width);
    let fy = fraction(frame.y - current.y, current.height);
    let fw = fraction(frame.width, current.width);
    let fh = fraction(frame.height, current.height);

    Some(Rect {
        x: destination.x + scale(fx, destination.width),
        y: destination.y + scale(fy, destination.height),
        width: scale(fw, destination.width),
        height: scale(fh, destination.height),
    })
}

fn display_index(frame: Rect, current: Rect, displays: &[Rect]) -> Option<usize> {
    if let Some(index) = displays.iter().position(|d| *d == current) {
        return Some(index);
    }
    // The reported work area may not be byte-identical to a display's, so fall
    // back to the display containing the frame's centre.
    let cx = frame.x + frame.width / 2;
    let cy = frame.y + frame.height / 2;
    displays.iter().position(|d| {
        cx >= d.x && cx < d.x + d.width && cy >= d.y && cy < d.y + d.height
    })
}

fn fraction(part: i32, whole: i32) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

fn scale(fraction: f64, whole: i32) -> i32 {
    (fraction * whole as f64).round() as i32
}

/// The work area at 90% of its size, centred: a nearly full window with a small
/// margin on every side.
pub fn almost_maximize(area: Rect) -> Rect {
    center(area, (area.width * 9 / 10, area.height * 9 / 10))
}

/// A comfortable centred default: 60% of the width by 70% of the height.
pub fn reasonable_size(area: Rect) -> Rect {
    center(area, (area.width * 3 / 5, area.height * 7 / 10))
}

/// The middle half of the width at full height: a centred column.
pub fn center_half(area: Rect) -> Rect {
    let width = area.width / 2;
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y,
        width,
        height: area.height,
    }
}

/// Which way a step resizes the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Larger,
    Smaller,
}

/// A step is a twentieth of the work area on every side, so 5% grows or shrinks
/// each edge and the change scales with the display.
const STEP_DENOMINATOR: i32 = 20;
const MIN_WIDTH: i32 = 400;
const MIN_HEIGHT: i32 = 300;

/// Grow or shrink `frame` by a step on every side, staying centred on its own
/// centre. Growing never exceeds the work area; shrinking never goes below a
/// usable minimum. Repeated steps therefore accumulate to a limit and stop.
pub fn step(frame: Rect, area: Rect, step: Step) -> Rect {
    let dx = area.width / STEP_DENOMINATOR;
    let dy = area.height / STEP_DENOMINATOR;
    let (width, height) = match step {
        Step::Larger => (frame.width + 2 * dx, frame.height + 2 * dy),
        Step::Smaller => (frame.width - 2 * dx, frame.height - 2 * dy),
    };
    let width = width.clamp(MIN_WIDTH.min(area.width), area.width);
    let height = height.clamp(MIN_HEIGHT.min(area.height), area.height);

    let cx = frame.x + frame.width / 2;
    let cy = frame.y + frame.height / 2;
    Rect {
        x: (cx - width / 2).clamp(area.x, area.x + area.width - width),
        y: (cy - height / 2).clamp(area.y, area.y + area.height - height),
        width,
        height,
    }
}

/// A command whose repeats cycle through a set of sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cycle {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    Center,
}

/// The fractions a repeat cycles through, in order.
pub const CYCLE_FRACTIONS: [(i32, i32); 3] = [(1, 2), (2, 3), (1, 3)];

/// The rectangle for a cycle command at a given step. The half commands scale
/// one dimension and stay anchored to their edge; centre scales both and stays
/// centred. At the 1/2 step the half commands delegate to the matching `Region`,
/// so a fresh press is byte-for-byte the existing single-press result.
pub fn cycle_rect(cycle: Cycle, index: usize, area: Rect) -> Rect {
    let (n, d) = CYCLE_FRACTIONS[index % CYCLE_FRACTIONS.len()];
    let is_half = (n, d) == (1, 2);
    match cycle {
        Cycle::LeftHalf if is_half => Region::LeftHalf.rect(area),
        Cycle::RightHalf if is_half => Region::RightHalf.rect(area),
        Cycle::TopHalf if is_half => Region::TopHalf.rect(area),
        Cycle::BottomHalf if is_half => Region::BottomHalf.rect(area),
        Cycle::LeftHalf => Rect { x: area.x, y: area.y, width: area.width * n / d, height: area.height },
        Cycle::RightHalf => {
            let width = area.width * n / d;
            Rect { x: area.x + area.width - width, y: area.y, width, height: area.height }
        }
        Cycle::TopHalf => Rect { x: area.x, y: area.y, width: area.width, height: area.height * n / d },
        Cycle::BottomHalf => {
            let height = area.height * n / d;
            Rect { x: area.x, y: area.y + area.height - height, width: area.width, height }
        }
        Cycle::Center => center(area, (area.width * n / d, area.height * n / d)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect { x: 0, y: 0, width: 1920, height: 1080 };

    #[test]
    fn halves_tile_without_gap_or_overlap() {
        let left = Region::LeftHalf.rect(AREA);
        let right = Region::RightHalf.rect(AREA);
        assert_eq!(left, Rect { x: 0, y: 0, width: 960, height: 1080 });
        assert_eq!(right, Rect { x: 960, y: 0, width: 960, height: 1080 });
        assert_eq!(left.x + left.width, right.x);
        assert_eq!(right.x + right.width, AREA.x + AREA.width);
    }

    #[test]
    fn vertical_halves_tile_exactly() {
        let top = Region::TopHalf.rect(AREA);
        let bottom = Region::BottomHalf.rect(AREA);
        assert_eq!(top.y + top.height, bottom.y);
        assert_eq!(bottom.y + bottom.height, AREA.y + AREA.height);
    }

    #[test]
    fn quarters_cover_the_area_exactly() {
        let tl = Region::TopLeftQuarter.rect(AREA);
        let tr = Region::TopRightQuarter.rect(AREA);
        let bl = Region::BottomLeftQuarter.rect(AREA);
        let br = Region::BottomRightQuarter.rect(AREA);
        let covered: i32 = [tl, tr, bl, br].iter().map(|r| r.width * r.height).sum();
        assert_eq!(covered, AREA.width * AREA.height);
        assert_eq!(tl.x + tl.width, tr.x);
        assert_eq!(tl.y + tl.height, bl.y);
        assert_eq!(br.x + br.width, AREA.x + AREA.width);
        assert_eq!(br.y + br.height, AREA.y + AREA.height);
    }

    #[test]
    fn thirds_cover_the_width_exactly() {
        let l = Region::LeftThird.rect(AREA);
        let c = Region::CenterThird.rect(AREA);
        let r = Region::RightThird.rect(AREA);
        assert_eq!(l.x + l.width, c.x);
        assert_eq!(c.x + c.width, r.x);
        assert_eq!(r.x + r.width, AREA.x + AREA.width);
        assert_eq!(l.height, AREA.height);
    }

    #[test]
    fn odd_width_thirds_and_halves_still_cover_exactly() {
        let area = Rect { x: 10, y: 5, width: 1001, height: 769 };
        let l = Region::LeftThird.rect(area);
        let c = Region::CenterThird.rect(area);
        let r = Region::RightThird.rect(area);
        assert_eq!(l.x, area.x);
        assert_eq!(l.x + l.width, c.x);
        assert_eq!(c.x + c.width, r.x);
        assert_eq!(r.x + r.width, area.x + area.width);
        let lh = Region::LeftHalf.rect(area);
        let rh = Region::RightHalf.rect(area);
        assert_eq!(rh.x + rh.width, area.x + area.width);
        assert_eq!(lh.x + lh.width, rh.x);
    }

    #[test]
    fn maximise_is_the_work_area() {
        assert_eq!(Region::Maximize.rect(AREA), AREA);
    }

    #[test]
    fn respects_a_non_zero_work_area_origin() {
        let area = Rect { x: 100, y: 40, width: 800, height: 600 };
        assert_eq!(
            Region::LeftHalf.rect(area),
            Rect { x: 100, y: 40, width: 400, height: 600 }
        );
    }

    #[test]
    fn centre_keeps_size_and_centres() {
        let r = center(AREA, (800, 600));
        assert_eq!(r, Rect { x: 560, y: 240, width: 800, height: 600 });
    }

    #[test]
    fn centre_of_an_odd_sized_window_is_defined() {
        let r = center(AREA, (801, 601));
        assert_eq!(r.width, 801);
        assert_eq!(r.height, 601);
        assert_eq!(r.x, (1920 - 801) / 2);
    }

    #[test]
    fn centre_keeps_an_oversized_window_reachable() {
        let area = Rect { x: 0, y: 0, width: 400, height: 300 };
        let r = center(area, (800, 600));
        assert_eq!((r.x, r.y), (0, 0));
        assert_eq!((r.width, r.height), (800, 600));
    }

    #[test]
    fn next_display_keeps_the_relative_region() {
        let a = Rect { x: 0, y: 0, width: 1920, height: 1080 };
        let b = Rect { x: 1920, y: 0, width: 1920, height: 1080 };
        let left_half = Region::LeftHalf.rect(a);
        let moved = next_display(left_half, a, &[a, b]).unwrap();
        assert_eq!(moved, Rect { x: 1920, y: 0, width: 960, height: 1080 });
    }

    #[test]
    fn next_display_wraps_to_the_first() {
        let a = Rect { x: 0, y: 0, width: 1920, height: 1080 };
        let b = Rect { x: 1920, y: 0, width: 1920, height: 1080 };
        let on_b = Region::RightHalf.rect(b);
        let moved = next_display(on_b, b, &[a, b]).unwrap();
        assert_eq!(moved, Region::RightHalf.rect(a));
    }

    #[test]
    fn next_display_maps_across_different_scale() {
        let a = Rect { x: 0, y: 0, width: 1920, height: 1080 };
        // A smaller, higher-DPI panel to the right, half the resolution.
        let b = Rect { x: 1920, y: 0, width: 960, height: 540 };
        let left_half = Region::LeftHalf.rect(a);
        let moved = next_display(left_half, a, &[a, b]).unwrap();
        assert_eq!(moved, Rect { x: 1920, y: 0, width: 480, height: 540 });
    }

    #[test]
    fn next_display_is_a_no_op_on_one_display() {
        let a = Rect { x: 0, y: 0, width: 1920, height: 1080 };
        assert_eq!(next_display(Region::LeftHalf.rect(a), a, &[a]), None);
    }

    #[test]
    fn next_display_finds_the_current_by_the_frame_when_work_area_differs() {
        let a = Rect { x: 0, y: 0, width: 1920, height: 1080 };
        let b = Rect { x: 1920, y: 0, width: 1920, height: 1080 };
        // A frame on display b, but a `current` work area that matches neither
        // exactly (a taskbar-shrunk area), so the centre falls back in.
        let frame = Rect { x: 2000, y: 100, width: 400, height: 300 };
        let shrunk = Rect { x: 1920, y: 0, width: 1920, height: 1040 };
        let moved = next_display(frame, shrunk, &[a, b]);
        assert!(moved.is_some());
        // It resolved b as current and wrapped to a.
        assert!(moved.unwrap().x < 1920);
    }

    #[test]
    fn almost_maximise_is_ninety_percent_centred() {
        let r = almost_maximize(AREA);
        assert_eq!((r.width, r.height), (1728, 972));
        assert_eq!(r.x, (1920 - 1728) / 2);
        assert_eq!(r.y, (1080 - 972) / 2);
    }

    #[test]
    fn reasonable_size_is_sixty_by_seventy_centred() {
        let r = reasonable_size(AREA);
        assert_eq!((r.width, r.height), (1152, 756));
        assert_eq!(r.x, (1920 - 1152) / 2);
    }

    #[test]
    fn sizes_respect_a_non_zero_origin() {
        let area = Rect { x: 100, y: 40, width: 800, height: 600 };
        let r = reasonable_size(area);
        assert_eq!(r.x, area.x + (area.width - r.width) / 2);
        assert_eq!(r.y, area.y + (area.height - r.height) / 2);
    }

    #[test]
    fn center_half_is_the_middle_column() {
        let r = center_half(AREA);
        assert_eq!(r, Rect { x: 480, y: 0, width: 960, height: 1080 });
    }

    #[test]
    fn larger_grows_around_the_centre() {
        let frame = Rect { x: 800, y: 400, width: 320, height: 240 };
        let cx = frame.x + frame.width / 2;
        let grown = step(frame, AREA, Step::Larger);
        assert!(grown.width > frame.width && grown.height > frame.height);
        assert_eq!(grown.x + grown.width / 2, cx);
    }

    #[test]
    fn smaller_shrinks_around_the_centre() {
        let frame = Rect { x: 400, y: 200, width: 1000, height: 800 };
        let cx = frame.x + frame.width / 2;
        let small = step(frame, AREA, Step::Smaller);
        assert!(small.width < frame.width && small.height < frame.height);
        assert_eq!(small.x + small.width / 2, cx);
    }

    #[test]
    fn larger_clamps_to_the_work_area() {
        let mut frame = Rect { x: 0, y: 0, width: 200, height: 200 };
        for _ in 0..40 {
            frame = step(frame, AREA, Step::Larger);
        }
        assert_eq!((frame.width, frame.height), (AREA.width, AREA.height));
        assert!(frame.x >= AREA.x && frame.y >= AREA.y);
    }

    #[test]
    fn smaller_clamps_to_the_minimum() {
        let mut frame = Rect { x: 0, y: 0, width: 1920, height: 1080 };
        for _ in 0..40 {
            frame = step(frame, AREA, Step::Smaller);
        }
        assert_eq!((frame.width, frame.height), (400, 300));
    }

    #[test]
    fn cycle_left_half_advances_through_the_fractions() {
        assert_eq!(cycle_rect(Cycle::LeftHalf, 0, AREA), Region::LeftHalf.rect(AREA));
        assert_eq!(
            cycle_rect(Cycle::LeftHalf, 1, AREA),
            Rect { x: 0, y: 0, width: 1280, height: 1080 }
        );
        assert_eq!(
            cycle_rect(Cycle::LeftHalf, 2, AREA),
            Rect { x: 0, y: 0, width: 640, height: 1080 }
        );
    }

    #[test]
    fn cycle_wraps_after_a_third() {
        assert_eq!(cycle_rect(Cycle::LeftHalf, 3, AREA), cycle_rect(Cycle::LeftHalf, 0, AREA));
    }

    #[test]
    fn cycle_right_half_stays_anchored_right() {
        let two_thirds = cycle_rect(Cycle::RightHalf, 1, AREA);
        assert_eq!(two_thirds.x + two_thirds.width, AREA.x + AREA.width);
        assert_eq!(two_thirds.width, 1280);
        assert_eq!(cycle_rect(Cycle::RightHalf, 0, AREA), Region::RightHalf.rect(AREA));
    }

    #[test]
    fn cycle_top_and_bottom_scale_height() {
        let top = cycle_rect(Cycle::TopHalf, 1, AREA);
        assert_eq!((top.width, top.height), (1920, 720));
        let bottom = cycle_rect(Cycle::BottomHalf, 1, AREA);
        assert_eq!(bottom.y + bottom.height, AREA.y + AREA.height);
        assert_eq!(bottom.height, 720);
    }

    #[test]
    fn cycle_center_scales_both_dimensions_centred() {
        let half = cycle_rect(Cycle::Center, 0, AREA);
        assert_eq!((half.width, half.height), (960, 540));
        assert_eq!(half.x, (1920 - 960) / 2);
        assert_eq!(half.y, (1080 - 540) / 2);
    }

    #[test]
    fn first_press_of_each_half_matches_the_region() {
        for (cycle, region) in [
            (Cycle::LeftHalf, Region::LeftHalf),
            (Cycle::RightHalf, Region::RightHalf),
            (Cycle::TopHalf, Region::TopHalf),
            (Cycle::BottomHalf, Region::BottomHalf),
        ] {
            // Odd-sized area so the anchoring rounding would show.
            let area = Rect { x: 7, y: 3, width: 1001, height: 769 };
            assert_eq!(cycle_rect(cycle, 0, area), region.rect(area));
        }
    }
}
