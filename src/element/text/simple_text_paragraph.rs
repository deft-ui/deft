use skia_safe::{scalar, Canvas, Font, FontMetrics, GlyphId, Paint, Point, Rect};
use skia_safe::canvas::GlyphPositions;
use skia_safe::textlayout::{LineMetrics, PositionWithAffinity, TextStyle};
use skia_safe::textlayout::paragraph::{GlyphInfo, VisitorInfo};
use crate::base;
use crate::canvas_util::CanvasHelper;
use crate::element::paragraph::TextUnit;
use crate::element::text::text_paragraph::TextParams;
use crate::number::DeNan;
use crate::string::StringUtils;
use crate::text::calculate_line_char_count;

#[derive(Clone)]
struct LineUnit {
    block: TextBlock,
    x: f32,
    char_offset: usize,
}

struct BoundsWithOffset {
    pub x: f32,
    pub bounds: Rect,
}

impl BoundsWithOffset {
    pub fn new(x: f32, bounds: Rect) -> Self {
        Self { x, bounds }
    }
    pub fn bounds_with_offset(&self) -> Rect {
        self.bounds.with_offset((self.x, 0.0))
    }
}

impl LineUnit {
    fn get_inner_layout_bounds(&self, compact: bool) -> Vec<BoundsWithOffset> {
        let glyph_ids = self.block.font.str_to_glyphs_vec(self.block.text.as_str());
        let mut bounds = Vec::with_capacity(glyph_ids.len());
        let mut widths = Vec::with_capacity(glyph_ids.len());
        let mut result = Vec::with_capacity(glyph_ids.len());
        unsafe {
            bounds.set_len(glyph_ids.len());
            widths.set_len(glyph_ids.len());
        }
        get_fixed_widths_bounds(&self.block.font, &glyph_ids, &mut widths, &mut bounds, Some(&self.block.style.foreground()));
        let mut x = 0.0;
        let (_, metrics) = self.block.font.metrics();
        for i in 0..glyph_ids.len() {
            if !compact {
                bounds[i].left = 0.0;
                bounds[i].right = widths[i];
                bounds[i].bottom = metrics.descent;
                bounds[i].top = metrics.ascent;
            }
            result.push(BoundsWithOffset::new(x, bounds[i]));
            x += widths[i];
        }
        result
    }

    fn paint(&self, canvas: &Canvas, origin: Point, range: Option<(usize, usize)>, paint: Option<&Paint>) {
        let font = &self.block.font;
        let foreground = self.block.style.foreground();
        let paint = paint.unwrap_or(&foreground);
        let mut glyphs = font.str_to_glyphs_vec(self.block.text.as_str());
        let mut layout_bounds = self.get_inner_layout_bounds(true)
            .iter().map(|b| Point::new(b.x, 0.0)).collect::<Vec<_>>();
        let mut offset = 0;
        let (mut range_start, mut range_end) = range.unwrap_or((0, glyphs.len()));
        for i in 0..glyphs.len() {
            if glyphs[i] != 0 {
                glyphs[offset] = glyphs[i];
                layout_bounds[offset] = layout_bounds[i];
                offset += 1;
            } else {
                if i < range_start {
                    range_start -= 1;
                }
                if i < range_end {
                    range_end -= 1;
                }
            }
        }
        canvas.draw_glyphs_at(&glyphs[range_start..range_end], GlyphPositions::Points(&layout_bounds[range_start..range_end]), origin , &font, &paint);
    }

}

#[derive(Clone)]
struct TextLine {
    units: Vec<LineUnit>,
    line_number: usize,
    y: f32,
    baseline: f32,
    height: f32,
    char_offset: usize,
}

impl TextLine {
    pub fn new(line_number: usize, char_offset: usize) -> Self {
        Self {
            units: Vec::new(),
            line_number,
            baseline: 0.0,
            height: 0.0,
            y: 0.0,
            char_offset,
        }
    }
}

#[derive(Clone)]
pub struct SimpleTextParagraph {
    text: String,
    line_height: Option<f32>,
    height: f32,
    max_intrinsic_width: f32,
    text_blocks: Vec<TextBlock>,
    lines: Vec<TextLine>,
}


#[derive(Clone, Debug)]
pub struct TextBlock {
    pub text: String,
    pub style: TextStyle,
    pub font: Font,
}

impl SimpleTextParagraph {
    pub fn new(text_blocks: Vec<TextBlock>) -> Self {
        let mut font_line_height = 0.0;
        let mut text = String::new();
        for text_block in &text_blocks {
            text.push_str(text_block.text.as_str());
        }

        Self {
            text,
            max_intrinsic_width: 0.0,
            height: 0.0,
            line_height: None,
            text_blocks,
            lines: Vec::new(),
        }
    }

