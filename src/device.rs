use std::io;

pub const PANEL_WIDTH: usize = 1404;
pub const PANEL_HEIGHT: usize = 1872;
pub const FRAMEBUFFER_FORMAT: u8 = 7;

pub fn portrait_inverted(rotation: i32) -> Option<bool> {
    match rotation {
        0 => Some(false),
        3 => Some(true),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AxisRange {
    min: i32,
    max: i32,
}

impl AxisRange {
    pub fn new(min: i32, max: i32) -> io::Result<Self> {
        if max <= min {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid digitizer axis range",
            ));
        }
        Ok(Self { min, max })
    }

    pub fn to_pixel(self, raw: i32, pixels: usize, inverted: bool) -> i32 {
        let last = (pixels - 1) as i64;
        let position = i64::from(raw.clamp(self.min, self.max)) - i64::from(self.min);
        let extent = i64::from(self.max) - i64::from(self.min);
        let mapped = position * last / extent;
        if inverted {
            (last - mapped) as i32
        } else {
            mapped as i32
        }
    }
}
