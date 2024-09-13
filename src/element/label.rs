use std::cell::RefCell;
use std::rc::Rc;
use anyhow::{Error};
use quick_js::JsValue;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};
use skia_bindings::SkTextUtils_Align;
use skia_safe::{Canvas, Color, Color4f, Font, FontMgr, FontStyle, Paint, Typeface};
use skia_safe::textlayout::{FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, StrutStyle, TextAlign, TextStyle};
use crate::base::{ElementEvent, PropertyValue, Rect, TextUpdateDetail};
use crate::color::parse_hex_color;
use crate::element::{Element, ElementBackend, ElementRef};
use crate::js_call;
use crate::js::js_value_util::JsValueHelper;
use crate::number::DeNan;
use crate::string::StringUtils;
pub struct AttributeText {
    pub text: String,
    pub font: Font,
}

#[repr(C)]
pub struct Label {
    text: AttributeText,
    align: TextAlign,
    paint: Paint,
    selection_paint: Paint,
    paragraph_props: ParagraphProps,
    // Option<(start, end)>
    selection: Option<(usize, usize)>,
    element: ElementRef,
    line_height: Option<f32>,
}

#[derive(Clone)]
struct ParagraphProps {
    paragraph: Rc<RefCell<ParagraphInfo>>,
}

struct ParagraphInfo {
    paragraph: Paragraph,
    text_wrap: bool,
    paragraph_dirty: bool,
    chars_count: usize,
}

impl ParagraphInfo {

    fn do_layout(&mut self, width: f32) {
        // print_time!("text layout");
        let layout_width = if self.text_wrap {
            width
        } else {
            f32::NAN
        };
        self.paragraph.layout(layout_width);
        self.paragraph_dirty = false;
    }

    fn update_paragraph(&mut self, paragraph: Paragraph, chars_count: usize) {
        self.chars_count = chars_count;
        self.paragraph = paragraph;
        self.paragraph_dirty = true;
    }

    fn get_paragraph(&mut self, width: f32) -> &mut Paragraph {
        if self.paragraph_dirty {
            self.do_layout(width);
        }
        return &mut self.paragraph
    }
}

thread_local! {
    pub static DEFAULT_TYPE_FACE: Typeface = default_typeface();
    pub static FONT_MGR: FontMgr = FontMgr::new();
    pub static FONT_COLLECTION: FontCollection = FontCollection::new();
}

extern "C" fn measure_label(node_ref: NodeRef, width: f32, _mode: MeasureMode, _height: f32, _height_mode: MeasureMode) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(paragraph_props_ptr) = ctx.downcast_ref::<ParagraphProps>() {
            let paragraph = &mut paragraph_props_ptr.paragraph.borrow_mut();
            let p = paragraph.get_paragraph(width);
            let height = p.height();
            let text_width = p.max_intrinsic_width();
            // println!("text len:{}, width:{}, height:{}", paragraph.chars_count, text_width, height);
            return Size {
                width: text_width,
                height,
            };
        }
    }
    return Size {
        width: 0.0,
        height: 0.0,
    };
}



impl Label {

    fn new(element: ElementRef) -> Self {
        let font = DEFAULT_TYPE_FACE.with(|tf| Font::from_typeface(tf, 14.0));
        let text = AttributeText {
            text: "".to_string(),
            font,
        };
        let align = TextAlign::Left;
        let paint = Paint::default();
        let paragraph = Self::build_paragraph(&text, &paint, None, align);
        let paragraph_props = ParagraphProps {
            paragraph: Rc::new(RefCell::new(ParagraphInfo {
                paragraph,
                text_wrap: true,
                paragraph_dirty: true,
                chars_count: 0,
            })),
        };
        let mut selection_paint = Paint::default();
        selection_paint.set_color(parse_hex_color("214283").unwrap());
        Self {
            paragraph_props,
            text,
            align,
            paint: Paint::default(),
            selection_paint,
            selection: None,
            element,
            line_height: None,
        }
    }