    pub fn layout(&mut self, mut available_width: f32) {
        available_width = available_width.de_nan(f32::INFINITY);

        let mut left = 0.0;
        let mut top = 0.0;
        let mut char_offset = 0;
        let mut max_intrinsic_width = 0.0;

        let line_height = self.line_height;
        let mut lines = Vec::new();
        let mut current_line = TextLine::new(0, 0);

        for tb in &self.text_blocks {
            let glyphs = tb.font.str_to_glyphs_vec(&tb.text);
            let char_count = glyphs.len();
            if char_count == 0 {
                continue;
            }
            let mut widths = Vec::with_capacity(char_count);
            let mut x_pos = Vec::with_capacity(char_count + 1);
            let mut bounds = Vec::with_capacity(char_count);
            unsafe {
                widths.set_len(char_count);
                bounds.set_len(char_count);
                x_pos.set_len(char_count + 1);
            }
            get_fixed_widths_bounds(&tb.font, &glyphs, &mut widths, &mut bounds, Some(&tb.style.foreground()));
            x_pos[0] = 0.0;
            for i in 0..char_count {
                if glyphs[i] == 0 {
                    widths[i] = 0.0;
                }
                x_pos[i + 1] = x_pos[i] + widths[i];
            }
            let (_, font_metrics) = tb.font.metrics();

            let mut consumed_char_count = 0;
            while consumed_char_count < char_count {
                let mut cc = calculate_line_char_count(&x_pos[consumed_char_count..], available_width - left);
                if cc == 0 && left == 0.0 {
                    cc = 1;
                }
                if cc == 0 {
                    let next_line_number = current_line.line_number + 1;
                    left = 0.0;
                    top += current_line.height;
                    lines.push(current_line);
                    current_line = TextLine::new(next_line_number, char_offset);
                    current_line.y = top;
                    continue;
                }
                current_line.units.push(LineUnit {
                    block: TextBlock {
                        text: tb.text.substring(consumed_char_count, cc).to_string(),
                        style: tb.style.clone(),
                        font: tb.font.clone(),
                    },
                    x: left,
                    char_offset,
                });
                char_offset += cc;
                left += x_pos[consumed_char_count + cc - 1] - x_pos[consumed_char_count] + widths[consumed_char_count + cc - 1];
                consumed_char_count += cc;
                current_line.baseline = f32::max(current_line.baseline, -font_metrics.ascent + font_metrics.leading);
                current_line.height = f32::max(current_line.height, -font_metrics.ascent + font_metrics.descent + font_metrics.leading);
                max_intrinsic_width = f32::max(max_intrinsic_width, left);
            }
        }
        self.max_intrinsic_width = max_intrinsic_width;
        self.height = current_line.y + current_line.height;
        lines.push(current_line);
        self.lines = lines;
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn max_intrinsic_width(&self) -> f32 {
        self.max_intrinsic_width
    }

    pub fn get_char_bounds(&mut self, char_offset: usize) -> Option<Rect> {
        let (ln, unit) = self.get_unit_at_char_offset(char_offset)?;
        let char_offset = char_offset - unit.char_offset;
        let unit_origin = (unit.x, ln.y + ln.baseline);
        let bounds = unit.get_inner_layout_bounds(false);
        return Some(bounds[char_offset].bounds_with_offset().with_offset(unit_origin));
    }

    fn get_unit_at_char_offset(&self, char_offset: usize) -> Option<(&TextLine, &LineUnit)> {
        for ln in self.lines.iter().rev() {
            if ln.char_offset > char_offset {
                continue;
            }
            for unit in ln.units.iter().rev() {
                if unit.char_offset > char_offset {
                    continue;
                }
                return Some((ln, unit));
            }
            return None;
        }
        None
    }

    pub fn get_char_offset_at_coordinate(&self, coord: (f32, f32)) -> usize {
        let (x, y) = coord;
        if y < 0.0 {
            return 0;
        }
        for ln in self.lines.iter().rev() {
            if ln.y > coord.1 {
                continue;
            }
            if x < 0.0 {
                return ln.char_offset;
            }
            for unit in ln.units.iter().rev() {
                if unit.x > coord.0 {
                    continue;
                }
                let inner_x = x - unit.x;
                let inner_bounds = unit.get_inner_layout_bounds(false);
                let inner_bounds_len = inner_bounds.len();
                for b in 0..inner_bounds_len {
                    if inner_bounds[b].bounds_with_offset().right > inner_x {
                        return unit.char_offset + b;
                    }
                }
                return unit.char_offset;
            }
            return ln.char_offset;
        }
        return 0;
    }

    pub fn get_soft_line_height(&self, char_offset: usize) -> f32 {
        if let Some((ln, unit)) = self.get_unit_at_char_offset(char_offset) {
            ln.height
        } else {
            0.0
        }
    }

    pub fn paint(&self, canvas: &Canvas, p: impl Into<Point>) {
        canvas.save();
        let p = p.into();
        canvas.translate(p);
        for ln in &self.lines {
            let y = ln.y + ln.baseline;
            for unit in &ln.units {
                let tb = &unit.block;
                let x = unit.x;
                unit.paint(&canvas, Point::new(x, y), None, None);
            }
        }
        canvas.restore();
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn get_line_number_at_utf16_offset(&self, offset: usize) -> Option<usize> {
        let (ln, unit) = self.get_unit_at_char_offset(offset)?;
        Some(ln.line_number)
    }

    pub fn get_line_height_at(&self, line_number: usize) -> Option<f32> {
        let ln = self.lines.get(line_number)?;
        Some(ln.height)
    }

    pub fn paint_chars(&self,canvas: &Canvas, mut start: usize, end: usize, paint: Option<&Paint>) {
        while start < end {
            if let Some((ln, unit)) = self.get_unit_at_char_offset(start) {
                let unit_start = start - unit.char_offset;
                let paint_char_count = usize::min(unit.block.text.chars_count() - unit_start, end - start);
                unit.paint(canvas, Point::new(unit.x, ln.y + ln.baseline), Some((unit_start, unit_start + paint_char_count)), paint);
                start += paint_char_count;
            } else {
                return;
            }
        }
    }

}

pub fn get_fixed_widths_bounds(
    font: &Font,
    glyphs: &[GlyphId],
    mut widths: &mut [scalar],
    mut bounds: &mut [Rect],
    paint: Option<&Paint>,
) {
    font.get_widths_bounds(glyphs, Some(widths), Some(bounds), paint);
    for i in 0..glyphs.len() {
        if glyphs[i] == 0 {
            widths[i] = 0.0;
            bounds[i] = Rect::new(0.0, 0.0, 0.0, 0.0);
        }
    }
}