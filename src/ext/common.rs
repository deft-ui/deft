use crate::base::EventContext;
use crate::js::js_value_util::EventResult;
use crate::js::{FromJsValue, ToJsValue};
use log::error;
use quick_js::JsValue;

pub fn create_event_handler<T: ToJsValue + Clone>(
    event_name: &str,
    callback: JsValue,
) -> Box<dyn Fn(&mut EventContext<T>, JsValue)> {
    let en = event_name.to_string();
    Box::new(move |ctx: &mut EventContext<T>, detail| {
        let target = match ctx.target.clone().to_js_value() {
            Ok(target) => target,
            Err(e) => {
                error!("failed to convert target to js value: {:?}", e);
                return;
            }
        };
        let callback_result =
            callback.call_as_function(vec![JsValue::String(en.clone()), detail, target]);
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
