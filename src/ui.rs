//! Review UI — screen layout, HTML chrome generation, and touch hit-testing.
//!
//! The screen is composited from three independent blitz renders blitted into the
//! QTFB framebuffer: a top status bar, the card body (Anki's own HTML/CSS), and a
//! bottom action bar. Keeping the card in its own document stops Anki's per-deck
//! `body{}`/`.card{}` CSS from bleeding into our chrome. Hit-rects are defined here
//! in Rust to match the geometry of the HTML we generate.

use crate::backend::{CardKind, Counts, DeckInfo, Grade, ReviewCard, Stats};
use crate::pen::PenWeight;
use crate::prefs::Prefs;
use crate::qtfb::{Qtfb, REFRESH_MODE_CONTENT};
use crate::render::Renderer;

pub use crate::device::{PANEL_HEIGHT as HEIGHT, PANEL_WIDTH as WIDTH};
pub const NAV_H: usize = 110;
pub const TOP_H: usize = NAV_H * 2;
pub const BOTTOM_H: usize = 180;
pub const CARD_Y: usize = TOP_H;
pub const CARD_H: usize = HEIGHT - TOP_H - BOTTOM_H;

const HOME_W: usize = 160;
const SYNC_W: usize = 150;
const EXIT_W: usize = 140;

const TOOLS: [(Hit, &str, usize); 8] = [
    (Hit::PenWeight(PenWeight::Thin), "Thin", 164),
    (Hit::PenWeight(PenWeight::Medium), "Medium", 188),
    (Hit::PenWeight(PenWeight::Bold), "Bold", 164),
    (Hit::EraserToggle, "Eraser", 180),
    (Hit::Undo, "Undo", 164),
    (Hit::Redo, "Redo", 164),
    (Hit::Clear, "Clear", 160),
    (Hit::TextWeightToggle, "Bold text", 220),
];

// ⋮ overflow menu: a floating button in the card's bottom-right corner, with a
// popup panel above it (root actions, or the Flag… submenu).
pub const MENU_BTN: usize = 104;
const MENU_GAP: usize = 30; // inset from the right edge / bottom action bar
const MENU_PANEL_W: usize = 520;
const MENU_ITEM_H: usize = 118;

/// Which ⋮ panel is showing (`Closed` = none).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuState {
    Closed,
    Root,
    Flags,
}

/// Anki's seven card flags: (flag number, name). Single source of
/// truth — BOTH the Flags panel's HTML rows and hit_menu's row indices derive
/// from this table (in this order), so the drawn rows and the hit-test rows
/// cannot drift.
const FLAGS: [(u32, &str); 7] = [
    (1, "Red"),
    (2, "Orange"),
    (3, "Green"),
    (4, "Blue"),
    (5, "Pink"),
    (6, "Turquoise"),
    (7, "Purple"),
];

/// Rows in the panel for a given state. Root: Flag… / Bury card / Suspend note.
/// Flags: the seven flags + Remove flag.
fn menu_items(state: MenuState) -> usize {
    match state {
        MenuState::Closed => 0,
        MenuState::Root => 4,
        MenuState::Flags => FLAGS.len() + 1,
    }
}

fn menu_btn_rect() -> (usize, usize, usize, usize) {
    let x1 = WIDTH - MENU_GAP;
    let x0 = x1 - MENU_BTN;
    let y1 = HEIGHT - BOTTOM_H - MENU_GAP;
    let y0 = y1 - MENU_BTN;
    (x0, y0, x1, y1)
}

fn menu_panel_rect(items: usize) -> (usize, usize, usize, usize) {
    let (_, by0, bx1, _) = menu_btn_rect();
    let x1 = bx1;
    let x0 = x1 - MENU_PANEL_W;
    let y1 = by0 - 14;
    let y0 = y1 - MENU_ITEM_H * items;
    (x0, y0, x1, y1)
}

