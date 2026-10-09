#[path = "../src/device.rs"]
mod device;

use device::{AxisRange, FRAMEBUFFER_FORMAT, PANEL_HEIGHT, PANEL_WIDTH};

#[test]
fn pen_orientation_uses_appload_rotation_packets() {
    assert_eq!(device::portrait_inverted(0), Some(false));
    assert_eq!(device::portrait_inverted(3), Some(true));
    assert_eq!(device::portrait_inverted(1), None);
    assert_eq!(device::portrait_inverted(2), None);
}

#[test]
fn framebuffer_matches_appload_paper_pure_protocol() {
    assert_eq!((PANEL_WIDTH, PANEL_HEIGHT), (1404, 1872));
    assert_eq!(FRAMEBUFFER_FORMAT, 7);
    assert_eq!(PANEL_WIDTH * PANEL_HEIGHT * 3, 7_884_864);
}

#[test]
fn pen_extremes_land_inside_the_display() {
    let axis = AxisRange::new(100, 11180).unwrap();
    assert_eq!(axis.to_pixel(100, PANEL_WIDTH, false), 0);
    assert_eq!(axis.to_pixel(11180, PANEL_WIDTH, false), 1403);
    assert_eq!(axis.to_pixel(-1, PANEL_WIDTH, false), 0);
    assert_eq!(axis.to_pixel(12000, PANEL_WIDTH, false), 1403);
    assert_eq!(axis.to_pixel(100, PANEL_WIDTH, true), 1403);
    assert_eq!(axis.to_pixel(11180, PANEL_WIDTH, true), 0);
}

#[test]
fn pen_center_and_inverted_orientation_stay_aligned() {
    let axis = AxisRange::new(0, 1000).unwrap();
    let normal = axis.to_pixel(500, PANEL_HEIGHT, false);
    let inverted = axis.to_pixel(500, PANEL_HEIGHT, true);
    assert_eq!(normal + inverted, 1871);
    assert!(normal.abs_diff(inverted) <= 1);
}

#[test]
fn invalid_digitizer_ranges_are_rejected() {
    assert!(AxisRange::new(10, 10).is_err());
    assert!(AxisRange::new(11, 10).is_err());
}
