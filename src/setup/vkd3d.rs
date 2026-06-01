use colored::Colorize;
use std::path::Path;
use crate::TempestError;
use super::dll;

const REPO: &str = "HansKristian-Work/vkd3d-proton";
const DLLS: &[&str] = &["d3d12.dll", "d3d12core.dll"];
const OVERRIDES: &[(&str, &str)] = &[
    ("d3d12",     "native"),
    ("d3d12core", "native"),
];

pub fn is_installed(prefix: &Path) -> bool {
    let d3d12 = prefix.join("drive_c/windows/system32/d3d12.dll");
    dll::verify_dll(&d3d12)
}

pub async fn install(prefix: &Path) -> Result<(), TempestError> {
    println!("{} Installing vkd3d-proton (D3D12 → Vulkan)...", "[INFO]".cyan());

    let client = reqwest::Client::builder()
        .user_agent("tempest/0.1.0")
        .build()?;

    let (version, url) = super::fetch_github_release(&client, REPO, ".tar.zst").await?;
    println!("{} vkd3d-proton {} — downloading...", "[INFO]".cyan(), version);

    let tmp_dir = crate::config::Config::data_dir().join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;
    let archive = tmp_dir.join("vkd3d-proton.tar.zst");

    super::download_file(&client, &url, &archive).await?;
    extract_and_install(&archive, prefix)?;
    std::fs::remove_file(&archive).ok();

    for (name, override_type) in OVERRIDES {
        dll::set_dll_override(prefix, name, override_type);
    }

    set_d3d12_registry_overrides(prefix);

    println!("{} vkd3d-proton installed", "[PASS]".green());
    Ok(())
}

fn extract_and_install(archive: &Path, prefix: &Path) -> Result<(), TempestError> {
    use tar::Archive;

    let extract_dir = crate::config::Config::data_dir().join("tmp").join("vkd3d-extract");
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)?;
    }
    std::fs::create_dir_all(&extract_dir)?;

    let file = std::fs::File::open(archive)?;
    let decoder = zstd::Decoder::new(std::io::BufReader::new(file))
        .map_err(|e| TempestError::Other(e.to_string()))?;
    let mut ar = Archive::new(decoder);
    ar.unpack(&extract_dir)
        .map_err(|e| TempestError::Other(e.to_string()))?;

    let sys32    = prefix.join("drive_c/windows/system32");
    let syswow64 = prefix.join("drive_c/windows/syswow64");
    std::fs::create_dir_all(&sys32)?;
    std::fs::create_dir_all(&syswow64)?;

    for entry in std::fs::read_dir(&extract_dir)? {
        let vkd3d_dir = entry?.path();
        install_arch_dlls(&vkd3d_dir.join("x64"), &sys32)?;
        install_arch_dlls(&vkd3d_dir.join("x86"), &syswow64)?;
    }

    std::fs::remove_dir_all(&extract_dir).ok();
    Ok(())
}

fn install_arch_dlls(src_dir: &Path, dest_dir: &Path) -> Result<(), TempestError> {
    if !src_dir.exists() {
        return Ok(());
    }
    for name in DLLS {
        let src = src_dir.join(name);
        if src.exists() {
            dll::install_dll(&src, &dest_dir.join(name))?;
        }
    }
    Ok(())
}

fn set_d3d12_registry_overrides(prefix: &Path) {
    for (dll_name, override_type) in OVERRIDES {
        std::process::Command::new("wine")
            .env("WINEPREFIX", prefix)
            .env("WINEDEBUG", "-all")
            .args([
                "reg", "add",
                r"HKCU\Software\Wine\DllOverrides",
                "/v", dll_name,
                "/t", "REG_SZ",
                "/d", override_type,
                "/f",
            ])
            .status()
            .ok();
    }
}
