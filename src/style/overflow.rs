use crate::style::PropValueParse;

#[derive(Clone, Debug, PartialEq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Overflow {
    pub fn to_yoga_overflow(&self) -> yoga::Overflow {
        match self {
            Overflow::Visible => yoga::Overflow::Visible,
            Overflow::Hidden => yoga::Overflow::Hidden,
            Overflow::Scroll => yoga::Overflow::Scroll,
            Overflow::Auto => yoga::Overflow::Scroll,
        }
    }
}

impl PropValueParse for Overflow {
    fn parse_prop_value(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            "scroll" => Some(Self::Scroll),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
    fn to_style_string(&self) -> String {
        match self {
            Overflow::Visible => "visible",
            Overflow::Hidden => "hidden",
            Overflow::Scroll => "scroll",
            Overflow::Auto => "auto",
        }
        .to_owned()
    }
}
