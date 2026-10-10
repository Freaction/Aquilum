#[cfg(target_os = "windows")]
mod dxgi;
#[cfg(not(target_os = "windows"))]
mod soft;

#[cfg(target_os = "windows")]
pub use dxgi::Output;
#[cfg(not(target_os = "windows"))]
pub use soft::Output;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}
