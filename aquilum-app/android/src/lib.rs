#![cfg(target_os = "android")]

use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JClass, JString};

pub use aquilum_native::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    aquilum_native::android_main(app);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dmitriy_aquilum_MainActivity_onLink<'local>(mut env: EnvUnowned<'local>, _: JClass<'local>, link: JString<'local>) {
    env.with_env(|env| -> jni::errors::Result<()> {
        aquilum_native::android_link(link.try_to_string(env)?);
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
