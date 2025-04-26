use bitflags::bitflags;
use skia_safe::{Color, Font, FontMetrics, FontStyle, Paint};
use crate::string::StringUtils;

#[repr(i32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum TextAlign {
    Left = 0,
    Right = 1,
    Center = 2,
    Justify = 3,
    Start = 4,
    End = 5,
}

#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    font_size: f32,
    font_families: Vec<String>,
    foreground_paint: Paint,
    background_paint: Paint,
    font_style: FontStyle,
    decoration_type: TextDecoration,
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_color(&mut self, color: Color) {
        self.foreground_paint.set_color(color);
    }
    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
    }
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    pub fn set_font_families(&mut self, families: &Vec<String>) {
        self.font_families = families.clone();
    }

    pub fn font_families(&self) -> Vec<&str> {
        self.font_families.iter().map(|it| it.as_str()).collect()
    }

    pub fn set_foreground_paint(&mut self, paint: &Paint) {
        self.foreground_paint = paint.clone();
    }
    pub fn foreground(&self) -> Paint {
        self.foreground_paint.clone()
    }

    pub fn set_background_paint(&mut self, paint: &Paint) {
        self.background_paint = paint.clone();
    }

    pub fn background(&self) -> Paint {
        self.background_paint.clone()
    }

    pub fn set_font_style(&mut self, font_style: FontStyle) {
        self.font_style = font_style;
    }

    pub fn font_style(&self) -> &FontStyle {
        &self.font_style
    }

    pub fn set_decoration_type(&mut self, decoration_type: TextDecoration) {
        self.decoration_type = decoration_type;
    }

}

bitflags! {
    /// Multiple decorations can be applied at once. Ex: Underline and overline is
    /// (0x1 | 0x2)
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TextDecoration: u32 {
        const NO_DECORATION = 0;
        const UNDERLINE = 1;
        const OVERLINE = 2;
        const LINE_THROUGH = 4;
    }
}

pub const ALL_TEXT_DECORATIONS: TextDecoration = TextDecoration::ALL;

impl Default for TextDecoration {
    fn default() -> Self {
        TextDecoration::NO_DECORATION
    }
}

impl TextDecoration {
    pub const ALL: TextDecoration = TextDecoration::all();
}

fn predicate_len(width: f32, fm: &FontMetrics) -> usize {
    (width / fm.avg_char_width).floor() as usize
}

fn calculate_len(str: &str, width: f32, char_count: usize, available_width: f32, fm: &FontMetrics, font: &Font) -> usize {
    let str_chars = str.chars().count();
    if str_chars <= 1 {
        return str_chars;
    }
    if width == available_width {
        return char_count;
    }
    if width < available_width {
        let remaining_char_count = str.chars().count() - char_count;
        let add_char_count = usize::min(predicate_len(available_width - width, fm), remaining_char_count);
        if add_char_count == 0 {
            return char_count;
        }
        let (add_width, _) = font.measure_str(str.substring(char_count, add_char_count), None);
        let new_width = width + add_width;
        if new_width > width && add_char_count == 1 {
            char_count
        } else {
            calculate_len(str, new_width, char_count + add_char_count, available_width, fm, font)
        }
    } else {
        let minus_char_count = usize::min(char_count, predicate_len(width - available_width, fm));
        if minus_char_count == 0 {
            return char_count
        }
        let (minus_width, _) = font.measure_str(str.substring(char_count - minus_char_count, minus_char_count), None);
        let new_width = width - minus_width;
        if new_width < width && minus_char_count == 1 {
            char_count - minus_char_count
        } else {
            calculate_len(str, new_width, char_count - minus_char_count, available_width, fm, font)
        }
    }
}

pub fn calculate_line_char_count(x_pos: &[f32], available_width: f32) -> usize {
    if x_pos.len() <= 1 || x_pos[1] - x_pos[0] > available_width {
        return 0;
    }
    let x_offset = x_pos[0];
    let mut start = 0;
    let mut end = x_pos.len() - 1;
    while x_pos[end] - x_offset > available_width {
        if end - start == 1 {
            return start;
        }
        let mid = (start + end) / 2;
        if x_pos[mid] - x_offset > available_width {
            end = mid;
        } else {
            start = mid;
        }
    }
    end
}

pub fn break_lines(font: &Font, mut str: &str, available_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let (_, metrics) = font.metrics();
    let glyphs_ids = font.str_to_glyphs_vec(str);
    let mut x_pos_vec = Vec::with_capacity(glyphs_ids.len());
    unsafe {
        x_pos_vec.set_len(glyphs_ids.len());
    }
    font.get_x_pos(&glyphs_ids, &mut x_pos_vec, None);
    let mut x_pos = &x_pos_vec[0..];
    while x_pos.len() > 0 {
        let ln_len = calculate_line_char_count(&x_pos_vec, available_width);
        let ln_str = str.substring(0, ln_len);
        lines.push(ln_str.to_string());
        if ln_str.len() == str.len() {
            break;
        }
        x_pos = &x_pos[ln_len..];
        str = &str[ln_str.len()..];
    }
    lines
}
