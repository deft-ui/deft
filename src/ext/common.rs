use crate::base::EventContext;
use crate::js::js_value_util::EventResult;
use crate::js::FromJsValue;
use quick_js::JsValue;

pub fn create_event_handler(
    event_name: &str,
    callback: JsValue,
) -> Box<dyn Fn(&mut EventContext, JsValue)> {
    let en = event_name.to_string();
    Box::new(move |ctx: &mut EventContext, detail| {
        let callback_result =
            callback.call_as_function(vec![JsValue::String(en.clone()), detail]);
        if let Ok(cb_result) = callback_result {
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
