use quick_js::JsValue;
use crate::base::EventContext;
use crate::js::js_value_util::EventResult;

pub fn create_event_handler<T>(event_name: &str, callback: JsValue) -> Box<dyn Fn(&mut EventContext<T>, i32, JsValue)> {
    let en = event_name.to_string();
    Box::new(move |ctx: &mut EventContext<T>, target, detail| {
        let callback_result = callback.call_as_function(vec![
            //TODO remove int(0)
            JsValue::Int(0), JsValue::String(en.clone()), detail, JsValue::Int(target),
        ]);
        if let Ok(cb_result) = callback_result {
            use crate::js::js_value_util::FromJsValue;
            ;
            if let Ok(res) = EventResult::from_js_value(cb_result) {
                if res.propagation_cancelled {
                    ctx.propagation_cancelled = true;
                }
                if res.prevent_default {
                    ctx.prevent_default = true;
                }
            }
        }
    })
}