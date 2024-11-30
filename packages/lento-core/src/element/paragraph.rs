use crate as lento;
use crate::color::parse_hex_color;
use crate::element::text::text_paragraph::ParagraphRef;
use crate::element::text::{FONT_COLLECTION, FONT_MGR};
use crate::element::{Element, ElementBackend};
use crate::js::JsError;
use crate::number::DeNan;
use crate::string::StringUtils;
use crate::style::{parse_color, parse_color_str, parse_optional_color_str};
use crate::{js_deserialize, js_serialize};
use lento_macros::{element_backend, js_methods, mrc_object};
use rodio::cpal::available_hosts;
use serde::{Deserialize, Serialize};
use skia_bindings::SkFontStyle_Weight;
use skia_safe::font_style::{Slant, Weight, Width};
use skia_safe::textlayout::{
    Decoration, FontFamilies, Paragraph as SkParagraph, ParagraphBuilder, ParagraphStyle,
    PlaceholderStyle, StrutStyle, TextAlign, TextDecoration, TextDirection, TextStyle,
    TypefaceFontProvider,
};
use skia_safe::{Canvas, Color, Font, FontMgr, FontStyle, Paint, Point, Rect};
use std::str::FromStr;
use yoga::{Context, MeasureMode, Node, NodeRef, Size};

const DEFAULT_FONT_NAME: &str = "monospace";

#[derive(Clone)]
pub struct ParagraphParams {
    pub line_height: Option<f32>,
    pub align: TextAlign,
    pub color: Color,
    pub font_size: f32,
    pub font_families: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ParagraphUnit {
    Text(TextUnit),
}

js_serialize!(ParagraphUnit);
js_deserialize!(ParagraphUnit);

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TextUnit {
    pub text: String,
    pub font_families: Option<Vec<String>>,
    pub font_size: Option<f32>,
    pub color: Option<String>,
    pub text_decoration_line: Option<String>,
    pub weight: Option<String>,
    pub background_color: Option<String>,
}

js_serialize!(TextUnit);
js_deserialize!(TextUnit);

#[element_backend]
pub struct Paragraph {
    element: Element,
    params: ParagraphParams,
    lines: Vec<Line>,
}

struct Line {
    units: Vec<ParagraphUnit>,
    sk_paragraph: SkParagraph,
    layout_calculated: bool,
}

impl Line {
    pub fn new(units: Vec<ParagraphUnit>, paragraph_params: &ParagraphParams) -> Self {
        let sk_paragraph = Paragraph::build_paragraph(paragraph_params, &units);
        Self {
            layout_calculated: false,
            units,
            sk_paragraph,
        }
    }

    fn rebuild_paragraph(&mut self, paragraph_params: &ParagraphParams) {
        self.sk_paragraph = Paragraph::build_paragraph(paragraph_params, &self.units);
        self.layout_calculated = false;
    }

    fn force_layout(&mut self, available_width: f32) {
        self.layout_calculated = true;
        self.sk_paragraph.layout(available_width);
    }

    fn layout(&mut self, available_width: Option<f32>, element_width: f32) {
        if let Some(w) = available_width {
            self.force_layout(w);
        } else if (!self.layout_calculated) {
            self.force_layout(element_width);
        }
    }

}

extern "C" fn measure_paragraph(
    node_ref: NodeRef,
    width: f32,
    width_mode: MeasureMode,
    _height: f32,
    height_mode: MeasureMode,
) -> Size {
    if let Some(ctx) = Node::get_context(&node_ref) {
        if let Some(paragraph) = ctx.downcast_ref::<ParagraphWeak>() {
            if let Ok(mut p) = paragraph.upgrade() {
                p.layout(Some(width));
                return Size {
                    width: p.max_intrinsic_width(),
                    height: p.height(),
                };
            }
        }
    }
    Size {
        width: 0.0,
        height: 0.0,
    }
}

#[js_methods]
impl Paragraph {
    #[js_func]
    pub fn new_element() -> Element {
        Element::create(Paragraph::create)
    }

    #[js_func]
    pub fn add_line(&mut self, units: Vec<ParagraphUnit>) {
        let line = Line::new(units, &self.params);
        self.lines.push(line);
        self.mark_dirty();
    }

    #[js_func]
    pub fn insert_line(&mut self, index: usize, units: Vec<ParagraphUnit>) {
        let line = Line::new(units, &self.params);
        self.lines.insert(index, line);
        self.mark_dirty();
    }

    #[js_func]
    pub fn delete_line(&mut self, line: usize) {
        self.lines.remove(line);
        self.mark_dirty();
    }

    #[js_func]
    pub fn update_line(&mut self, index: usize, units: Vec<ParagraphUnit>) {
        self.lines[index] = Line::new(units, &self.params);
        self.mark_dirty();
    }

    #[js_func]
    pub fn clear(&mut self) {
        self.lines.clear();
        self.mark_dirty();
    }

    fn layout(&mut self, available_width: Option<f32>) {
        let layout_width = self.element.layout.get_layout_width();
        for ln in &mut self.lines {
            ln.layout(available_width, layout_width);
        }
    }

    pub fn height(&self) -> f32 {
        let mut height = 0.0;
        for ln in &self.lines {
            height += ln.sk_paragraph.height().de_nan(0.0);
        }
        height
    }

    pub fn max_intrinsic_width(&self) -> f32 {
        let mut max_width = 0.0;
        for ln in &self.lines {
            max_width = f32::max(max_width, ln.sk_paragraph.max_intrinsic_width());
        }
        max_width
    }

    fn rebuild_paragraph(&mut self) {
        let params = self.params.clone();
        for ln in &mut self.lines {
            ln.rebuild_paragraph(&params);
        }
    }

