
#[macro_export]
macro_rules! backend_as_api {
    ($trait_name: ident, $ty: ty, $as_name: ident, $as_mut_name: ident) => {
        pub trait $trait_name {
            fn $as_name(&self) -> &$ty;
            fn $as_mut_name(&mut self) -> &mut $ty;
        }

        impl $trait_name for Element {
            fn $as_name(&self) -> &$ty {
                self.get_backend_as::<$ty>()
            }

            fn $as_mut_name(&mut self) -> &mut $ty {
                self.get_backend_mut_as::<$ty>()
            }
        }
    };
}

#[macro_export]
macro_rules! compute_style {
    ($key: ident, $element: expr, $field: ident, $default: expr) => {
        {
            use crate::style::PropValue;
            let mut queue = std::collections::VecDeque::new();
            queue.push_back($element.clone());
            loop {
                let mut e = match queue.pop_front() {
                    Some(e) => e,
                    None => break,
                };
                let new_value = match e.inner.$field {
                    PropValue::Inherit => {
                        if let Some(p) = e.get_parent() {
                            p.computed_style.$field
                        } else {
                            $default
                        }
                    }
                    PropValue::Custom(c) => {
                        c
                    }
                };
                if e.computed_style.$field != new_value {
                    e.computed_style.$field = new_value;
                    if let Some(on_changed) = &mut e.on_changed {
                        (on_changed)(StylePropKey::$key);
                    }
                    for child in e.children.iter() {
                        queue.push_back(child.clone());
                    }
                }
            }
        }
    };
}

#[macro_export]
macro_rules! inherit_color_prop {
    ($update_fn: ident, $update_children_fn: ident, $field: ident, $key: expr, $default: expr) => {
        pub fn $update_fn(&mut self) {
            self.computed_style.$field = match self.inner.$field {
                ColorPropValue::Inherit => {
                    if let Some(p) = self.get_parent() {
                        p.computed_style.$field
                    } else {
                        $default
                    }
                }
                ColorPropValue::Color(c) => {c}
            };
            //TODO check change?
            if let Some(on_changed) = &mut self.on_changed {
                (on_changed)($key);
            }
            self.$update_children_fn();
        }

        pub fn $update_children_fn(&mut self) {
            for mut c in self.get_children().clone() {
                match c.$field {
                    ColorPropValue::Inherit => {
                        c.computed_style.$field = self.computed_style.$field;
                        //TODO check change?
                        if let Some(on_changed) = &mut c.on_changed {
                           (on_changed)($key);
                        }
                        c.$update_children_fn();
                    },
                    _ => {}
                }
            }
        }

    };
}

#[macro_export]
macro_rules! create_element {
    ($ty: ty,  { $($key: expr => $value: expr,)* }) => {
        {
            let mut element = Element::create(<$ty>::create);
            use crate::HashMap;
            let mut style = Vec::new();
            $(
                if let Some(p) = crate::element::StyleProp::parse(stringify!($key), $value) {
                   style.push(p);
                }
            )*
            element.set_style_props(style);
            element
        }
    };
}

#[macro_export]
macro_rules! tree {
    ($node: expr, [ $($child: expr,)* ]) => {
        {
            $($node.add_child_view($child.clone(), None);)*
            $node.clone()
        }

    };
}