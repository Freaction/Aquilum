use std::path::PathBuf;
#[cfg(not(target_os = "android"))]
use std::process::Command;

pub fn appimage() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE").filter(|_| cfg!(target_os = "linux")).map(PathBuf::from)
}

pub fn app_path() -> Option<PathBuf> {
    if let Some(image) = appimage() {
        return Some(image);
    }
    let exe = std::env::current_exe().ok()?;
    if !cfg!(target_os = "macos") {
        return Some(exe);
    }
    let exe = exe.canonicalize().unwrap_or(exe);
    let bundle = exe.to_string_lossy().split_once(".app/Contents/MacOS/").map(|(bundle, _)| PathBuf::from(format!("{bundle}.app")));
    Some(bundle.unwrap_or(exe))
}

#[cfg(not(target_os = "android"))]
pub fn restart() {
    let Some(app) = app_path() else { return };
    let command = if app.extension().is_some_and(|extension| extension == "app") {
        let mut command = Command::new("open");
        command.arg("-n").arg(&app);
        command
    } else {
        Command::new(&app)
    };
    launch(command, &app.display().to_string());
}

#[cfg(target_os = "android")]
pub fn restart() {}

#[cfg(not(target_os = "android"))]
pub fn launch(mut command: Command, what: &str) {
    if let (Ok(appdir), Some(_)) = (std::env::var("APPDIR"), appimage()) {
        let vars = std::env::vars_os().filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)));
        for (key, value) in outside_appimage(vars, &appdir) {
            match value {
                Some(value) => command.env(key, value),
                None => command.env_remove(key),
            };
        }
    }
    let what = what.to_owned();
    match command.spawn() {
        Ok(mut child) => {
            std::thread::spawn(move || match child.wait() {
                Ok(status) if !status.success() => report!("{what} не открыт: {status}"),
                Err(error) => report!("{what} не открыт: {error}"),
                Ok(_) => {}
            });
        }
        Err(error) => report!("{what} не открыт: {error}"),
    }
}

#[cfg(not(target_os = "android"))]
fn outside_appimage(vars: impl Iterator<Item = (String, String)>, appdir: &str) -> Vec<(String, Option<String>)> {
    let inside = |entry: &str| std::path::Path::new(entry).starts_with(appdir);
    vars.filter(|(_, value)| value.split(':').any(inside))
        .map(|(key, value)| {
            let kept: Vec<&str> = value.split(':').filter(|entry| !entry.is_empty() && !inside(entry)).collect();
            let kept = (!kept.is_empty()).then(|| kept.join(":"));
            (key, kept)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn programs_opened_from_an_appimage_get_the_users_environment_back() {
        let appdir = "/tmp/.mount_AquilaX";
        let vars = [
            ("LD_LIBRARY_PATH", "/tmp/.mount_AquilaX/usr/lib/:/tmp/.mount_AquilaX/lib64/:/opt/lib"),
            ("PATH", "/tmp/.mount_AquilaX/usr/bin/:/usr/local/bin:/usr/bin"),
            ("PYTHONHOME", "/tmp/.mount_AquilaX/usr/"),
            ("QT_PLUGIN_PATH", "/tmp/.mount_AquilaX/usr/lib/qt5/plugins/:"),
            ("APPDIR", "/tmp/.mount_AquilaX"),
            ("HOME", "/home/user"),
            ("LS_COLORS", "di=01;34:ln=01;36"),
            ("OTHER", "/tmp/.mount_AquilaXY/usr"),
        ]
        .map(|(key, value)| (key.to_owned(), value.to_owned()));
        let changes = super::outside_appimage(vars.into_iter(), appdir);
        let changed = |key: &str| changes.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_deref());
        assert_eq!(changed("LD_LIBRARY_PATH"), Some(Some("/opt/lib")));
        assert_eq!(changed("PATH"), Some(Some("/usr/local/bin:/usr/bin")));
        assert_eq!(changed("PYTHONHOME"), Some(None));
        assert_eq!(changed("QT_PLUGIN_PATH"), Some(None), "пустой хвост — не текущая папка");
        assert_eq!(changed("APPDIR"), Some(None));
        assert_eq!(changes.len(), 5, "остальное окружение не трогается: {changes:?}");
    }
}