/// Hit-test a tap while the ⋮ menu is open: a panel item, else None (dismiss).
/// Row indices are matched exhaustively per state — no catch-all mapping — so a
/// tap outside the drawn rows can never fire an action.
pub fn hit_menu(x: i32, y: i32, state: MenuState) -> MenuHit {
    if x < 0 || y < 0 {
        return MenuHit::None;
    }
    let (x, y) = (x as usize, y as usize);
    let (px0, py0, px1, py1) = menu_panel_rect(menu_items(state));
    if x >= px0 && x < px1 && y >= py0 && y < py1 {
        let row = (y - py0) / MENU_ITEM_H;
        return match state {
            MenuState::Closed => MenuHit::None,
            MenuState::Root => match row {
                0 => MenuHit::OpenFlags,
                1 => MenuHit::Bury,
                2 => MenuHit::Suspend,
                3 => MenuHit::UndoReview,
                _ => MenuHit::None,
            },
            // Same order as draw_menu_panel: the FLAGS rows, then Remove flag.
            MenuState::Flags => match row {
                r if r < FLAGS.len() => MenuHit::Flag(FLAGS[r].0),
                r if r == FLAGS.len() => MenuHit::Flag(0),
                _ => MenuHit::None,
            },
        };
    }
    MenuHit::None
}

// Home-screen deck list geometry (hit_test_home must match compose_home's CSS).
pub const HOME_ROW_H: usize = 116;
const HOME_INDENT: usize = 46; // px per tree level
const HOME_PAD_L: usize = 32; // list left padding
const HOME_CHEV_W: usize = 84; // chevron tap zone width (after indent)
const SCROLL_BTN_W: usize = 88; // ▲/▼ header buttons
/// Visible deck rows that fit under the header.
pub fn home_page_rows() -> usize {
    (HEIGHT - NAV_H - HOME_ROW_H) / HOME_ROW_H
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Question,
    Answer,
}

#[derive(Clone, Copy, Debug)]
pub enum Hit {
    ShowAnswer,
    Grade(Grade),
    Sync,
    Exit,
    Home,
    Undo,
    Redo,
    PenWeight(PenWeight),
    TextWeightToggle,
    Clear,
    EraserToggle,
    /// The ⋮ overflow button (bottom-right) was tapped.
    Menu,
    None,
}

/// A tap while the ⋮ overflow menu is open.
#[derive(Clone, Copy, Debug)]
pub enum MenuHit {
    /// Root row "Flag…" — switch the panel to the flags submenu (stays open).
    OpenFlags,
    Bury,
    Suspend,
    UndoReview,
    /// A flags-submenu row: 1-7 sets that flag, 0 = "Remove flag".
    Flag(u32),
    /// Tapped outside the panel — dismiss it.
    None,
}

/// A tap on the home screen. `Deck`/`Toggle` carry the absolute index into the
/// visible deck list.
#[derive(Clone, Copy, Debug)]
pub enum HomeHit {
    Deck(usize),
    Toggle(usize),
    Sync,
    Exit,
    ScrollUp,
    ScrollDown,
    None,
}

/// Map a touch in physical panel coords to a UI action for the current phase.
pub fn hit_test(x: i32, y: i32, phase: Phase) -> Hit {
    if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
        return Hit::None;
    }
    let (x, y) = (x as usize, y as usize);

    if y < NAV_H {
        if x < HOME_W {
            return Hit::Home;
        }
        let mut edge = WIDTH;
        edge -= EXIT_W;
        if x >= edge {
            return Hit::Exit;
        }
        edge -= SYNC_W;
        if x >= edge {
            return Hit::Sync;
        }
        // Counts and the deck name are informational.
        return Hit::None;
    }

    if y < TOP_H {
        let mut edge = 0;
        for (hit, _, width) in TOOLS {
            edge += width;
            if x < edge {
                return hit;
            }
        }
        return Hit::None;
    }

    // Bottom action bar.
    if y >= HEIGHT - BOTTOM_H {
        return match phase {
            Phase::Question => Hit::ShowAnswer,
            Phase::Answer => {
                let q = WIDTH / 4;
                let idx = (x / q).min(3);
                Hit::Grade(match idx {
                    0 => Grade::Again,
                    1 => Grade::Hard,
                    2 => Grade::Good,
                    _ => Grade::Easy,
                })
            }
        };
    }

    // The ⋮ overflow button floats in the card's bottom-right corner — a small,
    // deliberate target checked BEFORE the card-region palm ignore below.
    let (mx0, my0, mx1, my1) = menu_btn_rect();
    if x >= mx0 && x < mx1 && y >= my0 && y < my1 {
        return Hit::Menu;
    }

    // Card region: IGNORE touches. The pen draws here and the palm rests here while
    // writing — a card tap must NOT reveal the answer or act (that fired "show
    // answer on its own"). Reveal is the deliberate Show Answer button only.
    let _ = phase;
    Hit::None
}

