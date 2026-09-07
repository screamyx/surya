//! Tests for `window.rs`. Pure: no window, no display, no settings file.

use super::*;

/// A single 1920x1080 display at the origin, the ordinary case.
fn one_display() -> Vec<Bounds<Pixels>> {
    vec![Bounds {
        origin: Point { x: px(0.0), y: px(0.0) },
        size: Size { width: px(1920.0), height: px(1080.0) },
    }]
}

fn placed(x: f32, y: f32, width: f32, height: f32) -> WindowPlacement {
    WindowPlacement { x, y, width, height, maximized: false }
}

/// The finding itself: what the person left is what they get back. e2e2 closed
/// at 1094x1447 at (52,0) and reopened on the 1336x888 default.
#[test]
fn a_saved_window_comes_back_where_it_was_left() {
    let displays = vec![Bounds {
        origin: Point { x: px(0.0), y: px(0.0) },
        size: Size { width: px(1920.0), height: px(1600.0) },
    }];
    let restored = restore(Some(placed(52.0, 0.0, 1094.0, 1447.0)), &displays);
    assert_eq!(
        restored,
        Some(WindowBounds::Windowed(Bounds {
            origin: Point { x: px(52.0), y: px(0.0) },
            size: Size { width: px(1094.0), height: px(1447.0) },
        }))
    );
}

#[test]
fn a_first_run_has_nothing_to_restore() {
    assert_eq!(restore(None, &one_display()), None);
}

/// A window left maximized comes back maximized, carrying the rect it
/// un-maximizes to.
#[test]
fn maximized_survives_the_restart() {
    let saved = WindowPlacement { maximized: true, ..placed(100.0, 60.0, 1320.0, 880.0) };
    let restored = restore(Some(saved), &one_display());
    match restored {
        Some(WindowBounds::Maximized(rect)) => {
            assert_eq!(f32::from(rect.size.width), 1320.0);
            assert_eq!(f32::from(rect.origin.x), 100.0);
        }
        other => panic!("expected Maximized, got {other:?}"),
    }
}

/// Fullscreen is saved as its underlying rect and reopens windowed. Coming
/// back fullscreen with the app's own chrome hidden leaves no visible way out.
#[test]
fn fullscreen_reopens_windowed_at_the_rect_under_it() {
    let saved = WindowPlacement::of(WindowBounds::Fullscreen(Bounds {
        origin: Point { x: px(40.0), y: px(20.0) },
        size: Size { width: px(1280.0), height: px(800.0) },
    }));
    assert!(!saved.maximized);
    assert_eq!(
        restore(Some(saved), &one_display()),
        Some(WindowBounds::Windowed(Bounds {
            origin: Point { x: px(40.0), y: px(20.0) },
            size: Size { width: px(1280.0), height: px(800.0) },
        }))
    );
}

/// The rect a maximized window reports is its restore rect, so it round-trips
/// as one (gpui platform.rs 1885-1887).
#[test]
fn the_saved_rect_is_the_restore_rect() {
    let rect = Bounds {
        origin: Point { x: px(7.0), y: px(9.0) },
        size: Size { width: px(1000.0), height: px(700.0) },
    };
    let saved = WindowPlacement::of(WindowBounds::Maximized(rect));
    assert!(saved.maximized);
    assert_eq!(saved.rect(), rect);
}

/// The one that keeps a person from losing their window. A second monitor
/// that is unplugged leaves the saved rect nowhere.
#[test]
fn a_rect_on_a_monitor_that_is_gone_falls_back() {
    let saved = placed(2400.0, 300.0, 1200.0, 800.0);
    assert_eq!(restore(Some(saved), &one_display()), None, "the display it was on is not here");
    // Plug it back in and the same rect is fine.
    let mut both = one_display();
    both.push(Bounds {
        origin: Point { x: px(1920.0), y: px(0.0) },
        size: Size { width: px(1920.0), height: px(1080.0) },
    });
    assert!(restore(Some(saved), &both).is_some());
}

/// Overlapping a display is not the same as being reachable: a window one
/// pixel onto the screen cannot be grabbed and dragged back.
#[test]
fn a_sliver_on_screen_is_not_reachable() {
    let barely = placed(1919.0, 500.0, 1200.0, 800.0);
    assert_eq!(restore(Some(barely), &one_display()), None);
    let enough = placed(1920.0 - 120.0, 500.0, 1200.0, 800.0);
    assert!(restore(Some(enough), &one_display()).is_some(), "120 px of strip is the line");
}

/// Two displays each showing a sliver do not add up to a grabbable strip, so
/// each display is tested on its own.
#[test]
fn slivers_on_two_displays_do_not_add_up() {
    let displays = vec![
        Bounds {
            origin: Point { x: px(0.0), y: px(0.0) },
            size: Size { width: px(1000.0), height: px(1000.0) },
        },
        // A gap, then a second display. The saved window straddles the gap.
        Bounds {
            origin: Point { x: px(1100.0), y: px(0.0) },
            size: Size { width: px(1000.0), height: px(1000.0) },
        },
    ];
    let straddling = placed(940.0, 100.0, 220.0, 700.0);
    assert_eq!(restore(Some(straddling), &displays), None);
}

/// A window dragged above the top of the screen has no strip showing at all.
#[test]
fn a_rect_above_the_screen_falls_back() {
    assert_eq!(restore(Some(placed(200.0, -400.0, 1200.0, 800.0)), &one_display()), None);
}

/// A hand-edited settings file must not open a window with no size, and NaN
/// must not slip through the comparisons as a silent true.
#[test]
fn a_nonsense_placement_falls_back() {
    for bad in [
        placed(0.0, 0.0, 0.0, 0.0),
        placed(0.0, 0.0, 10.0, 10.0),
        placed(f32::NAN, 0.0, 1200.0, 800.0),
        placed(0.0, f32::INFINITY, 1200.0, 800.0),
        placed(0.0, 0.0, f32::NAN, 800.0),
    ] {
        assert_eq!(restore(Some(bad), &one_display()), None, "accepted {bad:?}");
    }
}

/// A machine reporting no displays at all: open on the default rather than
/// somewhere nobody can see.
#[test]
fn no_displays_means_no_restore() {
    assert_eq!(restore(Some(placed(0.0, 0.0, 1200.0, 800.0)), &[]), None);
}

/// The file is written and read by different builds, so the field names are
/// part of the contract.
#[test]
fn the_saved_shape_is_camel_case_json() {
    let json = serde_json::to_string(&placed(1.0, 2.0, 3.0, 4.0)).expect("serialize");
    assert!(json.contains("\"width\""), "got {json}");
    assert!(json.contains("\"maximized\""), "got {json}");
    let back: WindowPlacement = serde_json::from_str(&json).expect("round trip");
    assert_eq!(back, placed(1.0, 2.0, 3.0, 4.0));
}

/// A file written before this field existed has no `maximized`, and must
/// still load.
#[test]
fn an_older_file_without_maximized_still_loads() {
    let back: WindowPlacement =
        serde_json::from_str(r#"{"x":10,"y":20,"width":1200,"height":800}"#).expect("older file");
    assert!(!back.maximized);
    assert_eq!(f32::from(back.rect().size.width), 1200.0);
}
