#[macro_export]
macro_rules! define_ref_and_resource {
    ($ty: ident, $target_ty: ty) => {
        crate::define_ref!($ty, $target_ty);
        crate::define_resource!($ty);
    };
}

#[macro_export]
macro_rules! define_resource {
    ($ty: ident) => {
        impl crate::js::js_value_util::ToJsValue for $ty {
            fn to_js_value(self) -> Result<JsValue, Error> {
                Ok(JsValue::Resource(quick_js::ResourceValue { resource: std::rc::Rc::new(std::cell::RefCell::new(self)) }))
            }
        }

        impl crate::js::js_value_util::FromJsValue for $ty {
            fn from_js_value(value: JsValue) -> Result<Self, Error> {
                if let Some(r) = value.as_resource(|r: &mut $ty| r.clone()) {
                    Ok(r)
                } else {
                    use anyhow::anyhow;
                    Err(anyhow!("invalid value"))
                }
            }
        }
    };
}