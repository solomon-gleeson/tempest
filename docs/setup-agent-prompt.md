# Tempest setup agent prompt

You are setting up **Tempest**, a Linux launcher for the game **Vortex**, on THIS machine.
Tempest wraps Wine, DXVK and vkd3d-proton. Vortex is a Bevy/wgpu game, so a working
**hardware Vulkan driver is mandatory**. Missing drivers cause most failures.

Work in phases. After each phase, show me the output and a one-line verdict before moving on.
Before any `sudo` command, print it with a one-sentence reason, then run it. Do not run
destructive commands, do not `curl | bash` anything except the official installer below, and do
not touch files outside my home directory and the package manager. If a step fails, stop and use
the troubleshooting table before continuing.

## Phase 0: gather facts (no changes)

```bash
cat /etc/os-release
lspci -nn | grep -Ei 'vga|3d|display'
uname -r
command -v wine winetricks vulkaninfo glxinfo
vulkaninfo --summary 2>&1 | head -n 40 || echo "vulkaninfo not installed yet"
```

State back to me:
- **Distro family**: Fedora/RHEL, Debian/Ubuntu/Mint/Pop, Arch/Manjaro, openSUSE, or other.
- **GPU vendor**: Intel, AMD or NVIDIA. Note if it is a hybrid/Optimus laptop or a Steam Deck.

## Phase 1: install Vulkan drivers

Pick the block matching the distro and GPU. Always include the 32-bit (multilib/i386) packages;
Wine needs them. A reboot or full re-login is required afterwards before the driver loads.

### Fedora / RHEL / CentOS (dnf)
```bash
sudo dnf upgrade --refresh
sudo dnf install vulkan-tools vulkan-loader vulkan-loader.i686
# AMD or Intel:
sudo dnf install mesa-vulkan-drivers mesa-vulkan-drivers.i686
# Intel also:
sudo dnf install intel-media-driver
# NVIDIA (needs RPM Fusion):
sudo dnf install akmod-nvidia xorg-x11-drv-nvidia-cuda xorg-x11-drv-nvidia-libs.i686
```

### Debian / Ubuntu / Mint / Pop!_OS (apt)
```bash
sudo dpkg --add-architecture i386
sudo apt update
sudo apt install vulkan-tools
# AMD or Intel:
sudo apt install mesa-vulkan-drivers mesa-vulkan-drivers:i386
# Intel also:
sudo apt install intel-media-va-driver
# NVIDIA:
sudo ubuntu-drivers autoinstall   # or: sudo apt install nvidia-driver
```
Mint or older Ubuntu only, when drivers are too old (`vkd3d result -5`, software renderer):
```bash
sudo add-apt-repository ppa:kisak/kisak-mesa
sudo apt update && sudo apt upgrade
```

### Arch / Manjaro / EndeavourOS (pacman)
Enable the `[multilib]` repo in `/etc/pacman.conf`, run `sudo pacman -Syu`, then:
```bash
sudo pacman -S vulkan-icd-loader lib32-vulkan-icd-loader vulkan-tools mesa lib32-mesa
# AMD:
sudo pacman -S vulkan-radeon lib32-vulkan-radeon
# Intel:
sudo pacman -S vulkan-intel lib32-vulkan-intel
# NVIDIA:
sudo pacman -S nvidia nvidia-utils lib32-nvidia-utils
```

### openSUSE (zypper)
```bash
sudo zypper refresh && sudo zypper update
sudo zypper install vulkan-tools libvulkan1 libvulkan1-32bit
# AMD/Intel Mesa:
sudo zypper install Mesa-libGL1 libvulkan_radeon libvulkan_intel
```

Verify after reboot:
```bash
vulkaninfo --summary
```
This must list a real GPU (e.g. "Intel UHD Graphics", "AMD Radeon", "NVIDIA"). If the only device
is `llvmpipe`/`softpipe`, or only `dzn`/`lavapipe`, the hardware driver is still missing: install
the vendor package above. A `Skipping this driver ... libvulkan_dzn.so` line is fine as long as a
real GPU is also listed.

## Phase 2: install Tempest

```bash
curl -fsSL https://raw.githubusercontent.com/solomon-gleeson/tempest/master/install.sh | bash
```
Confirm `tempest` is on PATH (`command -v tempest`); if not, add the bin location the installer
prints. Then run the guided setup (installs Wine, builds a Wine prefix, installs DXVK and
vkd3d-proton, downloads Vortex, registers `vortex://`):
```bash
tempest setup
```
It is interactive and will ask to confirm Wine/GameMode installation. Answer yes.

