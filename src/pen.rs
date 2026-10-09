//! Raw-evdev reader for the reMarkable Paper Pure pen ("Elan marker input").
//!
//! The QTFB socket only forwards the *touchscreen* (finger) as `INPUT_TOUCH_*`,
//! which the UI uses for buttons. The Wacom-style marker is a separate evdev
//! device, so the whiteboard reads it directly here — the same approach
//! `fastterm/src/input.rs` uses for the keyd virtual keyboard.
//!
//! We resolve the device by `EVIOCGNAME` (the event index is NOT stable across
//! OTA/reboot — see memory `keyd event-index NOT stable`), open it `O_NONBLOCK`
//! so it can be `poll()`-multiplexed with the QTFB socket, read the ABS ranges at
//! runtime via `EVIOCGABS`, and map digitizer coords → the 1404×1872 portrait
//! panel. Pressure drives stroke width; we ink only while the pen is in contact.

use crate::device::{AxisRange, PANEL_HEIGHT, PANEL_WIDTH};
use std::io;
use std::os::unix::io::RawFd;

// evdev event/axis codes.
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0x00;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_PRESSURE: u16 = 0x18;
const BTN_TOUCH: u16 = 0x14a; // 330 — pen in contact with the surface
const BTN_TOOL_PEN: u16 = 0x140; // 320 — pen tip in proximity (hover)
const BTN_TOOL_RUBBER: u16 = 0x141; // 321 — pen flipped: eraser end in proximity

// EVIOCGRAB — take the marker exclusively so backgrounded xochitl doesn't also
// process it (mirrors fastScratch). `_IOW('E', 0x90, int)`.
const EVIOCGRAB: libc::Ioctl = 0x4004_4590u64 as libc::Ioctl;

// Panel geometry (native portrait). Matches ui::WIDTH / ui::HEIGHT.

fn eviocgname(len: usize) -> libc::Ioctl {
    (0x8000_0000u64 | ((len as u64) << 16) | (0x45 << 8) | 0x06) as libc::Ioctl
}
fn eviocgabs(axis: u16) -> libc::Ioctl {
    // _IOR('E', 0x40+axis, struct input_absinfo) — 6× i32 = 24 bytes.
    ((2u64 << 30) | (24 << 16) | (0x45 << 8) | (0x40 + axis as u64)) as libc::Ioctl
}

#[repr(C)]
struct InputEvent {
    tv_sec: i64,
    tv_usec: i64,
    type_: u16,
    code: u16,
    value: i32,
}

/// One inking sample in panel coordinates, emitted on each `SYN_REPORT` while the
/// pen is (or just became / just stopped being) in contact.
#[derive(Clone, Copy, Debug)]
pub struct PenSample {
    pub x: i32,
    pub y: i32,
    pub w: u8,
    pub down: bool,  // first contact sample of a stroke
    pub up: bool,    // pen lifted (x/y are the last known point)
    pub erase: bool, // the eraser end (flipped pen) is in use
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PenWeight {
    Thin,
    #[default]
    Medium,
    Bold,
}

impl PenWeight {
    pub fn width(self, pressure_width: u8) -> u8 {
        let pressure = pressure_width.clamp(2, 6) - 2;
        match self {
            Self::Thin => 2 + pressure / 2,
            Self::Medium => 4 + pressure / 2,
            Self::Bold => 7 + pressure,
        }
    }
}

#[derive(Default)]
pub struct PenTap {
    origin: Option<(i32, i32)>,
}

impl PenTap {
    pub fn feed(&mut self, sample: PenSample) -> Option<(i32, i32)> {
        if sample.down {
            self.origin = if sample.erase {
                None
            } else {
                Some((sample.x, sample.y))
            };
        }
        if let Some((x, y)) = self.origin {
            if (sample.x - x).abs() > 24 || (sample.y - y).abs() > 24 {
                self.origin = None;
            }
        }
        if sample.up {
            self.origin.take()
        } else {
            None
        }
    }
}

pub struct Pen {
    fd: RawFd,
    grabbed: bool,
    x_range: AxisRange,
    y_range: AxisRange,
    pressure_range: AxisRange,
    // accumulated axis state within the current SYN frame
    x: i32,
    y: i32,
    pressure: i32,
    contact: bool,     // currently touching
    was_contact: bool, // contact at last emitted sample
    rubber: bool,      // eraser end (BTN_TOOL_RUBBER) in proximity
    prox: bool,        // either end reported in proximity (hovering)
    activity: bool,    // any marker traffic since the last take_activity()
    flip180: bool,     // scene is portrait-inverted: point-mirror every sample
    pub dev_path: String,
}

impl Pen {
    /// Resolve + open the marker device (non-blocking). `Err` if not found — the
    /// caller degrades to a touch-only UI.
    pub fn open() -> io::Result<Self> {
        let (fd, dev_path) = resolve_marker()?;
        // Non-blocking so drain() never stalls the poll loop.
        unsafe {
            let fl = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, fl | libc::O_NONBLOCK);
        }
        let ranges = (|| {
            Ok::<_, io::Error>((
                abs_range(fd, ABS_X)?,
                abs_range(fd, ABS_Y)?,
                abs_range(fd, ABS_PRESSURE)?,
            ))
        })();
        let (x_range, y_range, pressure_range) = match ranges {
            Ok(ranges) => ranges,
            Err(error) => {
                unsafe { libc::close(fd) };
                return Err(error);
            }
        };
        // Grab exclusively (best-effort). Released on close / process exit.
        let grabbed = unsafe { libc::ioctl(fd, EVIOCGRAB, 1 as libc::c_int) } == 0;
        Ok(Self {
            fd,
            grabbed,
            x_range,
            y_range,
            pressure_range,
            x: 0,
            y: 0,
            pressure: 0,
            contact: false,
            was_contact: false,
            rubber: false,
            prox: false,
            activity: false,
            flip180: false,
            dev_path,
        })
    }

