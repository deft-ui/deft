use skia_safe::{Font, FontMetrics};
use crate::string::StringUtils;

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
    if x_pos.len() <= 1 || x_pos[1] > available_width {
        return 0;
    }
    let x_offset = x_pos[0];
    let mut start = 0;
    let mut end = x_pos.len() - 1;
    while x_pos[end] - x_offset > available_width && end - start > 1 {
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
