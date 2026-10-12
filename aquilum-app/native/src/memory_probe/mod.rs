mod counting;
mod probe;
mod process;
mod script;
mod trace;

pub use counting::Counting;
pub use probe::Probe;
pub use script::Step;

#[cfg(test)]
pub(crate) fn heap() -> usize {
    counting::CURRENT.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn trace(on: bool) {
    trace::TRACE.store(on, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn report() -> String {
    let mut text = process::breakdown();
    if trace::TRACE.load(std::sync::atomic::Ordering::Relaxed) {
        text.push_str(&trace::large_report());
    }
    text
}
