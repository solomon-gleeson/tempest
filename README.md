# Tempest

A native Linux launcher for [Vortex](https://vortex.towerstats.com), written in Rust.

Tempest handles Wine setup, authentication, URI scheme registration, and game launching — so you can click Play on the Vortex website and have it Just Work on Linux.

## Install

```bash
curl -fsSL https://github.com/yourusername/tempest/releases/latest/download/install.sh | bash
tempest setup
```

## What it does

- Detects your distro and installs Wine + winetricks with the right packages
- Creates a dedicated Wine prefix (`~/.local/share/tempest/prefix`) with D3D compiler, VC++ runtime, and core fonts
- Registers the `vortex://` URI scheme so browser Play buttons work
- Handles authentication via browser cookie — no password stored
- Filters Wine's verbose log noise so you see relevant output only
- Downloads and updates `Vortex.exe` with a progress bar
- Diagnoses your full stack (Vulkan, GPU, Wine, network) with per-distro fix hints

## Usage

```
tempest setup        # First-run: install Wine, create prefix, download client
tempest login        # Authenticate with Vortex (opens browser)
tempest play <id>    # Launch a game by ID
tempest update       # Update Vortex.exe to latest version
tempest doctor       # Diagnose issues
tempest uninstall    # Remove everything
```

The `vortex://` URI handler is registered during setup, so clicking Play on the website calls `tempest uri-handler <uri>` automatically.

## Supported distros

| Distro | Status |
|--------|--------|
| Fedora / RHEL / CentOS Stream | Supported |
| Debian / Ubuntu / Mint / Pop!_OS | Supported |
| Arch / Manjaro / EndeavourOS | Supported |
| openSUSE | Supported |

## Requirements

- Wine 7+ (installed by `tempest setup`)
- Vulkan-capable GPU (NVIDIA, AMD, or Intel)
- `xdg-utils` for URI scheme registration

## Configuration

Config lives at `~/.config/tempest/config.toml`. You can set custom Wine environment variables there:

```toml
[wine]
binary = "wine"

[wine.env]
DXVK_HUD = "fps"
VK_ICD_FILENAMES = "/usr/share/vulkan/icd.d/nvidia_icd.x86_64.json"

[launcher]
filter_wine_noise = true
auto_update = true
```

## Debug logging

```bash
TEMPEST_LOG=debug tempest play 4
```

## Known issues

- Wayland: Wine runs under XWayland. Native Wayland support depends on your Wine version.
- NVIDIA on Optimus laptops: you may need to set `VK_ICD_FILENAMES` in `[wine.env]` to force the discrete GPU.
- First launch after `wineboot` can be slow (30–60 s) while Wine initialises the prefix.

## Contributing

Bug reports and PRs welcome. Run `cargo test` before submitting.
