use std::collections::HashMap;
#[macro_export]
macro_rules! set_style {
    ($element: expr, { $($key: expr => $value: expr,)* }) => {
        use crate::HashMap;
        let mut style = Vec::new();
        $(
            if let Some(p) = crate::element::StyleProp::parse(stringify!($key), $value) {
               style.push(p);
            } else {
                eprintln!("invalid key:{}", stringify!($key));
            }
        )*
        $element.set_style_props(style);
    };
}