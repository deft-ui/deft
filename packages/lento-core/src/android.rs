use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jlong};
use crate::app::AppEvent;
use crate::event_loop::{send_event};

#[no_mangle]
pub extern "system" fn Java_fun_kason_lento_InputChannel_send<'local>(mut env: JNIEnv<'local>,
                                                                     class: JClass<'local>,
                                                                     window_id: jlong,
                                                                     input: JString<'local>)
{
    let input: String =
        env.get_string(&input).expect("Couldn't get java string!").into();
    println!("receive input:{}", input);
    send_event(AppEvent::CommitInput(window_id as i32, input)).unwrap();
}