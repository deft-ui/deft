#[macro_export]
macro_rules! backend_as_api {
    ($trait_name: ident, $ty: ty, $as_name: ident, $as_mut_name: ident) => {
        pub trait $trait_name {
            fn $as_name(&self) -> &$ty;
            fn $as_mut_name(&mut self) -> &mut $ty;
        }

        impl $trait_name for ElementRef {
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
            let mut element = ElementRef::new(<$ty>::create);
            use crate::HashMap;
            use crate::element::AllStylePropertyKey;
            use crate::element::StylePropertyValue;
            let mut style = HashMap::new();
            $(
                if let Some(p) = AllStylePropertyKey::from_str(stringify!($key)) {
                   let v = StylePropertyValue::from_str($value);
                   style.insert(p, v);
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