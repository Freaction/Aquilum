use super::{Interface, MAX_SCALE, SCALE_STEP};

#[test]
fn scale_is_rounded_clamped_and_persisted() {
    let dir = crate::test_dir::tempdir();
    let mut interface = Interface::load(dir.path());
    assert_eq!(interface.scale(), 1.0);
    assert!(interface.set_scale(1.0 + SCALE_STEP * 3.0));
    assert_eq!(interface.scale(), 1.15);
    assert!(interface.set_scale(9.0));
    assert_eq!(interface.scale(), MAX_SCALE);
    assert!(!interface.set_scale(9.0));
    interface.set_sidebar_open(false);
    crate::disk::wait();
    let reloaded = Interface::load(dir.path());
    assert_eq!(reloaded.scale(), MAX_SCALE);
    assert!(!reloaded.sidebar_open());
}
