use std::sync::OnceLock;
use jni::JNIEnv;
use jni::objects::{JClass, JString, JValue};
use jni::sys::{jboolean, jfloat, jint, jlong};
use skia_safe::Rect;
use winit::platform::android::activity::AndroidApp;
use crate::app::{AppEvent, InsetType};
use crate::event_loop::{create_event_loop_proxy};
use crate::send_app_event;
use log::debug;

pub static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

pub fn init_android_app(app: &AndroidApp) {
    let app = app.clone();
    ANDROID_APP.get_or_init(move || app);
}

#[no_mangle]
pub extern "system" fn Java_deft_DeftActivity_send<'local>(mut env: JNIEnv<'local>,
                                                                     class: JClass<'local>,
                                                                     window_id: jlong,
                                                                     input: JString<'local>)
{
    let input: String =
        env.get_string(&input).expect("Couldn't get java string!").into();
    debug!("receive input:{}", input);
    if let Err(e) = send_app_event(AppEvent::CommitInput(window_id as i32, input)) {
        debug!("send app event error: {:?}", e);
    }
}

#[no_mangle]
pub extern "system" fn Java_deft_DeftActivity_sendKey0<'local>(mut env: JNIEnv<'local>,
                                                                      class: JClass<'local>,
                                                                      window_id: jlong,
                                                                      input: JString<'local>,
                                                                      pressed: jboolean)
{
    let input: String =
        env.get_string(&input).expect("Couldn't get java string!").into();
    debug!("receive key input:{}", input);
    if let Err(e) = send_app_event(AppEvent::NamedKeyInput(window_id as i32, input, pressed != 0)) {
        debug!("send app event error: {:?}", e);
    }
}

#[no_mangle]
pub extern "system" fn Java_deft_DeftActivity_setInset0<'local>(mut env: JNIEnv<'local>,
                                                                 class: JClass<'local>,
                                                                 window_id: jlong,
                                                                 inset_type: jint,
                                                                 top: jfloat,
                                                                 right: jfloat,
                                                                 bottom: jfloat,
                                                                 left: jfloat,)
{
    if let Some(ty) = InsetType::from_i32(inset_type) {
        let rect = Rect::new(left, top, right, bottom);
        debug!("setInset0,{} {:?}, {:?}", window_id, ty, rect);
        send_app_event(AppEvent::SetInset(window_id as i32, ty, rect));
    } else {
        debug!("unknown inset type: {:?}", inset_type);
    }
}

pub fn clipboard_write_text(content: &str) -> Result<(), jni::errors::Error> {
    use jni::JavaVM;
    use jni::objects::JObject;
    let app = ANDROID_APP.get().unwrap();
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as _)? };
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as _) };
    let mut env = vm.attach_current_thread()?;
    let content = env.new_string(content)?;
    env.call_method(&activity, "setClipboardText", "(Ljava/lang/String;)V", &[
        JValue::Object(&content)
    ])?.v()?;
    Ok(())
}