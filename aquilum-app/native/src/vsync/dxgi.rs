use std::time::Duration;

use windows::Win32::Graphics::Dwm::{DWM_TIMING_INFO, DwmFlush, DwmGetCompositionTimingInfo};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, IDXGIOutput};

fn primary_output() -> Option<IDXGIOutput> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
    let adapter = unsafe { factory.EnumAdapters1(0) }.ok()?;
    unsafe { adapter.EnumOutputs(0) }.ok()
}

thread_local! {
    static OUTPUT: Option<IDXGIOutput> = primary_output();
}

pub(super) fn wait() -> bool {
    OUTPUT.with(|output| output.as_ref().is_some_and(|o| unsafe { o.WaitForVBlank() }.is_ok())) || unsafe { DwmFlush() }.is_ok()
}

pub(super) fn period() -> Option<Duration> {
    let mut info = DWM_TIMING_INFO { cbSize: size_of::<DWM_TIMING_INFO>() as u32, ..Default::default() };
    unsafe { DwmGetCompositionTimingInfo(windows::Win32::Foundation::HWND::default(), &mut info) }.ok()?;
    let rate = info.rateRefresh;
    (rate.uiNumerator > 0 && rate.uiDenominator > 0).then(|| Duration::from_secs_f64(f64::from(rate.uiDenominator) / f64::from(rate.uiNumerator)))
}
