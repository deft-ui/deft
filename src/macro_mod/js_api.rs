#![allow(unused_mut)]
use quick_js::JsValue;
#[macro_export]
macro_rules! export_js_async_api {
    ($js_ctx: expr, $name: expr, $func: ident, $in_type: ty) => {
        {
            let js_ctx = $js_ctx.clone();
            $js_ctx.add_callback($name, move | params: JsValue| {
                use crate::js::js_value_util::SerializeToJsValue;
                use crate::js::js_value_util::ToJsValue;
                use crate::js::js_value_util::FromJsValue;
                let mut js_ctx = js_ctx.clone();
                let params = <$in_type>::from_js_value(params)?;
                let result = js_ctx.create_async_task(async move {
                    let result = $func(params).await;
                    match result {
                        Ok(r) => {
                            // let serializer = JsValueSerializer {};
                            // let js_r = r.serialize(serializer)?;
                            Ok(r.to_js_value()?)
                        },
                        Err(e) => Err(anyhow!(e)),
                    }
                });
                Ok::<_, anyhow::Error>(result)
            }).unwrap();
        }
    };
    ($js_ctx: expr, $name: expr, $func: ident, $in_type: ty, $in_type2: ty) => {
        {
            let js_ctx = $js_ctx.clone();
            $js_ctx.add_callback($name, move | p1: JsValue, p2: JsValue| {
                let mut js_ctx = js_ctx.clone();
                use crate::js::js_value_util::SerializeToJsValue;
                use crate::js::js_value_util::ToJsValue;
                use crate::js::js_value_util::FromJsValue;
                let p1 = <$in_type>::from_js_value(p1)?;
                let p2 = <$in_type2>::from_js_value(p2)?;
                let result = js_ctx.create_async_task(async move {
                    let result = $func(p1, p2).await;
                    match result {
                        Ok(r) => {
                            Ok(r.to_js_value()?)
                        },
                        Err(e) => Err(anyhow!(e)),
                    }
                });
                Ok::<_, anyhow::Error>(result)
            }).unwrap();
        }
    };
}

#[macro_export]
macro_rules! export_js_object_api_raw {
    ($js_ctx: expr, $name: expr, $obj_type: ty, $func: ident, $($pn: ident => $in_type: ty), *) => {
        {
            $js_ctx.add_callback($name, move |js_obj: JsValue, $($pn: JsValue,)* | {
                use crate::js::js_value_util::FromJsValue;
                use crate::js::js_value_util::ToJsValue;
                use crate::js::js_value_util::SerializeToJsValue;
                use crate::js::js_value_util::SerializeResultToJsValue;
                use crate::js::js_value_util::ResultToJsValue;
                let mut obj = <$obj_type>::from_js_value(js_obj)?;
                obj.$func(
                    $(
                        <$in_type>::from_js_value($pn)?,
                    )*
                ).to_js_value()
            }).unwrap();
        }
    };
}

#[macro_export]
macro_rules! export_js_object_api {
    ($js_ctx: expr, $name: expr, $obj_type: ty, $func: ident) => {
        {
            crate::export_js_object_api_raw!($js_ctx, $name, $obj_type, $func, )
        }

    };
    ($js_ctx: expr, $name: expr, $obj_type: ty, $func: ident, $in_type: ty) => {
        {
            crate::export_js_object_api_raw!($js_ctx, $name, $obj_type, $func, p1 => $in_type)
        }

    };
    ($js_ctx: expr, $name: expr, $obj_type: ty, $func: ident, $in_type: ty, $in_type2: ty) => {
        crate::export_js_object_api_raw!($js_ctx, $name, $obj_type, $func,  p1 => $in_type, p2 => $in_type2)
    };
    ($js_ctx: expr, $name: expr, $obj_type: ty, $func: ident, $in_type: ty, $in_type2: ty, $in_type3: ty) => {
        crate::export_js_object_api_raw!($js_ctx, $name, $obj_type, $func, p1 => $in_type, p2 => $in_type2, p3 => $in_type3)
    }
}
#[macro_export]
macro_rules! export_js_api_raw {
    ($js_ctx: expr, $name: expr, $func: ident, $($pn: ident => $in_type: ty), *) => {
        {
            $js_ctx.add_callback($name, move | $($pn: JsValue,)* | {
                use crate::js::js_value_util::FromJsValue;
                use crate::js::js_value_util::ToJsValue;
                use crate::js::js_value_util::SerializeToJsValue;
                let result = $func(
                    $(
                        <$in_type>::from_js_value($pn)?,
                    )*
                );
                match result {
                    Ok(r) => {
                        Ok(r.to_js_value()?)
                    }
                    Err(e) => Err(anyhow!(e)),
                }
            }).unwrap();
        }
    };
}

#[macro_export]
macro_rules! export_js_api {
    ($js_ctx: expr, $name: expr, $func: ident) => {
        {
            $js_ctx.add_callback($name, move || {
                let result = $func();
                use crate::js::js_value_util::FromJsValue;
                use crate::js::js_value_util::ToJsValue;
                use crate::js::js_value_util::SerializeToJsValue;
                match result {
                    Ok(r) => {
                        //let serializer = JsValueSerializer {};
                        //let js_r = r.serialize(serializer)?;
                        Ok(r.to_js_value()?)
                    },
                    Err(e) => Err(anyhow!(e)),
                }
            }).unwrap();
        }
    };
    ($js_ctx: expr, $name: expr, $func: ident, $in_type: ty) => {
        {
            use crate::export_js_api_raw;
            export_js_api_raw!($js_ctx, $name, $func, p1 => $in_type)
        }
    };
    ($js_ctx: expr, $name: expr, $func: ident, $in_type: ty, $in_type2: ty) => {
        {
            use crate::export_js_api_raw;
            export_js_api_raw!($js_ctx, $name, $func, p1 => $in_type, p2 => $in_type2)
        }
    };
        ($js_ctx: expr, $name: expr, $func: ident, $in_type: ty, $in_type2: ty, $in_type3: ty) => {
        {
            use crate::export_js_api_raw;
            export_js_api_raw!($js_ctx, $name, $func, p1 => $in_type, p2 => $in_type2, p3 => $in_type3)
        }
    };
}