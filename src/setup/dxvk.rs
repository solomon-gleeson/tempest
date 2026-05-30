use colored::Colorize;
use std::path::Path;
use crate::TempestError;
use super::dll;

const REPO: &str = "doitsujin/dxvk";
const DLLS: &[&str] = &["d3d9.dll", "d3d10core.dll", "d3d10_1.dll", "d3d11.dll", "dxgi.dll"];
const OVERRIDES: &[(&str, &str)] = &[
    ("d3d9",     "native,builtin"),
    ("d3d10core","native,builtin"),
    ("d3d10_1",  "native,builtin"),
    ("d3d11",    "native,builtin"),
    ("dxgi",     "native,builtin"),
];

pub fn is_installed(prefix: &Path) -> bool {
    let dxgi = prefix.join("drive_c/windows/system32/dxgi.dll");
    dll::verify_dll(&dxgi)
}

pub async fn install(prefix: &Path) -> Result<(), TempestError> {
    println!("{} Installing DXVK (D3D9/10/11 → Vulkan)...", "[INFO]".cyan());

    let client = reqwest::Client::builder()
        .user_agent("tempest/0.1.0")
        .build()?;

    let (version, url) = super::fetch_github_release(&client, REPO, ".tar.gz").await?;
    println!("{} DXVK {} — downloading...", "[INFO]".cyan(), version);

    let tmp_dir = crate::config::Config::data_dir().join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;
    let archive = tmp_dir.join("dxvk.tar.gz");

    super::download_file(&client, &url, &archive).await?;
    extract_and_install(&archive, prefix)?;
    std::fs::remove_file(&archive).ok();

    for (name, override_type) in OVERRIDES {
        dll::set_dll_override(prefix, name, override_type);
    }

    println!("{} DXVK installed", "[PASS]".green());
    Ok(())
}

fn extract_and_install(archive: &Path, prefix: &Path) -> Result<(), TempestError> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let extract_dir = crate::config::Config::data_dir().join("tmp").join("dxvk-extract");
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)?;
    }
    std::fs::create_dir_all(&extract_dir)?;

    let file = std::fs::File::open(archive)?;
    let gz = GzDecoder::new(std::io::BufReader::new(file));
    let mut ar = Archive::new(gz);
    ar.unpack(&extract_dir)
        .map_err(|e| TempestError::Other(e.to_string()))?;

    let sys32   = prefix.join("drive_c/windows/system32");
    let syswow64 = prefix.join("drive_c/windows/syswow64");
    std::fs::create_dir_all(&sys32)?;
    std::fs::create_dir_all(&syswow64)?;

    for entry in std::fs::read_dir(&extract_dir)? {
        let dxvk_dir = entry?.path();
        install_arch_dlls(&dxvk_dir.join("x64"), &sys32)?;
        install_arch_dlls(&dxvk_dir.join("x32"), &syswow64)?;
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
