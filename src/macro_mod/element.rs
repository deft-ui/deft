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
macro_rules! create_element {
    ($ty: ty,  { $($key: expr => $value: expr,)* }) => {
        {
            let mut element = Element::create(<$ty>::create);
            use crate::HashMap;
            let mut style = Vec::new();
            $(
                if let Some(p) = crate::element::FixedStyleProp::parse(stringify!($key), $value) {
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
