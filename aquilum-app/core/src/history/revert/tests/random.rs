use super::{between, undo, Edited, Map};
use crate::documents::merge::random_tests::Lcg;

const WORDS: [&str; 10] = ["кот", "🐈", "дом", "ёж", "a", "Ω", "мир", "🌍", "", "  "];
const BLOCKS: usize = 6;

fn word(random: &mut Lcg) -> &'static str {
    WORDS[random.next(WORDS.len() as u64)]
}

fn line(random: &mut Lcg) -> String {
    (0..1 + random.next(3)).map(|_| word(random)).collect::<Vec<_>>().join(" ")
}

fn piece(random: &mut Lcg, eol: &str) -> String {
    (0..random.next(5)).map(|_| if random.next(3) == 0 { eol } else { word(random) }).collect()
}

fn end_of_line(random: &mut Lcg) -> &'static str {
    ["\n", "\r\n"][random.next(2)]
}

fn random_splice(random: &mut Lcg, edited: &mut Edited, eol: &str) {
    let bounds: Vec<usize> = edited.text.char_indices().map(|(at, _)| at).chain([edited.text.len()]).collect();
    let at = bounds[random.next(bounds.len() as u64)];
    let end = edited.text[at..].char_indices().nth(random.next(5)).map_or(edited.text.len(), |(offset, _)| at + offset);
    let insert = piece(random, eol);
    edited.splice(at, end - at, &insert);
}

fn bytewise(round: usize, old: &str, (new, map): (String, Map)) -> (String, Map) {
    match round % 2 {
        0 => (new, map),
        _ => {
            let map = between(&old, &new.as_str()).unwrap();
            (new, map)
        }
    }
}

fn render(index: usize, lines: &[String], eol: &str) -> String {
    format!("## {index}{eol}") + &lines.iter().map(|line| format!("{line}{eol}")).collect::<String>()
}

fn mutate(random: &mut Lcg, lines: &[String]) -> Vec<String> {
    let mut lines = lines.to_vec();
    let at = random.next(lines.len().max(1) as u64);
    match (random.next(5), lines.is_empty()) {
        (_, true) | (0, _) => lines.insert(random.next(lines.len() as u64 + 1), line(random)),
        (1, _) => drop(lines.remove(at)),
        (2, _) => lines[at] = line(random),
        (3, _) => lines[at] = format!("{} {}", word(random), lines[at]),
        _ => lines[at] = lines[at].split_once(' ').map_or(String::new(), |(_, rest)| rest.to_owned()),
    }
    lines
}

#[test]
fn reverting_the_latest_version_gives_back_exactly_the_previous_text() {
    let mut random = Lcg(20_261_028);
    for round in 0..600 {
        let eol = end_of_line(&mut random);
        let previous: String = (0..random.next(6)).map(|_| piece(&mut random, eol)).collect();
        let mut version = Edited::new(&previous);
        for _ in 0..1 + random.next(4) {
            random_splice(&mut random, &mut version, eol);
        }
        let (version, own) = bytewise(round, &previous, version.done());
        let (text, displaced) = undo(&previous, &own, &version, &Vec::new(), &version);
        assert_eq!(text, previous, "раунд {round}: откат последней версии возвращает ровно прежний текст; версия {version:?}");
        assert!(displaced.is_empty(), "раунд {round}: без поздних правок копия-конфликт не нужна");
    }
}

#[test]
fn untouched_blocks_keep_later_edits_and_reverted_blocks_return() {
    let mut random = Lcg(7_340_033);
    for round in 0..400 {
        let eol = end_of_line(&mut random);
        let before: Vec<Vec<String>> = (0..BLOCKS).map(|_| (0..random.next(4)).map(|_| line(&mut random)).collect()).collect();
        let whole = |blocks: &[Vec<String>]| blocks.iter().enumerate().map(|(index, lines)| render(index, lines, eol)).collect::<String>();
        let previous = whole(&before);
        let touched: Vec<bool> = (0..BLOCKS).map(|index| index == round % BLOCKS || random.next(3) == 0).collect();
        let (mut blocks, mut version) = (before.clone(), Edited::new(&previous));
        for index in (0..BLOCKS).filter(|&index| touched[index]) {
            let next = mutate(&mut random, &blocks[index]);
            version.replace(&render(index, &blocks[index], eol), &render(index, &next, eol));
            blocks[index] = next;
        }
        let (version, own) = bytewise(round, &previous, version.done());
        let mut current = Edited::new(&version);
        for _ in 0..random.next(5) {
            let index = random.next(BLOCKS as u64);
            if !touched[index] {
                let next = mutate(&mut random, &blocks[index]);
                current.replace(&render(index, &blocks[index], eol), &render(index, &next, eol));
                blocks[index] = next;
            }
        }
        let (current, later) = current.done();
        let expected: Vec<Vec<String>> = (0..BLOCKS).map(|index| if touched[index] { before[index].clone() } else { blocks[index].clone() }).collect();
        let (text, displaced) = undo(&previous, &own, &version, &later, &current);
        assert_eq!(text, whole(&expected), "раунд {round}: блоки версии вернулись, поздние правки других блоков остались\nбыло {previous:?}\nверсия {version:?}\nсейчас {current:?}");
        assert!(displaced.is_empty(), "раунд {round}: непересекающиеся правки не дают копию-конфликт");
    }
}

#[test]
fn overlapping_later_edits_never_break_the_text() {
    let mut random = Lcg(1_000_003);
    for round in 0..600 {
        let eol = end_of_line(&mut random);
        let previous: String = (0..random.next(6)).map(|_| piece(&mut random, eol)).collect();
        let mut version = Edited::new(&previous);
        for _ in 0..1 + random.next(3) {
            random_splice(&mut random, &mut version, eol);
        }
        let (version, own) = bytewise(round, &previous, version.done());
        let mut current = Edited::new(&version);
        for _ in 0..random.next(4) {
            random_splice(&mut random, &mut current, eol);
        }
        let (current, later) = bytewise(round + 1, &version, current.done());
        let (text, _) = undo(&previous, &own, &version, &later, &current);
        assert!(!text.contains('\u{FFFD}'), "раунд {round}: отмена поверх пересекающихся правок не режет символы: {text:?}");
    }
}