    fn mark_dirty(&mut self) {
        self.element.mark_dirty(true);
    }

    pub fn build_paragraph(
        paragraph_params: &ParagraphParams,
        units: &Vec<ParagraphUnit>,
    ) -> SkParagraph {
        // let mut text = text.trim_line_endings().to_string();
        // text.push_str(ZERO_WIDTH_WHITESPACE);
        let mut font_collection = FONT_COLLECTION.with(|f| f.clone());
        FONT_MGR.with(|fm| {
            font_collection.set_default_font_manager(Some(fm.clone()), None);
        });
        let mut paragraph_style = ParagraphStyle::new();
        paragraph_style.set_text_align(paragraph_params.align);

        if let Some(line_height) = paragraph_params.line_height {
            let mut strut_style = StrutStyle::default();
            strut_style.set_font_families(&[DEFAULT_FONT_NAME]);
            strut_style.set_strut_enabled(true);
            strut_style.set_font_size(line_height);
            strut_style.set_force_strut_height(true);
            paragraph_style.set_strut_style(strut_style);
        }

        let mut pb = ParagraphBuilder::new(&paragraph_style, font_collection);
        let p_color = paragraph_params.color;
        for u in units {
            match u {
                ParagraphUnit::Text(unit) => {
                    let mut text_style = TextStyle::new();
                    let font_families = unit
                        .font_families
                        .as_ref()
                        .unwrap_or(&paragraph_params.font_families);
                    let font_families = if font_families.is_empty() {
                        &paragraph_params.font_families
                    } else {
                        &font_families
                    };
                    let font_size = unit.font_size.unwrap_or(paragraph_params.font_size);
                    text_style.set_font_families(&font_families);
                    text_style.set_font_size(font_size);

                    let weight =
                        parse_optional_weight(unit.weight.as_ref()).unwrap_or(Weight::NORMAL);
                    let font_style = FontStyle::new(weight, Width::NORMAL, Slant::Upright);
                    text_style.set_font_style(font_style);

                    let decoration =
                        parse_optional_text_decoration(unit.text_decoration_line.as_ref());
                    text_style.set_decoration_type(decoration);

                    let color = parse_optional_color_str(unit.color.as_ref()).unwrap_or(p_color);
                    let mut paint = Paint::default();
                    paint.set_color(color);
                    text_style.set_foreground_paint(&paint);

                    if let Some(bg) = parse_optional_color_str(unit.background_color.as_ref()) {
                        let mut bg_paint = Paint::default();
                        bg_paint.set_color(bg);
                        text_style.set_background_paint(&bg_paint);
                    }

                    pb.push_style(&text_style);
                    pb.add_text(&unit.text);
                }
            }
        }

        pb.build()
    }
}

impl ElementBackend for Paragraph {
    fn create(mut element: Element) -> Self
    where
        Self: Sized,
    {
        let params = ParagraphParams {
            line_height: None,
            align: TextAlign::Left,
            color: Color::default(),
            font_size: 12.0,
            font_families: vec![DEFAULT_FONT_NAME.to_string()],
        };
        let units = Vec::new();
        let paragraph = Self::build_paragraph(&params, &units);
        let this = ParagraphData {
            lines: Vec::new(),
            element: element.clone(),
            params,
        }
        .to_ref();
        element
            .layout
            .set_context(Some(Context::new(this.as_weak())));
        element.layout.set_measure_func(Some(measure_paragraph));
        this
    }

    fn get_name(&self) -> &str {
        "Paragraph"
    }

    fn handle_style_changed(&mut self, key: &str) {
        match key.to_lowercase().as_str() {
            "color" => {
                self.params.color = self.element.layout.computed_style.color;
                self.rebuild_paragraph();
            }
            "fontsize" => {
                self.params.font_size = self.element.layout.font_size;
                self.rebuild_paragraph();
            }
            _ => {}
        }
    }

    fn draw(&self, canvas: &Canvas) {
        let mut p = self.clone();
        p.layout(None);
        let mut y = 0.0;
        for ln in &p.lines {
            ln.sk_paragraph.paint(canvas, (0.0, y));
            y += ln.sk_paragraph.height();
        }
    }
}

fn parse_optional_weight(value: Option<&String>) -> Option<Weight> {
    if let Some(v) = value {
        parse_weight(v)
    } else {
        None
    }
}
fn parse_weight(value: &str) -> Option<Weight> {
    let w = match value.to_lowercase().as_str() {
        "invisible" => Weight::INVISIBLE,
        "thin" => Weight::THIN,
        "extra-light" => Weight::EXTRA_LIGHT,
        "light" => Weight::LIGHT,
        "normal" => Weight::NORMAL,
        "medium" => Weight::MEDIUM,
        "semi-bold" => Weight::SEMI_BOLD,
        "bold" => Weight::BOLD,
        "extra-bold" => Weight::EXTRA_BOLD,
        "black" => Weight::BLACK,
        "extra-black" => Weight::EXTRA_BLACK,
        _ => return i32::from_str(value).ok().map(|w| Weight::from(w)),
    };
    Some(w)
}

fn parse_optional_text_decoration(value: Option<&String>) -> TextDecoration {
    if let Some(v) = value {
        parse_text_decoration(v)
    } else {
        TextDecoration::default()
    }
}

fn parse_text_decoration(value: &str) -> TextDecoration {
    let mut decoration = TextDecoration::default();
    for ty in value.split(" ") {
        let t = match value {
            "none" => TextDecoration::NO_DECORATION,
            "underline" => TextDecoration::UNDERLINE,
            "overline" => TextDecoration::OVERLINE,
            "line-through" => TextDecoration::LINE_THROUGH,
            _ => continue,
        };
        decoration.set(t, true);
    }
    decoration
}
