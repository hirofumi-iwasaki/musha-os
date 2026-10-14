// SPDX-License-Identifier: Apache-2.0
//! Shared physical text grid. Coordinates include the two-cell screen margin.
pub const MARGIN: usize = 2;
pub const LABEL: usize = MARGIN;
pub const VALUE: usize = LABEL + 16;
pub const HEADING: usize = 2;
pub const CORE: usize = 3;
pub const DISCOVERY: usize = 4;
pub const ARENA: usize = 5;
pub const PCI: usize = 6;
pub const XHCI: usize = 7;
pub const LAN: usize = 8;
pub const TIMER: usize = 9;
pub const KEYBOARD: usize = 10;
pub const APP: usize = 11;
pub const KEY: usize = 12;
pub const USB_READ: usize = 14;
pub const USB_BYTES: usize = 15;
pub const FILESYSTEM: usize = 16;
pub const FILE: usize = 17;
pub const NET: usize = 18;
pub const fn scale_for_height(height: usize) -> usize {
    let n = height.saturating_add(300) / 600;
    if n < 1 {
        1
    } else if n > 4 {
        4
    } else {
        n
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub scale: usize,
    pub cell_width: usize,
    pub line_pitch: usize,
    pub columns: usize,
    pub rows: usize,
    pub panel_column: usize,
    pub panel_row: usize,
}
impl Metrics {
    pub const fn new(width: usize, height: usize) -> Self {
        let scale = scale_for_height(height);
        let columns = width / (6 * scale);
        Self {
            scale,
            cell_width: 6 * scale,
            line_pitch: 10 * scale,
            columns,
            rows: height / (10 * scale),
            panel_column: if width >= 1200 { columns / 2 } else { MARGIN },
            panel_row: if width >= 1200 { MARGIN } else { NET + 2 },
        }
    }
    /// Preserve the viewer's fixed 80-column/28-row content on narrow screens.
    pub fn viewer(width: usize, height: usize) -> Self {
        let scale = scale_for_height(height)
            .min(width / (6 * 84))
            .min(height / (10 * 32))
            .max(1);
        Self {
            scale,
            cell_width: 6 * scale,
            line_pitch: 10 * scale,
            columns: width / (6 * scale),
            rows: height / (10 * scale),
            panel_column: MARGIN,
            panel_row: MARGIN,
        }
    }
    pub fn x(self, col: usize) -> usize {
        col.saturating_mul(self.cell_width)
    }
    pub fn y(self, row: usize) -> usize {
        row.saturating_mul(self.line_pitch)
    }
    pub fn status_end_column(self) -> usize {
        if self.panel_row == MARGIN {
            self.panel_column
        } else {
            self.columns.saturating_sub(MARGIN)
        }
    }
    pub fn usable_columns(self) -> usize {
        self.columns.saturating_sub(2 * MARGIN)
    }
    pub fn usable_rows(self) -> usize {
        self.rows.saturating_sub(2 * MARGIN)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolutions() {
        for (w, h, s, c, r, p) in [
            (1024, 768, 1, 170, 76, 2),
            (1280, 800, 1, 213, 80, 106),
            (1600, 1200, 2, 133, 60, 66),
            (1920, 1080, 2, 160, 54, 80),
            (2560, 1600, 3, 142, 53, 71),
            (3360, 2100, 4, 140, 52, 70),
            (3840, 2160, 4, 160, 54, 80),
            (320, 356, 1, 53, 35, 2),
        ] {
            let m = Metrics::new(w, h);
            assert_eq!((m.scale, m.columns, m.rows, m.panel_column), (s, c, r, p));
            assert!(m.y(m.rows - MARGIN - 1) + 8 * m.scale <= h);
            assert!(m.x(m.columns - MARGIN) <= w);
        }
        assert_eq!(scale_for_height(1440), 2);
        assert_eq!(scale_for_height(2880), 4);
    }
    #[test]
    fn small_screen_grid_preserves_margins_and_stride() {
        extern crate std;
        let (w, h, stride) = (320, 356, 323);
        let mut pixels = std::vec![0xdeadbeefu32;stride*h+4];
        let fb = crate::Framebuffer {
            base: pixels.as_mut_ptr() as usize,
            bytes: stride * h * 4,
            width: w,
            height: h,
            stride,
            format: 0,
        };
        let m = Metrics::new(w, h);
        for row in MARGIN..m.rows - MARGIN {
            unsafe {
                fb.text_grid(
                    "LONG GRID ROW 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                    LABEL,
                    row,
                    0xffffff,
                );
            }
        }
        let before = pixels.clone();
        unsafe {
            fb.text_grid("OUTSIDE", LABEL, m.rows - MARGIN, 0);
            fb.text_grid("OUTSIDE", usize::MAX, usize::MAX, 0);
        }
        assert_eq!(before, pixels);
        for y in 0..h {
            for x in 0..stride {
                if x < m.x(MARGIN)
                    || x >= m.x(m.columns - MARGIN)
                    || y < m.y(MARGIN)
                    || y >= m.y(m.rows - MARGIN)
                {
                    assert_eq!(pixels[y * stride + x], 0xdeadbeef);
                }
            }
        }
        assert_eq!(&pixels[stride * h..], &[0xdeadbeef; 4]);
        assert_eq!(Metrics::viewer(320, 356).line_pitch, 10);
    }
}
