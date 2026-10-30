#![cfg(target_os = "android")]

use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JCharArray, JClass, JIntArray, JString};
use jni::sys::{jboolean, jint};

pub use aquilum_native::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    aquilum_native::android_main(app);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dmitriy_aquilum_ImeView_onImeState<'local>(
    mut env: EnvUnowned<'local>,
    _: JClass<'local>,
    session: jint,
    seq: jint,
    echo: jboolean,
    text: JCharArray<'local>,
    marks: JIntArray<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let mut units = vec![0; text.len(env)?];
        text.get_region(env, 0, &mut units)?;
        let mut points = [0; 5];
        marks.get_region(env, 0, &mut points)?;
        let unit = |value: jint| usize::try_from(value).ok();
        let [anchor, cursor, start, end, base] = points.map(unit);
        let state = aquilum_native::ImeState::from_utf16(String::from_utf16_lossy(&units), (anchor.unwrap_or(0), cursor.unwrap_or(0)), start.zip(end));
        aquilum_native::android_ime_state(aquilum_native::ImeMessage { session, seq, echo, base, state });
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dmitriy_aquilum_DocumentActivity_onDocument<'local>(mut env: EnvUnowned<'local>, _: JClass<'local>, uri: JString<'local>) {
    env.with_env(|env| -> jni::errors::Result<()> {
        aquilum_native::android_document(if uri.is_null() { None } else { uri.try_to_string(env).ok() });
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
