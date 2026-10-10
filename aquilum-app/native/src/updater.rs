use std::io::Read;
use std::sync::{Arc, Mutex};

use base64::Engine;

const ENDPOINT: &str = "https://github.com/Freaction/Aquilum/releases/latest/download/latest.json";
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEI4QTY0MzM1MDczQUNDMTQKUldRVXpEb0hOVU9tdUFCWnVMRWlBVTRGWmlFOW0wd2VteVlXd3luMFAyMmczWktQbkRvVmhTQSsK";
const CHUNK: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    Checking,
    Latest,
    Available(String),
    Downloading { version: String, done: u64, total: Option<u64> },
    Ready(String),
    Installing,
    Failed(String),
}

#[derive(Clone)]
struct Release {
    version: String,
    url: String,
    signature: String,
}

#[derive(Clone)]
pub struct Updater {
    status: Arc<Mutex<Status>>,
    payload: Arc<Mutex<Option<Vec<u8>>>>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

pub fn available() -> bool {
    cfg!(feature = "installed")
}

fn target() -> Option<String> {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return None;
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        return None;
    };
    Some(format!("{os}-{arch}"))
}

fn version(text: &str) -> Vec<u64> {
    text.trim().trim_start_matches('v').split(['.', '-', '+']).map_while(|part| part.parse().ok()).collect()
}

pub fn newer(candidate: &str, current: &str) -> bool {
    version(candidate) > version(current)
}

fn find() -> Result<Option<Release>, String> {
    let agent = aquilum_core::web_agent::web_agent("updates");
    let manifest: serde_json::Value = serde_json::from_reader(agent.get(ENDPOINT).call().map_err(|e| e.to_string())?.into_reader()).map_err(|e| e.to_string())?;
    let version = manifest["version"].as_str().ok_or("в latest.json нет версии")?.to_owned();
    if !newer(&version, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    let target = target().ok_or("платформа не поддерживается")?;
    let platform = &manifest["platforms"][&target];
    let url = platform["url"].as_str().ok_or_else(|| format!("в latest.json нет сборки для {target}"))?.to_owned();
    let signature = platform["signature"].as_str().ok_or("в latest.json нет подписи")?.to_owned();
    Ok(Some(Release { version, url, signature }))
}

fn decoded(text: &str) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(text.trim()).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

pub fn verify(data: &[u8], signature: &str, key: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::decode(&decoded(key)?).map_err(|e| e.to_string())?;
    let signature = minisign_verify::Signature::decode(&decoded(signature)?).map_err(|e| e.to_string())?;
    key.verify(data, &signature, true).map_err(|e| e.to_string())
}

impl Updater {
    pub fn new(wake: Arc<dyn Fn() + Send + Sync>) -> Updater {
        Updater { status: Arc::new(Mutex::new(Status::Idle)), payload: Arc::new(Mutex::new(None)), wake }
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set(&self, status: Status) {
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
        (self.wake)();
    }

    fn busy(&self) -> bool {
        matches!(self.status(), Status::Checking | Status::Downloading { .. } | Status::Ready(_) | Status::Installing)
    }

    pub fn check(&self) {
        if !available() || self.busy() {
            return;
        }
        self.set(Status::Checking);
        let this = self.clone();
        std::thread::spawn(move || {
            this.set(match find() {
                Ok(Some(release)) => Status::Available(release.version),
                Ok(None) => Status::Latest,
                Err(error) => Status::Failed(error),
            });
        });
    }

    pub fn install(&self) {
        if !available() || self.busy() {
            return;
        }
        self.set(Status::Checking);
        let this = self.clone();
        std::thread::spawn(move || {
            let result = find().and_then(|release| match release {
                Some(release) => this.download(&release).map(|()| Some(release.version)),
                None => Ok(None),
            });
            this.set(match result {
                Ok(Some(version)) => Status::Ready(version),
                Ok(None) => Status::Latest,
                Err(error) => Status::Failed(error),
            });
        });
    }

    fn download(&self, release: &Release) -> Result<(), String> {
        let agent = aquilum_core::web_agent::web_agent("updates");
        let response = agent.get(&release.url).call().map_err(|e| e.to_string())?;
        let total = response.header("Content-Length").and_then(|v| v.parse().ok());
        let mut reader = response.into_reader();
        let mut data = Vec::with_capacity(total.unwrap_or(0) as usize);
        let mut buffer = vec![0u8; CHUNK];
        loop {
            let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..read]);
            self.set(Status::Downloading { version: release.version.clone(), done: data.len() as u64, total });
        }
        verify(&data, &release.signature, PUBLIC_KEY)?;
        *self.payload.lock().unwrap_or_else(|e| e.into_inner()) = Some(data);
        Ok(())
    }

    pub fn take_ready(&self) -> Option<Vec<u8>> {
        if !matches!(self.status(), Status::Ready(_)) {
            return None;
        }
        let payload = self.payload.lock().unwrap_or_else(|e| e.into_inner()).take()?;
        self.set(Status::Installing);
        Some(payload)
    }
}

