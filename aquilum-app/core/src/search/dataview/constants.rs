pub const NANOS_PER_SECOND: i64 = 1_000_000_000;
pub const NANOS_PER_MINUTE: i64 = 60 * NANOS_PER_SECOND;
pub const NANOS_PER_HOUR: i64 = 60 * NANOS_PER_MINUTE;
pub const NANOS_PER_DAY: i64 = 24 * NANOS_PER_HOUR;
pub const NANOS_PER_MONTH: i64 = 2_629_746 * NANOS_PER_SECOND;
const NANOS_PER_YEAR: i64 = 12 * NANOS_PER_MONTH;

pub const DURATION_FINISHED: &str = "конец";
pub const DURATION_ZERO_SECONDS: &str = "0 сек.";

pub const DURATION_UNITS: [(i64, [&str; 3]); 6] = [
    (NANOS_PER_YEAR, ["год", "года", "лет"]),
    (NANOS_PER_MONTH, ["мес.", "мес.", "мес."]),
    (NANOS_PER_DAY, ["день", "дня", "дней"]),
    (NANOS_PER_HOUR, ["час", "часа", "часов"]),
    (NANOS_PER_MINUTE, ["мин.", "мин.", "мин."]),
    (NANOS_PER_SECOND, ["сек.", "сек.", "сек."]),
];

pub const OWN_SECONDS: u32 = 1;
pub const OVER_NOTES_SECONDS: u32 = 60;
pub const MAX_OUTPUT_ROWS: usize = 500;

pub const NOTE_COLUMN: &str = "Заметка";
pub const ROWS_IDENTIFIER: &str = "rows";

pub const TASK_TEXT: &str = "text";
pub const TASK_DONE: &str = "completed";
pub const TASK_LINE: &str = "line";

pub const DEFAULT_DASH: &str = "—";
pub const DEFAULT_ZERO_DELTA: &str = "0";

pub const SPAN_START: &str = "start";
pub const SPAN_END: &str = "end";
pub const FILE_IDENTIFIER: &str = "file";
