use super::{fits, is_small, plan, Incoming, MilestonePlan, Recorded, Source, CAP, IDLE};
use crate::history::delta::single;
use crate::history::store::VersionFile;
use crate::text::content::Splice;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_800_000_000 + seconds)
}

fn version(id: &str, seconds: u64, source: Source, device: &str) -> VersionFile {
    VersionFile {
        id: id.to_owned(),
        at_ms: (1_800_000_000 + seconds) * 1000,
        device: device.to_owned(),
        source,
        from_ms: source.origin_ms(),
        name: None,
        is_current: false,
    }
}

fn typing(source: Source) -> Incoming {
    Incoming { source, same_as_latest: false, same_as_previous: false, original_differs: false, small: true, fits_latest: true }
}

fn run(incoming: Incoming, versions: &[VersionFile], last_edit: Option<SystemTime>, now: SystemTime) -> MilestonePlan {
    plan("dev1", &incoming, &Recorded { versions, names: &HashMap::new(), last_edit }, now)
}

#[test]
fn the_first_edit_keeps_the_original_and_starts_a_version() {
    let incoming = Incoming { original_differs: true, ..typing(Source::Me) };
    assert_eq!(run(incoming, &[], None, at(20)), MilestonePlan::CreateNew { keep_original: true });
    assert_eq!(run(typing(Source::Me), &[], None, at(20)), MilestonePlan::CreateNew { keep_original: false });
}

#[test]
fn typing_within_idle_coalesces_and_after_idle_or_cap_starts_anew() {
    let versions = [version("1_dev1_me.delta", 0, Source::Me, "dev1")];
    let update = MilestonePlan::UpdateLatest { id: versions[0].id.clone() };
    assert_eq!(run(typing(Source::Me), &versions, Some(at(10)), at(20)), update);
    let idle = at(10) + IDLE + Duration::from_secs(1);
    assert_eq!(run(typing(Source::Me), &versions, Some(at(10)), idle), MilestonePlan::CreateNew { keep_original: false });
    let capped = at(0) + CAP + Duration::from_secs(1);
    assert_eq!(run(typing(Source::Me), &versions, Some(capped), capped), MilestonePlan::CreateNew { keep_original: false });
}

#[test]
fn a_big_change_another_device_or_a_full_delta_starts_anew() {
    let versions = [version("1_dev1_me.delta", 0, Source::Me, "dev1")];
    let new = MilestonePlan::CreateNew { keep_original: false };
    assert_eq!(run(Incoming { small: false, ..typing(Source::Me) }, &versions, Some(at(10)), at(20)), new);
    assert_eq!(run(Incoming { fits_latest: false, ..typing(Source::Me) }, &versions, Some(at(10)), at(20)), new);
    let other = [version("1_dev2_me.delta", 0, Source::Me, "dev2")];
    assert_eq!(run(typing(Source::Me), &other, Some(at(10)), at(20)), new);
    assert_eq!(run(typing(Source::Agent), &versions, Some(at(10)), at(20)), new);
}

#[test]
fn returning_to_the_previous_text_removes_the_draft() {
    let versions = [version("1_dev1_start.md", 0, Source::Start, "dev1"), version("2_dev1_me.delta", 5, Source::Me, "dev1")];
    let incoming = Incoming { same_as_previous: true, ..typing(Source::Me) };
    assert_eq!(run(incoming, &versions, Some(at(10)), at(20)), MilestonePlan::RevertToPrevious { remove_id: versions[1].id.clone() });
}

#[test]
fn reading_progress_is_quiet_and_the_same_text_is_skipped() {
    let versions = [version("1_dev1_me.delta", 0, Source::Me, "dev1")];
    assert_eq!(run(typing(Source::Reading), &versions, None, at(20)), MilestonePlan::Quiet);
    let same = Incoming { same_as_latest: true, ..typing(Source::Agent) };
    assert_eq!(run(same, &versions, None, at(20)), MilestonePlan::Skip);
}

#[test]
fn restore_and_revert_labels_roundtrip() {
    for source in [Source::Restore(1_234), Source::Revert(5), Source::Me, Source::Start] {
        assert_eq!(Source::from_label(&source.file_label()), Some(source));
    }
}

#[test]
fn a_change_is_small_by_characters_and_fits_by_bytes() {
    let base = "было";
    let text = format!("было{}", "я".repeat(150));
    let splice = Splice { at: base.len(), removed: 0, inserted: text.len() - base.len() };
    assert!(is_small(&text.as_str(), &single(splice)).unwrap());
    let long = format!("было{}", "я".repeat(250));
    assert!(!is_small(&long.as_str(), &single(Splice { inserted: long.len() - base.len(), ..splice })).unwrap());
    assert!(!is_small(&text.as_str(), &single(Splice { removed: 200, ..splice })).unwrap(), "удаление в 200 байт не мелкая правка");
    assert!(fits(1000, &single(splice)) && !fits(1 << 20, &single(splice)));
}
