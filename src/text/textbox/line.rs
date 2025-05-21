use crate::element::text::simple_text_paragraph::SimpleTextParagraph;
use crate::element::text::{intersect_range, ColOffset};
use crate::string::StringUtils;
use crate::text::textbox::{ParagraphParams, TextBox, TextElement};

pub struct Line {
    pub units: Vec<TextElement>,
    pub sk_paragraph: SimpleTextParagraph,
    pub layout_calculated: bool,
}

impl Line {
    pub fn new(units: Vec<TextElement>, paragraph_params: &ParagraphParams) -> Self {
        let sk_paragraph = TextBox::build_paragraph(paragraph_params, &units);
        Self {
            layout_calculated: false,
            units,
            sk_paragraph,
        }
    }

    pub fn atom_count(&self) -> usize {
        let mut count = 0;
        for u in &self.units {
            count += u.atom_count();
        }
        count
    }

    pub fn get_text(&self) -> String {
        let mut result = String::new();
        for u in &self.units {
            result.push_str(u.text());
        }
        result
    }

    pub fn subtext(&self, start: ColOffset, end: ColOffset) -> String {
        let mut result = String::new();
        let mut iter = self.units.iter();
        let mut processed_atom_count = 0;
        loop {
            let u = match iter.next() {
                Some(u) => u,
                None => break,
            };
            let unit_atom_count = u.atom_count();
            if let Some(intersect) = intersect_range(
                (start, end),
                (processed_atom_count, unit_atom_count + processed_atom_count),
            ) {
                result.push_str(u.get_text(
                    intersect.0 - processed_atom_count,
                    intersect.1 - processed_atom_count,
                ));
            }
            processed_atom_count += unit_atom_count;
            if processed_atom_count >= end {
                break;
            }
        }
        result.to_string()
    }

    pub fn get_column_by_pixel_coord(&self, coord: (f32, f32)) -> usize {
        let (x, _y) = coord;
        let atom_count = self.atom_count();
        if atom_count == 0 {
            0
        } else if x > self.sk_paragraph.max_intrinsic_width() {
            atom_count
        } else {
            self.sk_paragraph.get_char_offset_at_coordinate(coord)
        }
    }

    pub fn get_utf8_offset(&self, char_offset: usize) -> usize {
        if char_offset == 0 {
            0
        } else {
            self.get_text().substring(0, char_offset).len()
        }
    }

    pub fn rebuild_paragraph(&mut self, paragraph_params: &ParagraphParams) {
        self.sk_paragraph = TextBox::build_paragraph(paragraph_params, &self.units);
        self.layout_calculated = false;
    }

    pub fn force_layout(&mut self, available_width: f32) {
        self.sk_paragraph.layout(available_width);
        self.layout_calculated = true;
    }
}