    pub fn fd(&self) -> RawFd {
        self.fd
    }

    /// True while either pen end is in proximity of the glass (hovering), i.e.
    /// `BTN_TOOL_PEN`/`BTN_TOOL_RUBBER` are asserted but the tip may not be down
    /// yet. This is the signal that makes palm rejection work *before* the first
    /// ink sample: the hand lands on the panel while the pen is still approaching,
    /// so a hover means any concurrent finger contact is a palm.
    ///
    /// Callers must pair this with a recency check on `take_activity()` — if the
    /// pen leaves without a closing `BTN_TOOL_*` report, proximity would otherwise
    /// stay latched and lock out every finger touch.
    pub fn in_proximity(&self) -> bool {
        self.prox
    }

    /// Whether any marker event at all (including a bare hover move, which emits no
    /// `PenSample`) arrived since the last call. Clears the flag.
    pub fn take_activity(&mut self) -> bool {
        std::mem::take(&mut self.activity)
    }

    /// Toggle the exclusive grab: `true` re-takes EVIOCGRAB, `false` releases it
    /// (so another process — the scratchpad overlay — can grab the marker). No-op
    /// when already in the wanted state. Returns the resulting grab state (`false`
    /// if taking the grab failed, e.g. someone else currently holds it).
    pub fn set_grab(&mut self, want: bool) -> bool {
        if want != self.grabbed {
            let arg: libc::c_int = if want { 1 } else { 0 };
            if unsafe { libc::ioctl(self.fd, EVIOCGRAB, arg) } == 0 {
                self.grabbed = want;
            }
        }
        self.grabbed
    }

    /// Track AppLoad's scene rotation. Point-mirroring both axes is
    /// the 180° rotation, so ink lands under the physical pen tip again once the
    /// host composites the buffer rotated.
    pub fn set_flip180(&mut self, flip: bool) {
        self.flip180 = flip;
    }

    fn map_x(&self, raw: i32) -> i32 {
        self.x_range.to_pixel(raw, PANEL_WIDTH, self.flip180)
    }
    fn map_y(&self, raw: i32) -> i32 {
        self.y_range.to_pixel(raw, PANEL_HEIGHT, self.flip180)
    }
    fn width(&self) -> u8 {
        // 2..=6 px from pressure.
        (2 + self.pressure_range.to_pixel(self.pressure, 5, false)) as u8
    }

    fn sample_event(&mut self, type_: u16, code: u16, value: i32) -> Option<PenSample> {
        self.activity = true;
        match type_ {
            EV_ABS => match code {
                ABS_X => self.x = value,
                ABS_Y => self.y = value,
                ABS_PRESSURE => self.pressure = value,
                _ => {}
            },
            EV_KEY => match code {
                BTN_TOUCH => {
                    self.contact = value != 0;
                    if !self.contact {
                        self.pressure = 0;
                    }
                }
                BTN_TOOL_RUBBER | BTN_TOOL_PEN => {
                    self.prox = value != 0;
                    self.rubber = code == BTN_TOOL_RUBBER && self.prox;
                    if !self.prox {
                        self.contact = false;
                        self.pressure = 0;
                    }
                }
                _ => {}
            },
            EV_SYN if code == SYN_REPORT => {
                let contact = self.contact || self.pressure > 0;
                let sample = if contact || self.was_contact {
                    Some(PenSample {
                        x: self.map_x(self.x),
                        y: self.map_y(self.y),
                        w: self.width(),
                        down: contact && !self.was_contact,
                        up: !contact,
                        erase: self.rubber,
                    })
                } else {
                    None
                };
                self.was_contact = contact;
                return sample;
            }
            _ => {}
        }
        None
    }

    /// Read all pending events (non-blocking) and invoke `f` once per completed
    /// `SYN_REPORT` that involves pen contact (a down/move/up sample). Returns
    /// `Ok(())` on EAGAIN (nothing more to read); `Err` on a real read error/EOF.
    pub fn drain<F: FnMut(PenSample)>(&mut self, mut f: F) -> io::Result<()> {
        let sz = std::mem::size_of::<InputEvent>();
        let mut buf = [0u8; std::mem::size_of::<InputEvent>() * 64];
        loop {
            let n =
                unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n < 0 {
                let e = io::Error::last_os_error();
                return match e.raw_os_error() {
                    Some(c) if c == libc::EAGAIN || c == libc::EWOULDBLOCK => Ok(()),
                    _ => Err(e),
                };
            }
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "marker closed",
                ));
            }
            let count = (n as usize) / sz;
            for i in 0..count {
                let ev =
                    unsafe { (buf.as_ptr().add(i * sz) as *const InputEvent).read_unaligned() };
                if let Some(sample) = self.sample_event(ev.type_, ev.code, ev.value) {
                    f(sample);
                }
            }
        }
    }
}

