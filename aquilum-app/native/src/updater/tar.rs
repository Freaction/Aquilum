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

pub(super) fn unpack(mut reader: impl Read, root: &Path) -> Result<(), String> {
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
