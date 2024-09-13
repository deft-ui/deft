use std::collections::HashMap;
#[macro_export]
macro_rules! set_style {
    ($element: expr, { $($key: expr => $value: expr,)* }) => {
        use crate::HashMap;
        use crate::element::AllStylePropertyKey;
        use crate::element::StylePropertyValue;
        let mut style = HashMap::new();
        $(
            if let Some(p) = AllStylePropertyKey::from_str(stringify!($key)) {
               let v = StylePropertyValue::from_str($value);
               style.insert(p, v);
            } else {
                eprintln!("invalid key:{}", stringify!($key));
            }
        )*
        $element.set_style_props(style);
    };
}