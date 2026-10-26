use android_activity::input::{ImeOptions, InputType, InputTypeClass, TextInputAction};
use winit_core::window::{ImeHint, ImePurpose};

pub(super) fn editor_info(hint: ImeHint, purpose: ImePurpose) -> (InputType, TextInputAction, ImeOptions) {
    let mut input_type = match purpose {
        ImePurpose::Number => {
            InputType::TYPE_CLASS_NUMBER
                | InputType::TYPE_NUMBER_FLAG_SIGNED
                | InputType::TYPE_NUMBER_FLAG_DECIMAL
        },
        ImePurpose::Pin => InputType::TYPE_CLASS_NUMBER | InputType::TYPE_NUMBER_VARIATION_PASSWORD,
        ImePurpose::Phone => InputType::TYPE_CLASS_PHONE,
        ImePurpose::Date => InputType::TYPE_CLASS_DATETIME | InputType::TYPE_DATETIME_VARIATION_DATE,
        ImePurpose::Time => InputType::TYPE_CLASS_DATETIME | InputType::TYPE_DATETIME_VARIATION_TIME,
        ImePurpose::DateTime => InputType::TYPE_CLASS_DATETIME,
        ImePurpose::Password => InputType::TYPE_CLASS_TEXT | InputType::TYPE_TEXT_VARIATION_PASSWORD,
        ImePurpose::Url => InputType::TYPE_CLASS_TEXT | InputType::TYPE_TEXT_VARIATION_URI,
        ImePurpose::Email => {
            InputType::TYPE_CLASS_TEXT | InputType::TYPE_TEXT_VARIATION_EMAIL_ADDRESS
        },
        _ => InputType::TYPE_CLASS_TEXT,
    };
    if input_type.class() == InputTypeClass::Text {
        for (wanted, flag) in [
            (ImeHint::MULTILINE, InputType::TYPE_TEXT_FLAG_MULTI_LINE),
            (ImeHint::AUTO_CAPITALIZATION, InputType::TYPE_TEXT_FLAG_CAP_SENTENCES),
            (ImeHint::TITLECASE, InputType::TYPE_TEXT_FLAG_CAP_WORDS),
            (ImeHint::UPPERCASE, InputType::TYPE_TEXT_FLAG_CAP_CHARACTERS),
            (ImeHint::SPELLCHECK, InputType::TYPE_TEXT_FLAG_AUTO_CORRECT),
        ] {
            input_type.set(flag, hint.contains(wanted));
        }
        let suggest = hint.intersects(ImeHint::COMPLETION | ImeHint::SPELLCHECK);
        input_type.set(InputType::TYPE_TEXT_FLAG_NO_SUGGESTIONS, !suggest);
    }
    let multiline = hint.contains(ImeHint::MULTILINE);
    let action = match purpose {
        _ if multiline => TextInputAction::Unspecified,
        ImePurpose::Url => TextInputAction::Go,
        _ => TextInputAction::Done,
    };
    let mut options = ImeOptions::IME_FLAG_NO_FULLSCREEN;
    options.set(ImeOptions::IME_FLAG_NO_ENTER_ACTION, multiline);
    options.set(ImeOptions::IME_FLAG_NO_PERSONALIZED_LEARNING, hint.contains(ImeHint::SENSITIVE_DATA));
    options.set(ImeOptions::IME_FLAG_FORCE_ASCII, hint.contains(ImeHint::LATIN));
    options.set_action(action);
    (input_type, action, options)
}
