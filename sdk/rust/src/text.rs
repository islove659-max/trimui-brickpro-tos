//! Render chữ (có dấu tiếng Việt) bằng ab_glyph + font Be Vietnam Pro (OFL) nhúng trong binary.
//! Có cache glyph theo (ký tự, cỡ chữ). Văn bản phải ở dạng NFC (dấu đã gộp sẵn): chuỗi lấy từ web/tên tệp
//! dạng NFD cần chuẩn hoá trước (SDK chưa kèm bộ chuẩn hoá để tránh phụ thuộc thêm crate).
use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use std::collections::HashMap;

static FONT_BYTES: &[u8] = include_bytes!("../assets/BeVietnamPro-Regular.ttf");

struct Glyph { w: usize, h: usize, left: i32, top: i32, cov: Vec<u8> }

pub struct Text {
    font: FontRef<'static>,
    cache: HashMap<(char, u32), Glyph>,
    off: [u32; 3], // offset bit kênh R,G,B của framebuffer
}

impl Text {
    pub fn new(channel_offsets: [u32; 3]) -> Result<Text, String> {
        let font = FontRef::try_from_slice(FONT_BYTES).map_err(|e| format!("font lỗi: {:?}", e))?;
        Ok(Text { font, cache: HashMap::new(), off: channel_offsets })
    }

    fn glyph(&mut self, c: char, size: u32) -> &Glyph {
        let font = &self.font;
        self.cache.entry((c, size)).or_insert_with(|| {
            let scale = PxScale::from(size as f32);
            let id = font.glyph_id(c);
            let g = id.with_scale_and_position(scale, point(0.0, 0.0));
            match font.outline_glyph(g) {
                Some(og) => {
                    let b = og.px_bounds();
                    let (w, h) = (b.width().ceil() as usize + 1, b.height().ceil() as usize + 1);
                    let mut cov = vec![0u8; w * h];
                    og.draw(|x, y, v| {
                        let (x, y) = (x as usize, y as usize);
                        if x < w && y < h { cov[y * w + x] = (v * 255.0) as u8; }
                    });
                    Glyph { w, h, left: b.min.x.floor() as i32, top: b.min.y.floor() as i32, cov }
                }
                None => Glyph { w: 0, h: 0, left: 0, top: 0, cov: Vec::new() }, // khoảng trắng
            }
        })
    }

    /// Độ rộng (điểm ảnh) của chuỗi ở cỡ `size`.
    pub fn measure(&self, text: &str, size: u32) -> f32 {
        let scaled = self.font.as_scaled(PxScale::from(size as f32));
        let mut x = 0.0; let mut prev = None;
        for c in text.chars() {
            let id = scaled.glyph_id(c);
            if let Some(p) = prev { x += scaled.kern(p, id); }
            x += scaled.h_advance(id); prev = Some(id);
        }
        x
    }

    /// Vẽ chuỗi, góc trên-trái (x, y). Pha trộn alpha lên `buf` (stride tính bằng pixel), cắt theo biên.
    pub fn draw(&mut self, buf: &mut [u32], stride: usize, width: usize, height: usize,
                x: i32, y: i32, text: &str, size: u32, color: (u8, u8, u8)) {
        let scale = PxScale::from(size as f32);
        let ascent = self.font.as_scaled(scale).ascent();
        let baseline = y as f32 + ascent;
        let mut pen = x as f32; let mut prev = None;
        let off = self.off;
        for c in text.chars() {
            let (id, adv, kern) = {
                let scaled = self.font.as_scaled(scale);
                let id = scaled.glyph_id(c);
                (id, scaled.h_advance(id), prev.map_or(0.0, |p| scaled.kern(p, id)))
            };
            pen += kern; prev = Some(id);
            let (gx, gy) = (pen.round() as i32, baseline.round() as i32);
            let g = self.glyph(c, size);
            for row in 0..g.h {
                let py = gy + g.top + row as i32;
                if py < 0 || py as usize >= height { continue; }
                for col in 0..g.w {
                    let a = g.cov[row * g.w + col] as u32;
                    if a == 0 { continue; }
                    let px = gx + g.left + col as i32;
                    if px < 0 || px as usize >= width { continue; }
                    let dst = &mut buf[py as usize * stride + px as usize];
                    let mix = |ch: u32, fg: u8| -> u32 { (((*dst >> ch) & 0xFF) * (255 - a) + fg as u32 * a) / 255 };
                    *dst = (mix(off[0], color.0) << off[0]) | (mix(off[1], color.1) << off[1]) | (mix(off[2], color.2) << off[2]);
                }
            }
            pen += adv;
        }
    }
}
