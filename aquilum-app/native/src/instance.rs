use std::fs::{File, TryLockError};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const PORT_FILE: &str = "instance.port";
const GREETING: &[u8] = b"aquilum:show";

static CALLED: AtomicBool = AtomicBool::new(false);

pub enum Instance {
    First(Option<File>),
    Running,
}

pub fn acquire(data_dir: &Path, wake: impl Fn() + Send + 'static) -> Instance {
    let file = std::fs::create_dir_all(data_dir).and_then(|()| File::options().create(true).write(true).truncate(false).open(data_dir.join("instance.lock")));
    let Ok(file) = file else { return Instance::First(None) };
    match file.try_lock() {
        Ok(()) => {
            if crate::system::DESKTOP {
                listen(&data_dir.join(PORT_FILE), wake);
            }
            Instance::First(Some(file))
        }
        Err(TryLockError::WouldBlock) => {
            call(&data_dir.join(PORT_FILE));
            Instance::Running
        }
        Err(TryLockError::Error(_)) => Instance::First(None),
    }
}

pub fn called() -> bool {
    CALLED.swap(false, Ordering::Relaxed)
}

fn listen(port_file: &Path, wake: impl Fn() + Send + 'static) {
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else { return };
    let Ok(address) = listener.local_addr() else { return };
    if std::fs::write(port_file, address.port().to_string()).is_err() {
        return;
    }
    let _ = aquilum_core::threads::background("второй запуск", move || {
        for mut stream in listener.incoming().map_while(Result::ok) {
            let mut greeting = [0; GREETING.len()];
            let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
            if stream.read_exact(&mut greeting).is_ok() && greeting == GREETING {
                CALLED.store(true, Ordering::Relaxed);
                wake();
            }
        }
    });
}

fn call(port_file: &Path) {
    let Some(port) = std::fs::read_to_string(port_file).ok().and_then(|text| text.parse::<u16>().ok()) else { return };
    #[cfg(target_os = "windows")]
    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(windows::Win32::UI::WindowsAndMessaging::ASFW_ANY) };
    if let Ok(mut stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port)) {
        let _ = stream.write_all(GREETING);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_launch_wakes_the_first_and_a_stray_connection_does_not() {
        let dir = crate::test_dir::tempdir();
        let (woken, wakes) = std::sync::mpsc::channel();
        let first = acquire(dir.path(), move || {
            let _ = woken.send(());
        });
        assert!(matches!(first, Instance::First(Some(_))));
        let port: u16 = std::fs::read_to_string(dir.path().join(PORT_FILE)).unwrap().parse().unwrap();
        drop(TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap());
        assert!(matches!(acquire(dir.path(), || {}), Instance::Running));
        wakes.recv_timeout(Duration::from_secs(5)).expect("первый экземпляр не разбужен");
        assert!(called());
        assert!(!called(), "сигнал снимается один раз");
        assert!(wakes.try_recv().is_err(), "пустое подключение разбудило окно");
        drop(first);
        assert!(matches!(acquire(dir.path(), || {}), Instance::First(Some(_))), "после выхода первого блокировка свободна");
    }
}
