//! `▀` truecolor pixels: fg = top pixel, bg = bottom pixel (SPEC §8.3).
//! Transparent pixels keep whatever is already in that half-cell, so sprites layer
//! over the scene background and over each other.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use super::sprites::Frame;

const HALF: &str = "▀";

/// Current (top, bottom) colors of a cell. A non-halfblock cell is its bg on both halves.
fn halves(buf: &Buffer, x: u16, y: u16) -> (Color, Color) {
    let c = &buf[(x, y)];
    if c.symbol() == HALF {
        (c.fg, c.bg)
    } else {
        (c.bg, c.bg)
    }
}

pub fn rgb([r, g, b]: [u8; 3]) -> Color {
    Color::Rgb(r, g, b)
}

/// Fill `area` with one flat color (2 pixels per cell).
pub fn fill(buf: &mut Buffer, area: Rect, color: Color) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].reset();
            buf[(x, y)].set_bg(color);
        }
    }
}

/// Draw `frame` with its top-left at cell column `x`, pixel row `py` (relative to
/// `area`, 2 pixel rows per cell). `flip` mirrors horizontally. Clipped to `area`.
pub fn draw(buf: &mut Buffer, area: Rect, frame: &Frame, x: i32, py: i32, flip: bool) {
    for sy in 0..frame.h {
        let pyy = py + sy as i32;
        if pyy < 0 {
            continue;
        }
        let cy = area.y as i32 + pyy / 2;
        if cy >= area.bottom() as i32 {
            break;
        }
        for sx in 0..frame.w {
            let cx = area.x as i32 + x + sx as i32;
            if cx < area.left() as i32 || cx >= area.right() as i32 {
                continue;
            }
            let src = if flip { frame.w - 1 - sx } else { sx };
            let Some(px) = frame.get(src, sy) else {
                continue;
            };
            let (cx, cy) = (cx as u16, cy as u16);
            let (mut top, mut bottom) = halves(buf, cx, cy);
            if pyy % 2 == 0 {
                top = rgb(px);
            } else {
                bottom = rgb(px);
            }
            let cell = &mut buf[(cx, cy)];
            cell.set_symbol(HALF);
            cell.set_fg(top);
            cell.set_bg(bottom);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(px: Vec<Option<[u8; 3]>>, w: u16, h: u16) -> Frame {
        Frame { w, h, px, ms: 100 }
    }

    #[test]
    fn transparency_keeps_background_per_half() {
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        let bg = Color::Rgb(1, 1, 1);
        fill(&mut buf, area, bg);
        let red = Some([255, 0, 0]);
        // 1x1 pixel at pixel row 1 → bottom half of cell (0,0)
        draw(&mut buf, area, &frame(vec![red], 1, 1), 0, 1, false);
        let c = &buf[(0, 0)];
        assert_eq!((c.symbol(), c.fg, c.bg), (HALF, bg, Color::Rgb(255, 0, 0)));
        // transparent pixel leaves cell (1,0) untouched
        draw(&mut buf, area, &frame(vec![None], 1, 1), 1, 0, false);
        assert_eq!(buf[(1, 0)].symbol(), " ");
    }

    #[test]
    fn clips_outside_area() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        let f = frame(vec![Some([9, 9, 9]); 9], 3, 3);
        draw(&mut buf, area, &f, -1, -1, false); // must not panic
        assert_eq!(buf[(0, 0)].fg, Color::Rgb(9, 9, 9));
    }
}