    pub fn set_text(&mut self, text: String) {
        if self.text.text != text {
            self.selection = None;
            self.text.text = text.clone();
            self.rebuild_paragraph();
            self.mark_dirty(true);

            let mut event = ElementEvent::new("textupdate", TextUpdateDetail {
                value: text
            }, self.element.clone());
            self.element.emit_event("textupdate", &mut event);
        }
    }

    pub fn set_selection(&mut self, selection: (usize, usize)) {
        let (start, end) = selection;
        self.selection = if end > start  && end <= self.text.text.chars().count() {
            Some(selection)
        } else {
            None
        };
        self.mark_dirty(false);
    }

    pub fn get_selection(&self) -> Option<(usize, usize)> {
        self.selection
    }

    pub fn get_selection_text(&self) -> Option<String> {
        if let Some((start, end)) = self.get_selection() {
            Some(self.text.text.substring(start, end - start).to_string())
        } else {
            None
        }
    }

    pub fn get_text(&self) -> &String {
        &self.text.text
    }

    pub fn set_font_size(&mut self, size: f32) {
        self.text.font.set_size(size);
        self.rebuild_paragraph();
        self.mark_dirty(true);
    }

    pub fn get_font(&self) -> &Font {
        &self.text.font
    }

    pub fn set_align(&mut self, align: TextAlign) {
        self.align = align;
        self.rebuild_paragraph();
        self.mark_dirty(false);
    }

    pub fn get_align(&self) -> TextAlign {
        self.align
    }

    pub fn get_color(&self) -> Color {
        self.paint.color()
    }

    pub fn rebuild_paragraph(&mut self) {
        let paragraph = Self::build_paragraph(&self.text, &self.paint, self.line_height, self.align);
        let mut pi = self.paragraph_props.paragraph.borrow_mut();
        pi.update_paragraph(paragraph, self.text.text.chars().count());
    }

    pub fn get_paint(&self) -> &Paint {
        &self.paint
    }

    pub fn set_text_wrap(&mut self, text_wrap: bool) {
        {
            let mut p = self.paragraph_props.paragraph.borrow_mut();
            p.text_wrap = text_wrap;
        }
        self.mark_dirty(true);
    }

    pub fn get_paragraph_height(&self) -> f32 {
        self.with_paragraph(|p| p.height())
    }

    pub fn get_paragraph_width(&self) -> f32 {
        self.with_paragraph(|p| p.max_intrinsic_width())
    }

    pub fn get_caret_at_offset_coordinate(&self, offset: (f32, f32)) -> usize {
        let (offset_x, offset_y) = offset;
        let (padding_top, _, _, padding_left) = self.element.get_padding();
        let paragraph_position = (offset_x - padding_left, offset_y - padding_top);
        let position = self.with_paragraph(
            |p| p.get_glyph_position_at_coordinate(paragraph_position).position as usize
        );
        usize::min(position, self.text.text.chars().count())
    }

    pub fn get_caret_by_line_and_coordinate_x(&self, line: usize, x_coordinate: f32) -> Option<usize> {
        let y = self.with_paragraph(|p| {
            p.get_line_metrics_at(line).map(|e| e.baseline as f32)
        });
        y.map(|y| self.get_caret_at_offset_coordinate((x_coordinate, y)))
    }

    pub fn get_caret_offset_coordinate(&self, caret: usize) -> ((f32, f32), (f32, f32)) {
        let (padding_top, _, _, padding_left) = self.element.get_padding();
        self.with_paragraph(|p| {
            let caret_height = self.get_font().size();
            let (right, middle) = if let Some(gc) = p.get_glyph_info_at_utf16_offset(caret) {
                //gc.grapheme_layout_bounds
                let x = gc.grapheme_layout_bounds.left + padding_left;
                let middle = (gc.grapheme_layout_bounds.top + gc.grapheme_layout_bounds.bottom) / 2.0 + padding_top;
                (x, middle)
            } else {
                (0.0, 0.0)
            };
            ((right, middle - caret_height / 2.0), (right, middle + caret_height / 2.0))
        })
    }

