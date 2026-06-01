use colored::Colorize;
use std::path::Path;
use crate::TempestError;

pub fn verify_dll(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    if let Ok(out) = std::process::Command::new("file").arg(path).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            return s.contains("PE32") || s.contains("MS-DOS executable");
        }
    }
    use std::io::Read;
    std::fs::File::open(path)
        .and_then(|mut f| {
            let mut magic = [0u8; 2];
            f.read_exact(&mut magic)?;
            Ok(magic == [0x4D, 0x5A])
        })
        .unwrap_or(false)
}

pub fn backup_dll(path: &Path) -> Result<(), TempestError> {
    if path.exists() {
        let bak = path.with_extension("dll.bak");
        std::fs::copy(path, &bak)?;
        tracing::debug!(
            "Backed up {} -> {}",
            path.display(),
            bak.display()
        );
    }
    Ok(())
}

pub fn install_dll(src: &Path, dest: &Path) -> Result<(), TempestError> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if !verify_dll(src) {
        return Err(TempestError::Other(format!(
            "{} does not appear to be a valid PE binary",
            src.display()
        )));
    }

    if dest.exists() && !verify_dll(dest) {
        println!(
            "{} {} is not a valid PE binary — replacing anyway",
            "[WARN]".yellow(),
            dest.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    backup_dll(dest)?;
    std::fs::copy(src, dest)?;
    println!(
        "{} {}",
        "[PASS]".green(),
        dest.file_name().unwrap_or_default().to_string_lossy()
    );
    Ok(())
}

pub fn install_dlls_from(src_dir: &Path, dest_dir: &Path, dlls: &[&str]) -> Result<(), TempestError> {
    if !src_dir.exists() {
        return Ok(());
    }
    for name in dlls {
        let src = src_dir.join(name);
        if src.exists() {
            install_dll(&src, &dest_dir.join(name))?;
        }
    }
    Ok(())
}

pub fn set_dll_override(prefix: &Path, dll_name: &str, override_type: &str) -> bool {
    let ok = std::process::Command::new("wine")
        .env("WINEPREFIX", prefix)
        .env("WINEDEBUG", "-all")
        .args([
            "reg",
            "add",
            r"HKCU\Software\Wine\DllOverrides",
            "/v",
            dll_name,
            "/t",
            "REG_SZ",
            "/d",
            override_type,
            "/f",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        tracing::debug!("DLL override: {} = {}", dll_name, override_type);
    }
    ok
}
