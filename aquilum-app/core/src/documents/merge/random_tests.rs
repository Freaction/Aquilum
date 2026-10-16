use super::merge_external_change;
use super::tests::merge;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: u64) -> usize {
        self.0 = (self.0 * 1_103_515_245 + 12_345) % 2_147_483_648;
        (self.0 % bound) as usize
    }
}

fn random_lines(random: &mut Lcg, max: u64, empty_one_in: u64) -> Vec<String> {
    (0..random.next(max))
        .map(|index| if random.next(empty_one_in) == 0 { String::new() } else { format!("строка {index}") })
        .collect()
}

fn mutate(random: &mut Lcg, base: &[String], max_changes: u64) -> Vec<String> {
    let mut disk = base.to_vec();
    for change in (1..=random.next(max_changes)).rev() {
        let at = random.next(disk.len() as u64 + 1);
        if random.next(2) == 0 || at >= disk.len() {
            disk.insert(at, format!("вставка {change}"));
        } else {
            disk[at] = format!("замена {change}");
        }
    }
    disk
}

#[test]
fn never_reorders_the_file_when_an_empty_document_takes_it() {
    let mut random = Lcg(20_260_818);
    for _ in 0..400 {
        let disk = random_lines(&mut random, 14, 3).join("\n");
        assert_eq!(merge("", &disk, "").text, disk);
    }
}

#[test]
fn lands_exactly_on_the_file_whenever_the_document_has_no_local_change() {
    let mut random = Lcg(20_260_817);
    for _ in 0..400 {
        let base = random_lines(&mut random, 12, 4);
        let disk = mutate(&mut random, &base, 4);
        let base_text = base.join("\n");
        let disk_text = disk.join("\n");
        assert_eq!(merge(&base_text, &disk_text, &base_text).text, disk_text);
    }
}

#[test]
fn reports_edits_that_never_share_a_position() {
    let mut random = Lcg(20_260_819);
    for _ in 0..400 {
        let base = random_lines(&mut random, 14, 3);
        let disk = mutate(&mut random, &base, 5);
        let current = if random.next(3) == 0 { String::new() } else { base.join("\n") };
        let result = merge_external_change(&base.join("\n"), &disk.join("\n"), &current);
        let mut reach: Option<usize> = None;
        for edit in &result.edits {
            if let Some(reach) = reach {
                assert!(edit.from > reach);
            }
            assert!(edit.to >= edit.from);
            reach = Some(edit.to);
        }
    }
}
