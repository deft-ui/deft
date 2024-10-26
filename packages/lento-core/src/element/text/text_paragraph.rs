use std::cell::RefCell;
use std::rc::Rc;
use skia_safe::{Font, Paint};
use skia_safe::textlayout::TextAlign;
use crate::element::text::{AtomOffset, ColOffset};
use crate::element::text::simple_text_paragraph::SimpleTextParagraph;
use crate::element::text::skia_text_paragraph::SkiaTextParagraph;
use crate::string::StringUtils;

#[derive(Clone)]
pub struct ParagraphRef {
    pub data: Rc<RefCell<ParagraphData>>,
}

pub struct ParagraphData {
    pub lines: Vec<Line>,
    pub text_wrap: bool,
}

pub struct Line {
    /// Atom count; \r\n is treated as one atom
    pub atom_count: AtomOffset,
    pub paragraph: SkiaTextParagraph,
    // pub paragraph: SimpleTextParagraph,
    pub paragraph_dirty: bool,
}

#[derive(Clone)]
pub struct TextParams {
    pub font: Font,
    pub paint: Paint,
    pub line_height: Option<f32>,
    pub align: TextAlign,
}

impl ParagraphData {
    pub fn update_line(&mut self, line: Vec<Line>) {
        self.lines = line;
    }

    pub fn get_line(&mut self, width: f32) -> &mut Vec<Line> {
        self.lines.iter_mut().for_each(|it| {
            if it.paragraph_dirty {
                let layout_width = if self.text_wrap {
                    width
                } else {
                    f32::NAN
                };
                it.paragraph.layout(layout_width);
                it.paragraph_dirty = false;
            }
        });

        return &mut self.lines;
    }
}

impl Line {
    pub fn get_caret_by_coord(&self, coord: (f32, f32)) -> usize {
        let col = self.paragraph.get_char_offset_at_coordinate(coord);
        usize::min(col, self.atom_count - 1)
    }
    pub fn get_column_by_atom_offset(&self, atom_offset: AtomOffset) -> usize {
        AtomOffset::min(atom_offset, self.atom_count - 1)
    }
    pub fn subtext(&self, start: ColOffset, end: ColOffset) -> &str {
        let text = self.paragraph.get_text();
        text.substring(start, end - start)
    }

    pub fn get_text(&self) -> &str {
        self.paragraph.get_text()
    }

    pub fn get_soft_line_height(&self, char_offset: usize) -> f32 {
        self.paragraph.get_soft_line_height(char_offset)
    }
}