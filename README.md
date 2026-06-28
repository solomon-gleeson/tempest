<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/banner.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/banner-light.png">
    <img src="assets/banner.png" alt="Tempest" width="600">
  </picture>
</p>

<p align="center">
  A native Linux launcher for <a href="https://playvortex.io">Vortex</a>, written in Rust.
</p>

---

Tempest is a community-built command-line tool that handles Wine configuration, authentication, URI scheme registration, and game launching.

**What it is:** A launcher wrapper that bridges the Linux desktop and the Windows Vortex client via Wine.

**What it is not:** An official product. Tempest is not affiliated with, endorsed by, or supported by the Vortex team or playvortex.io. It does not modify, redistribute, or replicate any part of the Vortex client.

---

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/solomon-gleeson/tempest/master/install.sh | bash
tempest setup
```

`setup` installs Wine, creates a dedicated Wine prefix, installs DXVK and vkd3d-proton, downloads Vortex, and registers the `vortex://` URI scheme.

New to Linux or hitting driver errors? Paste [docs/setup-agent-prompt.md](docs/setup-agent-prompt.md) into your AI agent and it will detect your distro and GPU, install the right Vulkan drivers, and configure Tempest for you.

---

## Commands

```
tempest setup              First-run: install Wine, create prefix, download client
tempest login              Authenticate with Vortex (opens browser)
tempest list               List all available games
tempest play <id>          Launch a game by ID
tempest update             Update Vortex.exe to latest version
tempest doctor             Diagnose issues across the full stack
tempest plugin             List installed plugins
tempest plugin <name>      Install a plugin (e.g. fps-unlocker)
tempest plugin uninstall   Remove a plugin
tempest uninstall          Remove everything Tempest installed
```

After setup, clicking Play on the Vortex website triggers `tempest uri-handler` automatically via the registered `vortex://` scheme.

---

## Supported distributions

| Distribution | Package manager |
|---|---|
| Fedora, RHEL, CentOS Stream | dnf |
| Debian, Ubuntu, Mint, Pop!_OS | apt |
| Arch, Manjaro, EndeavourOS | pacman |
| openSUSE | zypper |

---

## Configuration

`~/.config/tempest/config.toml` is created on first run. Notable options:

```toml
[wine]
binary = "wine"

[wine.env]
# Force the discrete GPU on Optimus laptops
VK_ICD_FILENAMES = "/usr/share/vulkan/icd.d/nvidia_icd.x86_64.json"
# Show an FPS overlay
DXVK_HUD = "fps"

[launcher]
filter_wine_noise = true   # suppress Wine fixme: and libEGL noise
use_esync = true            # reduce synchronisation overhead (all kernels)
use_fsync = true            # lower overhead (Linux 5.16+ / wine-staging)
use_gamemode = false        # set true after: sudo dnf install gamemode
shader_cache = true         # cache vkd3d-proton shaders across launches
```

---

## Diagnostics

```bash
tempest doctor
```

Checks Wine, Vulkan, GPU, DXVK, vkd3d-proton, GameMode, the URI handler, network connectivity, and receiver.exe, with per-distro fix hints for every failure.

```bash
TEMPEST_LOG=debug tempest play 4
```

---

## Contributing

Bug reports and pull requests are welcome. Run `cargo test` before submitting.

---

## Disclaimer

Tempest is an independent, community-developed tool and is not affiliated with, endorsed by, or in any way connected to the developers or operators of Vortex or playvortex.io. All trademarks and service marks are the property of their respective owners. Use of this tool is at your own risk.

---

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
