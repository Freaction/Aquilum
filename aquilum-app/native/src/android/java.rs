use std::sync::OnceLock;

use jni::objects::{JObject, JString, JValue};
use jni::strings::JNIStr;
use jni::{Env, JavaVM, jni_sig};
use winit::platform::android::activity::AndroidApp;

static APP: OnceLock<AndroidApp> = OnceLock::new();

pub fn init(app: &AndroidApp) {
    let _ = APP.set(app.clone());
}

fn call<T>(work: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<T>) -> Option<T> {
    let app = APP.get()?;
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let activity = app.activity_as_ptr() as jni::sys::jobject;
    vm.attach_current_thread(|env| -> jni::errors::Result<T> {
        let activity = unsafe { env.as_cast_raw::<JObject>(&activity)? };
        work(env, &activity)
    })
    .map_err(|error| eprintln!("jni: {error}"))
    .ok()
}

pub fn send_text(method: &JNIStr, text: &str) -> bool {
    call(|env, activity| {
        let text = JString::from_str(env, text)?;
        env.call_method(activity, method, jni_sig!("(Ljava/lang/String;)V"), &[JValue::Object(&text)])?;
        Ok(())
    })
    .is_some()
}

pub fn send_progress(method: &JNIStr, text: &str, done: i32, total: i32) -> bool {
    call(|env, activity| {
        let text = JString::from_str(env, text)?;
        env.call_method(activity, method, jni_sig!("(Ljava/lang/String;II)V"), &[JValue::Object(&text), JValue::Int(done), JValue::Int(total)])?;
        Ok(())
    })
    .is_some()
}

pub fn read_text(method: &JNIStr) -> Option<String> {
    call(|env, activity| {
        let value = env.call_method(activity, method, jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let text = env.cast_local::<JString>(value)?;
        if text.is_null() {
            return Ok(None);
        }
        text.try_to_string(env).map(Some)
    })
    .flatten()
}

pub fn read_long(method: &JNIStr) -> Option<i64> {
    call(|env, activity| env.call_method(activity, method, jni_sig!("()J"), &[])?.j())
}

pub fn send_bool(method: &JNIStr, value: bool) -> bool {
    call(|env, activity| {
        env.call_method(activity, method, jni_sig!("(Z)V"), &[JValue::Bool(value)])?;
        Ok(())
    })
    .is_some()
}

pub fn run(method: &JNIStr) -> bool {
    call(|env, activity| env.call_method(activity, method, jni_sig!("()V"), &[]).map(drop)).is_some()
}
