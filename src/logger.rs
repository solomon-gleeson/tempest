use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

static LOGGER: LazyLock<Mutex<Option<LogFile>>> =
    LazyLock::new(|| Mutex::new(None));

struct LogFile {
    path: PathBuf,
}

impl LogFile {
    fn append(&self, msg: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            writeln!(file, "{}", msg).ok();
        }
    }
}

fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();

    let days = secs / 86400;
    let time = secs % 86400;
    let hours = time / 3600;
    let mins = (time % 3600) / 60;
    let sec = time % 60;

    let y = 1970 + days / 365;
    let rem = days % 365;
    let leap = (y - 1969) / 4 - (y - 1901) / 100 + (y - 1601) / 400;
    let doy = rem.saturating_sub(leap);

    let (mon, day) = ymd_from_doy(doy);
    format!("{y:04}-{mon:02}-{day:02} {hours:02}:{mins:02}:{sec:02}")
}

fn ymd_from_doy(doy: u64) -> (u64, u64) {
    let dim = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut left = doy;
    for (i, &d) in dim.iter().enumerate() {
        if left < d {
            return (i as u64 + 1, left + 1);
        }
        left -= d;
    }
    (12, 31)
}

pub fn init(path: PathBuf) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut guard = LOGGER.lock().expect("logger lock");
    *guard = Some(LogFile { path });
}

pub fn error(msg: &str) {
    let entry = format!("[{}] [ERROR] {}", timestamp(), msg);
    eprintln!("{}", entry);
    let guard = LOGGER.lock().expect("logger lock");
    if let Some(log) = guard.as_ref() {
        log.append(&entry);
    }
}

#[allow(dead_code)]
pub fn warn(msg: &str) {
    let entry = format!("[{}] [WARN] {}", timestamp(), msg);
    eprintln!("{}", entry);
    let guard = LOGGER.lock().expect("logger lock");
    if let Some(log) = guard.as_ref() {
        log.append(&entry);
    }
}

#[allow(dead_code)]
pub fn info(msg: &str) {
    let entry = format!("[{}] [INFO] {}", timestamp(), msg);
    let guard = LOGGER.lock().expect("logger lock");
    if let Some(log) = guard.as_ref() {
        log.append(&entry);
    }
}