## Phase 3: verify the stack

```bash
tempest doctor
```
Every line should be `[PASS]`. For any `[FAIL]`, run the fix it suggests and re-run. Notes:
- `receiver.exe not found`: ignore, it is harmless (in-game notifications only).
- Wine prefix, DXVK or vkd3d failures: re-run `tempest setup`.

Then authenticate and test:
```bash
tempest login          # opens browser; paste the session_token cookie when asked
tempest list           # confirms auth works
tempest play 4         # test launch; use any game id
```
Make sure my Vortex account email is verified. An unverified account can fail to launch.

## Phase 4: config fixes, only if a launch error appears

Config is at `~/.config/tempest/config.toml`. Put environment overrides under `[wine.env]`.
There can be only **one** `WINEDLLOVERRIDES` key, so combine values with commas, e.g.
`WINEDLLOVERRIDES = "d3dcompiler_47=n,windows.gaming.input=d"`. Edit the file, do not blindly
append. After each change, re-run `tempest play <id>` and report the new error.

Baseline that fixes most rendering issues:
```toml
[wine.env]
WGPU_BACKEND = "vulkan"
```

Map the exact error string to the fix:

| Error | Cause | Fix |
|---|---|---|
| `Failed to create Vulkan instance` / `Failed to initialize DXVK` | no hardware Vulkan driver | back to Phase 1, install vendor driver, reboot |
| `vkCreateInstance ... libvulkan_dzn.so ... return code -9`, no real GPU listed | only `dzn` ICD present | install the real Mesa/vendor driver (Phase 1) |
| `llvmpipe` / `softpipe` shown as the GPU | software renderer | install vendor driver (Phase 1); on Mint add the kisak PPA |
| `Features ... TEXTURE_FORMAT_16BIT_NORM are required but not enabled` | driver missing or too old | install/upgrade vendor driver (Phase 1) |
| `D3DCompile2 Failed to compile shader, vkd3d result -5` | DX12/HLSL path failing | `winetricks d3dcompiler_47`, then `WINEDLLOVERRIDES = "d3dcompiler_47=n"`, keep `WGPU_BACKEND = "vulkan"` |
| older GPU, still crashing in DX12 | vkd3d feature use too new | `VKD3D_CONFIG = "no_upload_hvv,single_queue"` and `WINEDLLOVERRIDES = "d3d12=b"` |
| Steam Deck / gamepad crash on launch | Windows.Gaming.Input under Wine | `WINEDLLOVERRIDES = "windows.gaming.input=d"` |
| `could not load kernel32.dll, status c0000135` / `Wine exited with code 53` | missing 32-bit Wine or uninitialised prefix (Debian-based) | `sudo dpkg --add-architecture i386 && sudo apt update && sudo apt install wine32:i386 wine64`, then `tempest uninstall` and `tempest setup` |
| hang/crash mentioning gstreamer/media | winegstreamer probing | `WINEDLLOVERRIDES = "winegstreamer="` |

To run `d3dcompiler_47` against Tempest's prefix when the table calls for it:
```bash
WINEPREFIX="$HOME/.local/share/tempest/prefix" WINEDEBUG=-all winetricks -q d3dcompiler_47
```

Worked example, Intel UHD 620 laptop:
```toml
[wine.env]
WGPU_BACKEND = "vulkan"
WINEDLLOVERRIDES = "d3dcompiler_47=n"
VKD3D_CONFIG = "no_upload_hvv,single_queue"
```

## Phase 5: optional tuning

Under `[launcher]`: `use_fsync = true` (kernel 5.16+/wine-staging), `use_esync = true` and
`shader_cache = true` are good defaults. Set `use_gamemode = true` after installing the
`gamemode` package. On a hybrid/Optimus laptop, force the discrete GPU under `[wine.env]`:
`VK_ICD_FILENAMES = "/usr/share/vulkan/icd.d/nvidia_icd.x86_64.json"`.

## Finish

Once `tempest play <id>` launches and renders, stop and report:
1. detected distro and GPU,
2. driver packages installed,
3. the final `[wine.env]` block,
4. anything still broken.

Change nothing beyond what is needed for a clean launch.

## Debug tips

- Launch with logs: `TEMPEST_LOG=debug tempest play 4`
- Re-check the stack: `tempest doctor`
- Start fresh: `tempest uninstall` then `tempest setup`
