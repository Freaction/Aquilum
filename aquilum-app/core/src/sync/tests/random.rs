use super::device::Device;
use super::fake::FakeRemote;
use crate::documents::merge::random_tests::Lcg;

const NOTES: [&str; 4] = ["Один.md", "Два.md", "Папка/Три.md", "Папка/Четыре.md"];

fn edit(device: &Device, note: &str, line: &str, random: &mut Lcg) {
    let mut lines: Vec<String> = device.read(note).map(|text| text.lines().map(str::to_owned).collect()).unwrap_or_default();
    let at = random.next(lines.len() as u64 + 1);
    lines.insert(at, line.to_owned());
    device.write(note, &(lines.join("\n") + "\n"));
}

fn play(seed: u64) {
    let mut random = Lcg(seed);
    let remote = FakeRemote::default();
    let devices = [Device::new(), Device::new(), Device::new()];
    let mut typed = Vec::new();
    for step in 0..40 {
        let device = &devices[random.next(3)];
        let note = NOTES[random.next(NOTES.len() as u64)];
        match random.next(10) {
            0..=4 => {
                let line = format!("строка {seed}-{step}");
                edit(device, note, &line, &mut random);
                typed.push(line);
            }
            5 if device.read(note).is_some() => device.trash(note),
            _ => {
                remote.refuse_uploads(random.next(5) == 0);
                device.sync(&remote);
            }
        }
    }
    remote.refuse_uploads(false);
    let quiet = (0..10).any(|_| {
        let outcomes: Vec<_> = devices.iter().map(|device| device.sync(&remote)).collect();
        outcomes.iter().all(|outcome| outcome.failed.is_empty() && outcome.progress.total == 0)
    });
    assert!(quiet, "зерно {seed}: устройства не успокоились за 10 кругов");
    for device in &devices {
        assert_eq!(device.files(), remote.paths(), "зерно {seed}: файлы разошлись");
        for path in device.files() {
            assert_eq!(device.read(&path), remote.text(&path), "зерно {seed}: разный текст {path}");
        }
    }
    let kept: String = devices.iter().map(Device::kept_text).collect::<Vec<_>>().join("\n");
    for line in typed {
        assert!(kept.lines().any(|kept| kept == line), "зерно {seed}: пропала строка «{line}»");
    }
}

#[test]
fn many_devices_converge_and_lose_no_line() {
    for seed in 1..=25 {
        play(seed);
    }
}

#[test]
#[ignore]
fn many_devices_converge_and_lose_no_line_long() {
    for seed in 26..=525 {
        play(seed);
    }
}