#[cfg(target_os = "windows")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
    use windows::core::{HSTRING, w};
    let dir = std::env::temp_dir().join(format!("aquilum-update-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let installer = dir.join("Aquilum-setup.exe");
    std::fs::write(&installer, payload).map_err(|e| e.to_string())?;
    before_exit();
    unsafe {
        ShellExecuteW(None, w!("open"), &HSTRING::from(installer.as_os_str()), &HSTRING::from("/S /R /UPDATE /ARGS"), None, SW_SHOW);
    }
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let text = exe.to_string_lossy().into_owned();
    let (bundle, _) = text.split_once(".app/Contents/MacOS/").ok_or("приложение запущено не из пакета .app")?;
    let bundle = std::path::PathBuf::from(format!("{bundle}.app"));
    let parent = bundle.parent().ok_or("нет каталога пакета")?;
    let staging = parent.join(format!(".aquilum-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let gz = flate2::read::GzDecoder::new(payload);
    tar::unpack(gz, &staging)?;
    let fresh = std::fs::read_dir(&staging).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "app")).ok_or("в обновлении нет пакета .app")?;
    before_exit();
    let backup = parent.join(format!(".aquilum-old-{}", std::process::id()));
    std::fs::rename(&bundle, &backup).map_err(|e| e.to_string())?;
    if let Err(error) = std::fs::rename(&fresh, &bundle) {
        let _ = std::fs::rename(&backup, &bundle);
        return Err(error.to_string());
    }
    let _ = std::fs::remove_dir_all(&backup);
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::process::Command::new("open").arg("-n").arg(&bundle).spawn();
    std::process::exit(0);
}

#[cfg(target_os = "linux")]
pub fn apply(payload: &[u8], before_exit: impl FnOnce()) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let image = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from).ok_or("обновление поддерживается только для AppImage")?;
    let fresh = image.with_extension("update");
    std::fs::write(&fresh, payload).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    before_exit();
    std::fs::rename(&fresh, &image).map_err(|e| e.to_string())?;
    let _ = std::process::Command::new(&image).spawn();
    std::process::exit(0);
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn apply(_payload: &[u8], _before_exit: impl FnOnce()) -> Result<(), String> {
    Err("платформа не поддерживается".into())
}

#[cfg(target_os = "macos")]
mod tar {
    use std::io::Read;
    use std::path::{Component, Path, PathBuf};

    fn field(header: &[u8], from: usize, len: usize) -> String {
        let raw = &header[from..from + len];
        let end = raw.iter().position(|&b| b == 0).unwrap_or(len);
        String::from_utf8_lossy(&raw[..end]).into_owned()
    }

    fn octal(header: &[u8], from: usize, len: usize) -> u64 {
        u64::from_str_radix(field(header, from, len).trim(), 8).unwrap_or(0)
    }

    fn safe(root: &Path, name: &str) -> Option<PathBuf> {
        let relative = Path::new(name);
        relative.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)).then(|| root.join(relative))
    }

    pub fn unpack(mut reader: impl Read, root: &Path) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        let mut long: Option<String> = None;
        loop {
            let mut header = [0u8; 512];
            if reader.read_exact(&mut header).is_err() || header.iter().all(|&b| b == 0) {
                return Ok(());
            }
            let size = octal(&header, 124, 12);
            let mut data = vec![0u8; size as usize];
            reader.read_exact(&mut data).map_err(|e| e.to_string())?;
            let pad = (512 - size % 512) % 512;
            std::io::copy(&mut (&mut reader).take(pad), &mut std::io::sink()).map_err(|e| e.to_string())?;
            let kind = header[156];
            if kind == b'L' {
                long = Some(String::from_utf8_lossy(&data).trim_end_matches('\0').to_owned());
                continue;
            }
            if kind == b'x' {
                let pax = String::from_utf8_lossy(&data).into_owned();
                long = pax.lines().find_map(|l| l.split_once(" path=").map(|(_, p)| p.to_owned()));
                continue;
            }
            let prefix = field(&header, 345, 155);
            let name = long.take().unwrap_or_else(|| if prefix.is_empty() { field(&header, 0, 100) } else { format!("{prefix}/{}", field(&header, 0, 100)) });
            let Some(path) = safe(root, &name) else { continue };
            match kind {
                b'5' => std::fs::create_dir_all(&path).map_err(|e| e.to_string())?,
                b'2' => {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    std::os::unix::fs::symlink(field(&header, 157, 100), &path).map_err(|e| e.to_string())?;
                }
                b'0' | 0 => {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
                    let mode = octal(&header, 100, 8) as u32;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode | 0o600)).map_err(|e| e.to_string())?;
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions_compare_numerically() {
        assert!(super::newer("0.2.10", "0.2.9"));
        assert!(super::newer("v0.3.0", "0.2.6"));
        assert!(!super::newer("0.2.6", "0.2.6"));
        assert!(!super::newer("0.2.5", "0.2.6"));
    }

    #[test]
    #[ignore]
    fn verifies_real_release_signature() {
        let manifest: serde_json::Value = serde_json::from_reader(aquilum_core::web_agent::web_agent("updates").get(super::ENDPOINT).call().unwrap().into_reader()).unwrap();
        let platform = &manifest["platforms"]["windows-x86_64"];
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut aquilum_core::web_agent::web_agent("updates").get(platform["url"].as_str().unwrap()).call().unwrap().into_reader(), &mut data).unwrap();
        super::verify(&data, platform["signature"].as_str().unwrap(), super::PUBLIC_KEY).unwrap();
        data[100] ^= 1;
        assert!(super::verify(&data, platform["signature"].as_str().unwrap(), super::PUBLIC_KEY).is_err());
    }
}