/// Is (x,y) inside the scrollable card region? Excludes chrome AND the floating
/// ⋮ button (which must stay tappable — gesture routing consumes card-region
/// touches before hit_test runs).
pub fn in_card_region(x: i32, y: i32) -> bool {
    if x < 0 || (x as usize) >= WIDTH || y < CARD_Y as i32 || (y as usize) >= HEIGHT - BOTTOM_H {
        return false;
    }
    let (mx0, my0, mx1, my1) = menu_btn_rect();
    let (xu, yu) = (x as usize, y as usize);
    !(xu >= mx0 && xu < mx1 && yu >= my0 && yu < my1)
}

/// Composite a full review frame into a fresh WIDTH×HEIGHT RGBA buffer (white bg).
/// `card_window` is the pre-painted card region (WIDTH×CARD_H RGBA) from the live
/// `CardView` — the caller owns scroll/zoom state, we just composite.
pub fn compose_review(
    renderer: &Renderer,
    card_window: &[u8],
    card: &ReviewCard,
    phase: Phase,
    counts: Counts,
    status: &str,
    eraser: bool,
    prefs: &Prefs,
    menu: MenuState,
) -> Vec<u8> {
    let mut fb = vec![255u8; WIDTH * HEIGHT * 4];

    blit(&mut fb, card_window, WIDTH, CARD_H, 0, CARD_Y);

    let top_buf = renderer.render_rgba(
        &top_bar_html(counts, Some(card.kind), status, eraser, prefs),
        WIDTH as u32,
        TOP_H as u32,
    );
    blit(&mut fb, &top_buf, WIDTH, TOP_H, 0, 0);

    let bot_buf =
        renderer.render_rgba(&bottom_bar_html(card, phase), WIDTH as u32, BOTTOM_H as u32);
    blit(&mut fb, &bot_buf, WIDTH, BOTTOM_H, 0, HEIGHT - BOTTOM_H);

    hline_black(&mut fb, CARD_Y - 2, 2);
    hline_black(&mut fb, HEIGHT - BOTTOM_H, 2);

    // ⋮ overflow button (always) + its popup panel (when open), drawn last so they
    // sit on top of the card.
    draw_menu_button(&mut fb, renderer);
    if menu != MenuState::Closed {
        draw_menu_panel(&mut fb, renderer, menu, card.flags);
    }
    // Keep image and card-template contrast consistent on the monochrome panel.
    desaturate(&mut fb);
    fb
}

/// Patch ONLY the card band of an existing composed frame (`wb.base`) with a
/// freshly painted card window — used by scroll/zoom steps so the chrome isn't
/// re-rendered per step. Re-draws the ⋮ button (it floats over the card) and
/// re-applies black-and-white mode to the band.
pub fn patch_card_band(
    fb: &mut [u8],
    card_window: &[u8],
    renderer: &Renderer,
    menu: MenuState,
    current_flag: u8,
) {
    // Reset the band to white, then alpha-composite the window (matches the
    // full-compose path, which blits over a white frame).
    let band = &mut fb[CARD_Y * WIDTH * 4..(CARD_Y + CARD_H) * WIDTH * 4];
    for px in band.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 255, 255, 255]);
    }
    blit(fb, card_window, WIDTH, CARD_H, 0, CARD_Y);
    draw_menu_button(fb, renderer);
    if menu != MenuState::Closed {
        draw_menu_panel(fb, renderer, menu, current_flag);
    }
    desaturate(&mut fb[CARD_Y * WIDTH * 4..(CARD_Y + CARD_H) * WIDTH * 4]);
}

