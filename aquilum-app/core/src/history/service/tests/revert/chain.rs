use super::Story;
use crate::history::milestone::Source;

#[test]
fn reverting_twice_changes_nothing_the_second_time() {
    let story = Story::new("Дважды.md", "раз\nдва\nтри\n");
    story.agent("раз\nдва\nтри\n");
    story.edit(&[("два\n", "")], Source::Agent);
    story.edit(&[("три", "три и ещё")], Source::Agent);
    story.revert_nth(1);
    let count = story.versions().len();
    story.revert_nth(1);
    assert_eq!(story.text(), "раз\nдва\nтри и ещё\n", "повторная отмена той же версии ничего не меняет");
    assert_eq!(story.versions().len(), count, "повторная отмена не создаёт пустую версию");
}

#[test]
fn reverting_the_revert_brings_the_changes_back() {
    let story = Story::new("Туда-обратно.md", "раз\nдва\nтри\n");
    story.agent("раз\nдва\nтри\n");
    story.edit(&[("раз\n", ""), ("три", "три\nчетыре")], Source::Agent);
    story.edit(&[("два", "ДВА")], Source::Agent);
    story.revert_nth(1);
    assert_eq!(story.text(), "раз\nДВА\nтри\n");
    let last = story.versions().len() - 1;
    story.revert_nth(last);
    assert_eq!(story.text(), "ДВА\nтри\nчетыре\n", "отмена отмены возвращает изменения версии");
}

#[test]
fn a_revert_is_a_new_version_and_keeps_the_history() {
    let story = Story::new("Новая.md", "раз\nдва\n");
    story.agent("раз\nдва\n");
    story.edit(&[("два", "ДВА")], Source::Agent);
    let before = story.versions();
    story.revert_nth(1);
    let after = story.versions();
    assert_eq!(&after[..before.len()], before.as_slice(), "отмена не трогает прежние версии");
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(after[before.len()].source, Source::Revert(before[1].at_ms), "новая версия помнит, какую отменили");
    assert_eq!(story.version_text(&after[before.len()].id), "раз\nдва\n");
}

#[test]
fn a_restore_can_be_reverted() {
    let story = Story::new("Восстановление.md", "первая\n");
    story.agent("первая\n");
    story.agent("первая\nвторая\n");
    story.agent("первая\nвторая\nтретья\n");
    let first = story.versions().remove(0);
    story.save("первая\n", Source::Restore(first.at_ms));
    let last = story.versions().len() - 1;
    story.revert_nth(last);
    assert_eq!(story.text(), "первая\nвторая\nтретья\n", "отмена восстановления возвращает текст до него");
}

#[test]
fn the_oldest_version_and_an_unknown_one_cannot_be_reverted() {
    let story = Story::new("Старая.md", "раз\n");
    story.agent("раз\n");
    story.agent("раз\nдва\n");
    let oldest = story.versions().remove(0);
    assert!(story.revert(&oldest).is_err(), "у самой старой версии нет предыдущей");
    let mut unknown = oldest.clone();
    unknown.id = "1_00000000_me.delta".to_owned();
    assert!(story.revert(&unknown).is_err(), "неизвестная версия даёт ошибку");
    assert_eq!(story.text(), "раз\nдва\n", "заметка не изменилась");
}

#[test]
fn emoji_and_crlf_survive_a_revert_without_hints() {
    let story = Story::new("Эмодзи.md", "жук 🐞 и ёж\r\nзмея 🐍\r\nкит 🐋\r\n");
    story.agent("жук 🐞 и ёж\r\nзмея 🐍\r\nкит 🐋\r\n");
    story.agent("жук 🐝 и ёж\r\nкит 🐋\r\n");
    story.agent("жук 🐝 и ёж\r\nкит 🐋 и 🦎\r\n");
    story.revert_nth(1);
    assert_eq!(story.text(), "жук 🐞 и ёж\r\nзмея 🐍\r\nкит 🐋 и 🦎\r\n", "побайтовые участки без подсказки не режут символы и концы строк");
}
