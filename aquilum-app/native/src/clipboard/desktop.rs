use std::sync::{Mutex, TryLockError};

static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

fn access<T>(work: impl FnOnce(&mut arboard::Clipboard) -> Option<T>) -> Option<T> {
    let mut guard = match CLIPBOARD.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(TryLockError::WouldBlock) => return None,
    };
    if guard.is_none() {
        *guard = arboard::Clipboard::new().map_err(|error| report!("буфер обмена недоступен: {error}")).ok();
    }
    work(guard.as_mut()?)
}

fn with<T>(work: impl FnOnce(&mut arboard::Clipboard) -> Result<T, arboard::Error>) -> Option<T> {
    access(|clipboard| match work(clipboard) {
        Ok(value) => Some(value),
        Err(arboard::Error::ContentNotAvailable) => None,
        Err(error) => {
            report!("буфер обмена: {error}");
            None
        }
    })
}

pub fn copy(text: impl Into<String>) -> bool {
    with(|clipboard| clipboard.set_text(text.into())).is_some()
}

pub fn text() -> Option<String> {
    with(arboard::Clipboard::get_text)
}

pub fn html() -> Option<String> {
    access(|clipboard| clipboard.get().html().ok())
}

pub fn png() -> Option<Vec<u8>> {
    let image = with(arboard::Clipboard::get_image)?;
    let rgba = image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.bytes.into_owned())?;
    let mut out = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

pub fn release() {
    if let Ok(mut guard) = CLIPBOARD.lock() {
        guard.take();
    }
}