/// Draw a thin scroll-position indicator along the card window's right edge
/// (drawn into the WIDTH×CARD_H window buffer itself, before compositing).
pub fn draw_scroll_indicator(window: &mut [u8], scroll: f64, max_scroll: f64, content_h: f64) {
    if max_scroll <= 0.0 || content_h <= 0.0 {
        return;
    }
    let track_h = CARD_H as f64;
    let thumb_h = ((track_h / content_h) * track_h).max(60.0);
    let thumb_y = (scroll / max_scroll) * (track_h - thumb_h);
    let (y0, y1) = (thumb_y as usize, (thumb_y + thumb_h) as usize);
    let (x0, x1) = (WIDTH - 8, WIDTH - 2);
    for y in y0..y1.min(CARD_H) {
        for x in x0..x1 {
            let i = (y * WIDTH + x) * 4;
            window[i] = 60;
            window[i + 1] = 60;
            window[i + 2] = 60;
            window[i + 3] = 255;
        }
    }
}

/// Desaturate an RGBA buffer in place (luminance grey) for black-and-white mode.
fn desaturate(buf: &mut [u8]) {
    for px in buf.chunks_exact_mut(4) {
        // Rec.601 luma; keep alpha.
        let l = ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8;
        px[0] = l;
        px[1] = l;
        px[2] = l;
    }
}

/// The floating ⋮ button in the card's bottom-right corner.
fn draw_menu_button(fb: &mut [u8], renderer: &Renderer) {
    let (x0, y0, _, _) = menu_btn_rect();
    let html = chrome_page(&format!(
        r#"<div class="flex items-center justify-center bg-white text-black border-[3px] border-black text-[30px] font-bold box-border" style="width:{MENU_BTN}px;height:{MENU_BTN}px;">More</div>"#
    ));
    let buf = renderer.render_rgba(&html, MENU_BTN as u32, MENU_BTN as u32);
    blit(fb, &buf, MENU_BTN, MENU_BTN, x0, y0);
}

