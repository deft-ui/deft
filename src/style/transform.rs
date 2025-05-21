use crate::paint::MatrixCalculator;
use crate::style::length::parse_percent;
use crate::style::PropValueParse;
use std::str::FromStr;

impl PropValueParse for StyleTransform {
    fn parse_prop_value(value: &str) -> Option<Self> {
        //TODO support multiple op
        if let Some(op) = StyleTransformOp::parse(value) {
            Some(Self { op_list: vec![op] })
        } else {
            None
        }
    }
    fn to_style_string(&self) -> String {
        self.op_list
            .iter()
            .map(|it| it.to_style_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum TranslateLength {
    Point(f32),
    Percent(f32),
}

impl TranslateLength {
    pub fn adapt_zero(&mut self, other: &mut Self) {
        if self.is_zero() {
            match other {
                TranslateLength::Point(_) => {
                    *self = TranslateLength::Point(0.0);
                }
                TranslateLength::Percent(_) => {
                    *self = TranslateLength::Percent(0.0);
                }
            }
        } else if other.is_zero() {
            other.adapt_zero(self)
        }
    }
    pub fn to_absolute(&self, block_length: f32) -> f32 {
        match self {
            TranslateLength::Point(p) => *p,
            TranslateLength::Percent(p) => *p / 100.0 * block_length,
        }
    }

    pub fn to_style_string(&self) -> String {
        match self {
            TranslateLength::Point(v) => v.to_string(),
            TranslateLength::Percent(p) => {
                format!("{}%", p)
            }
        }
    }

    fn is_zero(&self) -> bool {
        match self {
            TranslateLength::Point(v) => *v == 0.0,
            TranslateLength::Percent(v) => *v == 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TranslateParams(pub TranslateLength, pub TranslateLength);

#[derive(Clone, Debug, PartialEq)]
pub struct ScaleParams(pub f32, pub f32);

#[derive(Clone, Debug, PartialEq)]
pub enum StyleTransformOp {
    Rotate(f32),
    Scale(ScaleParams),
    Translate(TranslateParams),
}

impl StyleTransformOp {
    pub fn parse(str: &str) -> Option<Self> {
        let value = str.trim();
        if !value.ends_with(")") {
            return None;
        }
        let left_p = value.find("(")?;
        let func = &value[0..left_p];
        let param_str = &value[left_p + 1..value.len() - 1];
        //TODO support double params
        match func {
            //"matrix" => parse_matrix(param_str).ok(),
            "translate" => parse_translate_op(param_str),
            "rotate" => parse_rotate_op(param_str),
            "scale" => parse_scale_op(param_str),
            _ => None,
        }
    }
    pub fn to_style_string(&self) -> String {
        match self {
            StyleTransformOp::Rotate(v) => {
                format!("rotate({})", v)
            }
            StyleTransformOp::Scale(v) => {
                format!("scale({}, {})", v.0, v.1)
            }
            StyleTransformOp::Translate(p) => {
                format!(
                    "translate({}, {})",
                    p.0.to_style_string(),
                    p.1.to_style_string()
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleTransform {
    pub op_list: Vec<StyleTransformOp>,
}

impl StyleTransform {
    pub fn empty() -> StyleTransform {
        Self {
            op_list: Vec::new(),
        }
    }

    pub fn preprocess(&self) -> StyleTransform {
        let mut list = Vec::new();
        for op in self.op_list.clone() {
            if let StyleTransformOp::Translate(params) = op {
                let (mut tl, mut tl2) = (params.0, params.1);
                tl.adapt_zero(&mut tl2);
                list.push(StyleTransformOp::Translate(TranslateParams(tl, tl2)));
                continue;
            }
            list.push(op);
        }
        StyleTransform { op_list: list }
    }

    pub fn apply(&self, width: f32, height: f32, mc: &mut MatrixCalculator) {
        for op in &self.op_list {
            match op {
                StyleTransformOp::Rotate(deg) => {
                    mc.rotate(*deg, None);
                }
                StyleTransformOp::Scale(ScaleParams(x, y)) => {
                    mc.scale((*x, *y));
                }
                StyleTransformOp::Translate(params) => {
                    let (x, y) = (&params.0, &params.1);
                    let x = x.to_absolute(width);
                    let y = y.to_absolute(height);
                    mc.translate((x, y));
                }
            }
        }
    }
}

fn parse_rotate_op(value: &str) -> Option<StyleTransformOp> {
    if let Some(v) = value.strip_suffix("deg") {
        let v = f32::from_str(v).ok()?;
        Some(StyleTransformOp::Rotate(v))
    } else {
        None
    }
}

fn parse_scale_op(value: &str) -> Option<StyleTransformOp> {
    let mut values = value.split(",").collect::<Vec<&str>>();
    if values.len() < 2 {
        values.push(values[0]);
    }
    let x = f32::from_str(values[0].trim()).ok()?;
    let y = f32::from_str(values[1].trim()).ok()?;
    Some(StyleTransformOp::Scale(ScaleParams(x, y)))
}

fn parse_translate_op(value: &str) -> Option<StyleTransformOp> {
    let mut values = value.split(",").collect::<Vec<&str>>();
    if values.len() < 2 {
        values.push(values[0]);
    }
    let x = parse_translate_length(values[0].trim())?;
    let y = parse_translate_length(values[1].trim())?;
    Some(StyleTransformOp::Translate(TranslateParams(x, y)))
}

fn parse_translate_length(value: &str) -> Option<TranslateLength> {
    if let Some(v) = parse_percent(value) {
        Some(TranslateLength::Percent(v))
    } else {
        let v = f32::from_str(value).ok()?;
        Some(TranslateLength::Point(v))
    }
}
