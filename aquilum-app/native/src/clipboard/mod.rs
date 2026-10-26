#[cfg(not(target_os = "android"))]
mod desktop;

#[cfg(target_os = "android")]
pub use crate::android::clipboard::*;
#[cfg(not(target_os = "android"))]
pub use desktop::*;