/// The popup panel above the ⋮ button. Root: Flag… / Bury card / Suspend note;
/// Flags: one labeled row per `FLAGS` entry, with a check on
/// the current flag) then "Remove flag". Row ORDER must match `hit_menu` — both
/// are generated from the same `FLAGS` table.
fn draw_menu_panel(fb: &mut [u8], renderer: &Renderer, state: MenuState, current_flag: u8) {
    let items = menu_items(state);
    if items == 0 {
        return;
    }
    let (x0, y0, x1, y1) = menu_panel_rect(items);
    let (w, h) = (x1 - x0, y1 - y0);
    let labels: Vec<String> = match state {
        MenuState::Closed => return,
        MenuState::Root => vec![
            "Flag card ›".into(),
            "Bury card".into(),
            "Suspend note".into(),
            "Undo last review".into(),
        ],
        MenuState::Flags => FLAGS
            .iter()
            .map(|(n, name)| {
                format!(
                    "{} {n} {name}",
                    if *n == current_flag as u32 { "✓" } else { "" }
                )
            })
            .chain(std::iter::once("Remove flag".into()))
            .collect(),
    };
    let rows = labels.iter().map(|label| format!(r#"<div class="flex items-center px-[32px] border-b-[2px] border-black text-[38px] font-bold box-border" style="height:{MENU_ITEM_H}px;">{}</div>"#, esc(label))).collect::<String>();
    let html = chrome_page(&format!(
        r#"<div class="bg-white border-[3px] border-black box-border" style="width:{w}px;height:{h}px;">{rows}</div>"#
    ));
    let buf = renderer.render_rgba(&html, w as u32, h as u32);
    blit(fb, &buf, w, h, x0, y0);
}

/// Composite + push a full review frame (renders the card window itself at
/// scroll 0 / zoom 1 — the app's live path goes through `main::redraw` instead).

/// The Review screen with no due card ("All done" / an empty deck): the top menu
/// bar (so Home / Sync / Exit still work) + a centred message filling the rest.
pub fn compose_review_empty(
    renderer: &Renderer,
    counts: Counts,
    status: &str,
    title: &str,
    body: &str,
) -> Vec<u8> {
    let mut fb = vec![255u8; WIDTH * HEIGHT * 4];
    let area_h = HEIGHT - TOP_H;
    let html = chrome_page(&format!(
        r#"<div class="flex flex-col items-center justify-center text-center px-[120px] box-border" style="height:{area_h}px;">
        <div class="text-[60px] font-bold mb-[24px]">{}</div><div class="text-[34px] leading-normal max-w-[1100px]">{}</div></div>"#,
        esc(title),
        esc(body)
    ));
    let area = renderer.render_rgba(&html, WIDTH as u32, area_h as u32);
    blit(&mut fb, &area, WIDTH, area_h, 0, TOP_H);
    let top = renderer.render_rgba(
        &top_bar_html(counts, None, status, false, &Prefs::default()),
        WIDTH as u32,
        TOP_H as u32,
    );
    blit(&mut fb, &top, WIDTH, TOP_H, 0, 0);
    hline_black(&mut fb, TOP_H - 2, 2);
    fb
}

/// Full-screen message (congrats / errors) into a fresh RGBA buffer.
pub fn compose_message(renderer: &Renderer, title: &str, body: &str) -> Vec<u8> {
    let html = chrome_page(&format!(
        r#"<div class="flex flex-col items-center justify-center text-center px-[120px] box-border" style="height:{HEIGHT}px;">
        <div class="text-[64px] font-bold mb-[28px]">{}</div>
        <div class="text-[36px] leading-normal max-w-[1100px]">{}</div></div>"#,
        esc(title),
        esc(body)
    ));
    renderer.render_rgba(&html, WIDTH as u32, HEIGHT as u32)
}

/// Deck selection and navigation share the exact painted row geometry.

pub fn hit_test_home(x: i32, y: i32, visible: &[DeckInfo], scroll: usize) -> HomeHit {
    if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
        return HomeHit::None;
    }
    let (x, y) = (x as usize, y as usize);
    if y < NAV_H {
        let mut edge = WIDTH;
        edge -= EXIT_W;
        if x >= edge {
            return HomeHit::Exit;
        }
        edge -= SYNC_W;
        if x >= edge {
            return HomeHit::Sync;
        }
        edge -= SCROLL_BTN_W;
        if x >= edge {
            return HomeHit::ScrollDown;
        }
        edge -= SCROLL_BTN_W;
        if x >= edge {
            return HomeHit::ScrollUp;
        }
        return HomeHit::None;
    }
    if y < NAV_H + HOME_ROW_H {
        return HomeHit::None;
    }
    let idx = scroll + (y - NAV_H - HOME_ROW_H) / HOME_ROW_H;
    let Some(d) = visible.get(idx) else {
        return HomeHit::None;
    };
    // Chevron zone (parents only) toggles collapse; the rest reviews the deck.
    if d.has_children {
        let indent = HOME_PAD_L + d.level as usize * HOME_INDENT;
        if x >= indent && x < indent + HOME_CHEV_W {
            return HomeHit::Toggle(idx);
        }
    }
    HomeHit::Deck(idx)
}

/// Home screen: header (title + streak + scroll + Sync/Exit) over a collapsible,
/// paged deck tree. Renders `visible[scroll ..]` for one page.
pub fn compose_home(
    renderer: &Renderer,
    visible: &[DeckInfo],
    scroll: usize,
    stats: Stats,
) -> Vec<u8> {
    let fit = home_page_rows();
    let end = (scroll + fit).min(visible.len());
    let rows = visible.get(scroll..end).unwrap_or(&[]).iter().map(|d| {
        let pad = HOME_PAD_L + d.level as usize * HOME_INDENT;
        let arrow = if !d.has_children { "" } else if d.collapsed { "›" } else { "⌄" };
        format!(r#"<div class="flex items-center border-b-[2px] border-black box-border pr-[32px]" style="height:{HOME_ROW_H}px;padding-left:{pad}px;">
            <div class="shrink-0 text-[40px] font-bold" style="width:{HOME_CHEV_W}px;">{arrow}</div>
            <div class="flex-1 min-w-0 truncate text-[40px] font-bold">{}</div>
            <div class="flex gap-[24px] text-[28px]"><span>New <b>{}</b></span><span>Learn <b>{}</b></span><span>Due <b>{}</b></span></div></div>"#, esc(&d.name), d.new, d.learn, d.review)
    }).collect::<String>();
    let controls = [
        ("↑", SCROLL_BTN_W),
        ("↓", SCROLL_BTN_W),
        ("Sync", SYNC_W),
        ("Close", EXIT_W),
    ]
    .iter()
    .map(|(label, w)| nav_button(label, *w))
    .collect::<String>();
    let html = chrome_page(&format!(
        r#"<div class="flex items-center border-b-[3px] border-black box-border" style="height:{NAV_H}px;">
        <div class="text-[44px] font-bold pl-[32px]">Anki</div><div class="flex-1 text-[28px] pl-[24px]">{} reviewed today · {} day streak</div>{controls}</div>
        <div class="flex items-center justify-between px-[32px] text-[28px] border-b-[2px] border-black box-border" style="height:{HOME_ROW_H}px;">
        <b>Choose a deck to start writing</b><span>{}–{} / {}</span></div>{rows}"#,
        stats.reviewed_today,
        stats.streak,
        (scroll + 1).min(visible.len()),
        end,
        visible.len()
    ));
    renderer.render_rgba(&html, WIDTH as u32, HEIGHT as u32)
}

/// Composite + push the home screen.
pub fn draw_home(
    qtfb: &mut Qtfb,
    renderer: &Renderer,
    visible: &[DeckInfo],
    scroll: usize,
    stats: Stats,
) {
    let fb = compose_home(renderer, visible, scroll, stats);
    qtfb.blit_rgba(&fb, WIDTH, HEIGHT, 0, 0);
    let _ = qtfb.set_refresh_mode(REFRESH_MODE_CONTENT);
    let _ = qtfb.update_full();
}

/// Full-screen message (congrats / errors).
pub fn draw_message(qtfb: &mut Qtfb, renderer: &Renderer, title: &str, body: &str) {
    let fb = compose_message(renderer, title, body);
    qtfb.blit_rgba(&fb, WIDTH, HEIGHT, 0, 0);
    let _ = qtfb.set_refresh_mode(REFRESH_MODE_CONTENT);
    let _ = qtfb.update_full();
}

/// Alpha-composite an RGBA `src` (`sw`×`sh`) into RGBA `dst` (WIDTH-wide) at
/// (`x`,`y`) over the existing (white) destination. blitz emits transparent /
/// partially-transparent pixels for unstyled backgrounds and anti-aliased glyph
/// edges; compositing over white keeps backgrounds white and text edges clean
/// (a straight copy would turn transparency black and halo the text).
fn blit(dst: &mut [u8], src: &[u8], sw: usize, sh: usize, x: usize, y: usize) {
    for row in 0..sh {
        let py = y + row;
        if py >= HEIGHT {
            break;
        }
        for col in 0..sw {
            let px = x + col;
            if px >= WIDTH {
                break;
            }
            let si = (row * sw + col) * 4;
            let di = (py * WIDTH + px) * 4;
            let a = src[si + 3] as u32;
            if a == 0 {
                continue; // keep dst (white)
            }
            if a == 255 {
                dst[di] = src[si];
                dst[di + 1] = src[si + 1];
                dst[di + 2] = src[si + 2];
            } else {
                let ia = 255 - a;
                for k in 0..3 {
                    dst[di + k] = ((src[si + k] as u32 * a + dst[di + k] as u32 * ia) / 255) as u8;
                }
            }
            dst[di + 3] = 255;
        }
    }
}

fn hline_black(dst: &mut [u8], y: usize, thickness: usize) {
    for yy in y..(y + thickness).min(HEIGHT) {
        for x in 0..WIDTH {
            let di = (yy * WIDTH + x) * 4;
            dst[di] = 0;
            dst[di + 1] = 0;
            dst[di + 2] = 0;
            dst[di + 3] = 255;
        }
    }
}

const CHROME_CSS: &str = include_str!("../assets/ui.css");

fn chrome_page(content: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>{CHROME_CSS}</style></head><body class="m-0 bg-white text-black font-sans">{content}</body></html>"#
    )
}

fn nav_button(label: &str, width: usize) -> String {
    format!(
        r#"<div class="shrink-0 flex items-center justify-center border-l-[2px] border-black box-border text-[32px] font-bold" style="width:{width}px;height:{NAV_H}px;">{label}</div>"#
    )
}

fn top_bar_html(
    counts: Counts,
    kind: Option<CardKind>,
    status: &str,
    eraser: bool,
    prefs: &Prefs,
) -> String {
    let counts = [("New", counts.new, CardKind::New), ("Learn", counts.learning, CardKind::Learning), ("Due", counts.review, CardKind::Review)].iter().map(|(label, value, k)| {
        let underline = if kind == Some(*k) { "border-b-[4px] border-black" } else { "" };
        format!(r#"<div class="flex flex-col items-center text-[24px] px-[12px]"><span>{label}</span><b class="text-[36px] {underline}">{value}</b></div>"#)
    }).collect::<String>();
    let tools = TOOLS.iter().map(|(hit, label, width)| {
        let selected = match hit {
            Hit::PenWeight(w) => !eraser && *w == prefs.pen_weight,
            Hit::EraserToggle => eraser,
            Hit::TextWeightToggle => prefs.bold_text,
            _ => false,
        };
        let colors = if selected { "bg-black text-white" } else { "bg-white text-black" };
        let line = match hit {
            Hit::PenWeight(w) => {
                let thickness = match w { PenWeight::Thin => 2, PenWeight::Medium => 4, PenWeight::Bold => 8 };
                let color = if selected { "white" } else { "black" };
                format!(r#"<div class="w-[48px] mt-[12px]" style="border-top:{thickness}px solid {color};"></div>"#)
            }
            _ => String::new(),
        };
        format!(r#"<div class="flex flex-col items-center justify-center shrink-0 border-r-[2px] border-black box-border text-[30px] font-bold {colors}" style="width:{width}px;height:{NAV_H}px;"><div>{label}</div>{line}</div>"#)
    }).collect::<String>();
    chrome_page(&format!(
        r#"<div class="flex items-center border-b-[2px] border-black box-border" style="height:{NAV_H}px;">
        {}<div class="flex px-[20px]">{counts}</div><div class="flex-1 min-w-0 truncate text-[28px] px-[16px]">{}</div>{}{}</div>
        <div class="flex border-b-[3px] border-black box-border" style="height:{NAV_H}px;">{tools}</div>"#,
        nav_button("Decks", HOME_W),
        esc(status),
        nav_button("Sync", SYNC_W),
        nav_button("Close", EXIT_W)
    ))
}

fn bottom_bar_html(card: &ReviewCard, phase: Phase) -> String {
    let content = match phase {
        Phase::Question => format!(
            r#"<div class="flex flex-col items-center justify-center bg-black text-white" style="height:{BOTTOM_H}px;">
            <div class="text-[46px] font-bold">Reveal answer</div><div class="text-[26px] mt-[8px]">Your handwriting stays visible</div></div>"#
        ),
        Phase::Answer => {
            let cells = [("Again", "Forgot", &card.button_labels[0]), ("Hard", "Struggled", &card.button_labels[1]), ("Good", "Remembered", &card.button_labels[2]), ("Easy", "Effortless", &card.button_labels[3])].iter().map(|(name, meaning, interval)| {
                let colors = if *name == "Good" { "bg-black text-white" } else { "bg-white text-black" };
                format!(r#"<div class="flex-1 flex flex-col items-center justify-center border-l-[3px] border-black box-border {colors}">
                    <div class="text-[42px] font-bold">{name}</div><div class="text-[24px]">{meaning}</div><div class="text-[28px] font-bold mt-[6px]">{}</div></div>"#, esc(interval))
            }).collect::<String>();
            format!(r#"<div class="flex" style="height:{BOTTOM_H}px;">{cells}</div>"#)
        }
    };
    chrome_page(&content)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend::Backend, demo};

    #[test]
    fn visible_action_buttons_fill_their_touch_regions() {
        let directory =
            std::env::temp_dir().join(format!("pure-action-bar-{}", std::process::id()));
        let path = directory.join("collection.anki2");
        demo::create(&path).unwrap();
        let mut backend = Backend::open(path.to_str().unwrap()).unwrap();
        backend.set_deck_scope(demo::DECK_NAME).unwrap();
        let card = backend.next_card().unwrap().unwrap();
        let renderer = Renderer::new();
        for phase in [Phase::Question, Phase::Answer] {
            let pixels = renderer.render_rgba(
                &bottom_bar_html(&card, phase),
                WIDTH as u32,
                BOTTOM_H as u32,
            );
            for x in [WIDTH / 8, WIDTH * 3 / 8, WIDTH * 5 / 8, WIDTH * 7 / 8] {
                let index = ((BOTTOM_H - 4) * WIDTH
                    + if phase == Phase::Question {
                        x
                    } else {
                        x / (WIDTH / 4) * (WIDTH / 4) + 1
                    })
                    * 4;
                assert!(
                    pixels[index..index + 3].iter().any(|value| *value < 240),
                    "action area must be visible through its bottom edge"
                );
            }
        }
        drop(backend);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn writing_tools_cover_the_toolbar_and_card_taps_never_reveal_answers() {
        assert_eq!(TOOLS.iter().map(|t| t.2).sum::<usize>(), WIDTH);
        let mut edge = 0;
        for (expected, _, width) in TOOLS {
            for x in [edge, edge + width / 2, edge + width - 1] {
                assert_eq!(
                    format!(
                        "{:?}",
                        hit_test(x as i32, NAV_H as i32 + 50, Phase::Question)
                    ),
                    format!("{expected:?}")
                );
            }
            edge += width;
        }
        assert!(matches!(
            hit_test(500, CARD_Y as i32 + 100, Phase::Question),
            Hit::None
        ));
        assert!(matches!(
            hit_test(WIDTH as i32, HEIGHT as i32, Phase::Question),
            Hit::None
        ));
    }

    #[test]
    fn chrome_has_true_monochrome_pixels_and_visible_tool_selection() {
        let renderer = Renderer::new();
        let prefs = Prefs::default();
        let pixels = renderer.render_rgba(
            &top_bar_html(Counts::default(), None, "Sample deck", false, &prefs),
            WIDTH as u32,
            TOP_H as u32,
        );
        assert!(pixels.chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2]));
        let sample = |x: usize| pixels[((NAV_H + 5) * WIDTH + x) * 4];
        assert_eq!(sample(10), 255);
        assert_eq!(sample(174), 0);
        assert_eq!(sample(362), 255);
    }
}