    pub fn with_paragraph<R, F: FnOnce(&mut Paragraph) -> R>(&self, callback: F) -> R {
        let layout = &self.element.layout;
        let content_width = layout.get_layout_width()
            - layout.get_layout_padding_left().de_nan(0.0)
            - layout.get_layout_padding_right().de_nan(0.0);

        let mut pi = self.paragraph_props.paragraph.borrow_mut();
        let p = pi.get_paragraph(content_width);
        callback(p)
    }

    pub fn get_line_height(&self) -> Option<f32> {
        self.line_height
    }

    pub fn get_computed_line_height(&self) -> f32 {
        match &self.line_height {
            None => self.get_font().size(),
            Some(line_height) => *line_height,
        }
    }

    pub fn build_paragraph(text: &AttributeText, paint: &Paint, line_height: Option<f32>, align: TextAlign) -> Paragraph {
        let mut font_collection = FONT_COLLECTION.with(|f| f.clone());
        FONT_MGR.with(|fm| {
            font_collection.set_default_font_manager(Some(fm.clone()), None);
        });
        let mut paragraph_style = ParagraphStyle::new();
        paragraph_style.set_text_align(align);

        if let Some(line_height) = line_height {
            let mut strut_style = StrutStyle::default();
            strut_style.set_font_families(&["Roboto"]);
            strut_style.set_strut_enabled(true);
            strut_style.set_font_size(line_height);
            strut_style.set_force_strut_height(true);
            paragraph_style.set_strut_style(strut_style);
        }

        let mut pb = ParagraphBuilder::new(&paragraph_style, font_collection);
        let mut text_style = TextStyle::new();
        text_style.set_foreground_paint(paint);
        text_style.set_font_size(text.font.size());

        pb.push_style(&text_style);
        pb.add_text(&text.text);
        // zero-width space for caret
        pb.add_text("\u{200B}");
        pb.build()
    }

    fn mark_dirty(&mut self, layout_dirty: bool) {
        self.element.mark_dirty(layout_dirty);
    }

}


fn default_typeface() -> Typeface {
    let font_mgr = FontMgr::new();
    font_mgr.legacy_make_typeface(None, FontStyle::default()).unwrap()
}

impl ElementBackend for Label {
    fn create(mut ele: ElementRef) -> Self {
        let mut label = Self::new(ele.clone());
        ele.layout.set_context(Some(Context::new(label.paragraph_props.clone())));
        ele.layout.set_measure_func(Some(measure_label));
        label
    }

    fn get_name(&self) -> &str {
        "Label"
    }

    fn handle_style_changed(&mut self, key: &str) {
        if key == "color" {
            let color = self.element.layout.computed_style.color;
            self.paint.set_color(color);
            self.rebuild_paragraph();
            self.mark_dirty(false);
        }
    }

    fn draw(&self, canvas: &Canvas) {
        let selection = self.selection;
        self.with_paragraph(|p| {
            if let Some((begin, end)) = selection {
                for offset in begin..end {
                    if let Some(g) = p.get_glyph_info_at_utf16_offset(offset) {
                        canvas.draw_rect(&g.grapheme_layout_bounds, &self.selection_paint);
                    }
                }
            }
            p.paint(canvas, (0.0, 0.0));
        });
    }

    fn set_property(&mut self, p: &str, v: JsValue) {
        js_call!("text", String, self, set_text, p, v);
        js_call!("fontsize", f32, self, set_font_size, p, v);
        js_call!("align", TextAlign, self, set_align, p, v);
    }

    fn get_property(&mut self, property_name: &str) -> Result<Option<JsValue>, Error> {
        match property_name {
            "text" => Ok(Some(JsValue::String(self.get_text().to_string()))),
            _ => {
                Ok(None)
            }
        }
    }

    fn handle_origin_bounds_change(&mut self, _bounds: &Rect) {
        let mut pi = self.paragraph_props.paragraph.borrow_mut();
        pi.paragraph_dirty = true;
    }

}

pub fn parse_align(align: &str) -> TextAlign {
    match align {
        "left" => TextAlign::Left,
        "right" => TextAlign::Right,
        "center" => TextAlign::Center,
        _ => TextAlign::Left,
    }
}