impl Drop for Pen {
    fn drop(&mut self) {
        unsafe {
            if self.grabbed {
                libc::ioctl(self.fd, EVIOCGRAB, 0 as libc::c_int);
            }
            libc::close(self.fd);
        }
    }
}

fn abs_range(fd: RawFd, axis: u16) -> io::Result<AxisRange> {
    let mut info = [0i32; 6]; // value,min,max,fuzz,flat,resolution
    let rc = unsafe { libc::ioctl(fd, eviocgabs(axis), info.as_mut_ptr() as *mut libc::c_void) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    AxisRange::new(info[1], info[2])
}

fn resolve_marker() -> io::Result<(RawFd, String)> {
    for i in 0..32 {
        let path = format!("/dev/input/event{i}");
        let c = std::ffi::CString::new(path.clone()).unwrap();
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY) };
        if fd < 0 {
            continue;
        }
        let mut name = [0u8; 256];
        let rc = unsafe {
            libc::ioctl(
                fd,
                eviocgname(name.len()),
                name.as_mut_ptr() as *mut libc::c_void,
            )
        };
        if rc >= 0 {
            let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            let n = String::from_utf8_lossy(&name[..end]).to_lowercase();
            // "Elan marker input" — exclude the "Elan touch input" digitizer.
            if n.contains("marker") || (n.contains("elan") && n.contains("pen")) {
                return Ok((fd, path));
            }
        }
        unsafe { libc::close(fd) };
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no Elan marker input device found",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pen() -> Pen {
        Pen {
            fd: -1,
            grabbed: false,
            x_range: AxisRange::new(0, 9620).unwrap(),
            y_range: AxisRange::new(0, 13000).unwrap(),
            pressure_range: AxisRange::new(0, 4096).unwrap(),
            x: 3000,
            y: 4000,
            pressure: 0,
            contact: false,
            was_contact: false,
            rubber: false,
            prox: false,
            activity: false,
            flip180: false,
            dev_path: String::new(),
        }
    }

    #[test]
    fn leaving_proximity_ends_contact_even_when_pressure_release_is_missing() {
        let mut pen = pen();
        pen.sample_event(EV_KEY, BTN_TOOL_PEN, 1);
        pen.sample_event(EV_ABS, ABS_PRESSURE, 1000);
        assert!(pen.sample_event(EV_SYN, SYN_REPORT, 0).unwrap().down);
        pen.sample_event(EV_KEY, BTN_TOOL_PEN, 0);
        assert!(pen.sample_event(EV_SYN, SYN_REPORT, 0).unwrap().up);
        assert!(pen.sample_event(EV_SYN, SYN_REPORT, 0).is_none());
        assert!(!pen.in_proximity());
    }

    #[test]
    fn tip_release_does_not_join_separate_letters() {
        let mut pen = pen();
        pen.sample_event(EV_ABS, ABS_PRESSURE, 1000);
        pen.sample_event(EV_KEY, BTN_TOUCH, 1);
        assert!(pen.sample_event(EV_SYN, SYN_REPORT, 0).unwrap().down);
        pen.sample_event(EV_KEY, BTN_TOUCH, 0);
        assert!(pen.sample_event(EV_SYN, SYN_REPORT, 0).unwrap().up);
        pen.sample_event(EV_KEY, BTN_TOUCH, 1);
        assert!(pen.sample_event(EV_SYN, SYN_REPORT, 0).unwrap().down);
    }

    #[test]
    fn pen_buttons_fire_once_on_lift_and_writing_never_counts_as_a_tap() {
        let sample = PenSample {
            x: 300,
            y: 50,
            w: 3,
            down: true,
            up: false,
            erase: false,
        };
        let mut tap = PenTap::default();
        assert_eq!(tap.feed(sample), None);
        assert_eq!(
            tap.feed(PenSample {
                down: false,
                up: true,
                ..sample
            }),
            Some((300, 50))
        );
        assert_eq!(
            tap.feed(PenSample {
                down: false,
                up: true,
                ..sample
            }),
            None
        );
        tap.feed(sample);
        tap.feed(PenSample {
            x: 350,
            down: false,
            ..sample
        });
        assert_eq!(
            tap.feed(PenSample {
                down: false,
                up: true,
                ..sample
            }),
            None
        );
    }

    #[test]
    fn weights_are_distinct_even_at_light_pressure() {
        assert_eq!(PenWeight::Thin.width(2), 2);
        assert_eq!(PenWeight::Medium.width(2), 4);
        assert_eq!(PenWeight::Bold.width(2), 7);
        assert!(PenWeight::Bold.width(6) > PenWeight::Bold.width(2));
    }
}
