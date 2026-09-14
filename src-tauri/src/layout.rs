//! Sizing and placing the widget window on whichever display it sits on.
//!
//! The window is undecorated and not user-resizable, so its height has to be
//! driven from the content: the page measures how tall it actually rendered and
//! hands that back through [`fit`]. Everything is clamped to the monitor's work
//! area, which is the screen minus the taskbar.
//!
//! Displays here cannot be assumed to be uniform — this machine mixes a
//! portrait panel with monitors at negative coordinates and runs at 125% — so
//! the maths always goes through the *current* monitor's origin, size and scale
//! factor rather than any global assumption.

use tauri::{LogicalPosition, LogicalSize, Runtime, WebviewWindow};

/// Logical width the widget prefers. Logical units already absorb the display's
/// DPI scaling, so this is the same apparent size at 100% and at 125%.
const PREFERRED_WIDTH: f64 = 320.0;
/// Below these the layout stops being readable, so they win over the work area.
const MIN_WIDTH: f64 = 260.0;
const MIN_HEIGHT: f64 = 200.0;
/// Height used before the page has reported its content height.
const INITIAL_HEIGHT: f64 = 420.0;
/// Gap kept between the widget and the edges of the work area.
const MARGIN: f64 = 16.0;

/// A monitor's usable region, in the logical pixels `set_size` and
/// `set_position` expect.
struct WorkArea {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl WorkArea {
    /// The largest widget that still leaves a margin on both sides, but never
    /// smaller than the readable minimum.
    fn max_size(&self) -> (f64, f64) {
        (
            (self.width - MARGIN * 2.0).max(MIN_WIDTH),
            (self.height - MARGIN * 2.0).max(MIN_HEIGHT),
        )
    }
}

/// The work area of the display the window is currently on.
fn work_area<R: Runtime>(window: &WebviewWindow<R>) -> Option<WorkArea> {
    // The user can drag the widget to any display, so resolve it each time
    // instead of caching the primary monitor.
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;

    let scale = monitor.scale_factor();
    if scale <= 0.0 {
        return None;
    }
    let area = monitor.work_area();
    Some(WorkArea {
        x: f64::from(area.position.x) / scale,
        y: f64::from(area.position.y) / scale,
        width: f64::from(area.size.width) / scale,
        height: f64::from(area.size.height) / scale,
    })
}

/// `f64::clamp` panics when the bounds cross, which they can here if a display
/// is smaller than the widget's minimum. Preferring the low bound keeps the
/// widget anchored to the top-left of the work area in that case.
fn clamp(value: f64, low: f64, high: f64) -> f64 {
    if high < low {
        low
    } else {
        value.clamp(low, high)
    }
}

/// Resizes the window to `content_height` CSS pixels and pulls it back inside
/// the work area, so growing downwards can never push the footer off-screen.
pub fn fit<R: Runtime>(window: &WebviewWindow<R>, content_height: f64) -> tauri::Result<()> {
    if !content_height.is_finite() || content_height <= 0.0 {
        return Ok(());
    }
    let Some(area) = work_area(window) else {
        return Ok(());
    };

    let (max_width, max_height) = area.max_size();
    let width = clamp(PREFERRED_WIDTH, MIN_WIDTH, max_width);
    let height = clamp(content_height.ceil(), MIN_HEIGHT, max_height);

    window.set_size(LogicalSize::new(width, height))?;
    reposition(window, &area, width, height)
}

/// Places the window at the top-right of its work area at a provisional height,
/// before the page has had a chance to measure itself.
pub fn place_initial<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let Some(area) = work_area(window) else {
        return Ok(());
    };
    let (max_width, max_height) = area.max_size();
    let width = clamp(PREFERRED_WIDTH, MIN_WIDTH, max_width);
    let height = clamp(INITIAL_HEIGHT, MIN_HEIGHT, max_height);

    window.set_size(LogicalSize::new(width, height))?;
    window.set_position(LogicalPosition::new(
        area.x + area.width - width - MARGIN,
        area.y + MARGIN,
    ))
}

/// Nudges the window so the whole of it lies within `area`. The bounds are
/// derived from the monitor's own origin, which may be negative.
fn reposition<R: Runtime>(
    window: &WebviewWindow<R>,
    area: &WorkArea,
    width: f64,
    height: f64,
) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    if scale <= 0.0 {
        return Ok(());
    }
    let position = window.outer_position()?;
    let x = clamp(
        f64::from(position.x) / scale,
        area.x + MARGIN,
        area.x + area.width - width - MARGIN,
    );
    let y = clamp(
        f64::from(position.y) / scale,
        area.y + MARGIN,
        area.y + area.height - height - MARGIN,
    );
    window.set_position(LogicalPosition::new(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_prefers_the_low_bound_when_bounds_cross() {
        // A display narrower than MIN_WIDTH inverts the position bounds; the
        // widget should sit at the work-area origin rather than panicking.
        assert_eq!(clamp(50.0, 16.0, -4.0), 16.0);
    }

    #[test]
    fn clamp_behaves_normally_when_bounds_are_sane() {
        assert_eq!(clamp(500.0, 16.0, 300.0), 300.0);
        assert_eq!(clamp(-40.0, 16.0, 300.0), 16.0);
        assert_eq!(clamp(120.0, 16.0, 300.0), 120.0);
    }

    #[test]
    fn max_size_never_drops_below_the_readable_minimum() {
        // A 200x150 work area cannot fit the widget plus margins.
        let tiny = WorkArea { x: 0.0, y: 0.0, width: 200.0, height: 150.0 };
        assert_eq!(tiny.max_size(), (MIN_WIDTH, MIN_HEIGHT));
    }

    #[test]
    fn max_size_leaves_a_margin_on_a_roomy_display() {
        // The 2048x1152 primary at 125% scaling: 1638.4 x 883.2 logical.
        let primary = WorkArea { x: 0.0, y: 0.0, width: 1638.4, height: 883.2 };
        let (w, h) = primary.max_size();
        assert_eq!(w, 1638.4 - 32.0);
        assert_eq!(h, 883.2 - 32.0);
    }
}
