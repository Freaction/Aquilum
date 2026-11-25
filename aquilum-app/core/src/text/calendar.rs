pub struct Names {
    pub months: [&'static str; 12],
    pub months_of: [&'static str; 12],
    pub months_short: [&'static str; 12],
    pub months_short_of: [&'static str; 12],
    pub weekdays: [&'static str; 7],
    pub weekdays_short: [&'static str; 7],
    pub weekdays_min: [&'static str; 7],
}

const RU_WEEKDAYS_SHORT: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];

pub const RU: Names = Names {
    months: ["январь", "февраль", "март", "апрель", "май", "июнь", "июль", "август", "сентябрь", "октябрь", "ноябрь", "декабрь"],
    months_of: ["января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября", "октября", "ноября", "декабря"],
    months_short: ["янв.", "февр.", "март", "апр.", "май", "июнь", "июль", "авг.", "сент.", "окт.", "нояб.", "дек."],
    months_short_of: ["янв.", "февр.", "мар.", "апр.", "мая", "июн.", "июл.", "авг.", "сент.", "окт.", "нояб.", "дек."],
    weekdays: ["понедельник", "вторник", "среда", "четверг", "пятница", "суббота", "воскресенье"],
    weekdays_short: RU_WEEKDAYS_SHORT,
    weekdays_min: RU_WEEKDAYS_SHORT,
};

const EN_MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const EN_MONTHS_SHORT: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub const EN: Names = Names {
    months: EN_MONTHS,
    months_of: EN_MONTHS,
    months_short: EN_MONTHS_SHORT,
    months_short_of: EN_MONTHS_SHORT,
    weekdays: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"],
    weekdays_short: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    weekdays_min: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],
};
