#![cfg(target_os = "android")]

use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JClass, JString};
use jni::sys::jint;

pub use aquilum_native::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    aquilum_native::android_main(app);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dmitriy_aquilum_ImeView_onImeState<'local>(
    mut env: EnvUnowned<'local>,
    _: JClass<'local>,
    seq: jint,
    text: JString<'local>,
    anchor: jint,
    cursor: jint,
    start: jint,
    end: jint,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let unit = |value: jint| usize::try_from(value).ok();
        let compose = unit(start).zip(unit(end));
        aquilum_native::android_ime_state(seq, text.try_to_string(env)?, (unit(anchor).unwrap_or(0), unit(cursor).unwrap_or(0)), compose);
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dmitriy_aquilum_MainActivity_onLink<'local>(mut env: EnvUnowned<'local>, _: JClass<'local>, link: JString<'local>) {
    env.with_env(|env| -> jni::errors::Result<()> {
        aquilum_native::android_link(link.try_to_string(env)?);
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
