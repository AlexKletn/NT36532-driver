# NT36532-driver

[Русская версия](README.ru.md)

[Contributing](CONTRIBUTING.md) · [GPL-2.0-only](LICENSE)

Rust workspace for a Linux driver for the display and touch functions of the Novatek NT36532 controller.

| Crate | Role |
| --- | --- |
| `panel-domain` | Display lifecycle and coordination of VDDIO, matrix bias, DSI, and backlight |
| `panel-nt36532` | NT36532 reset and DSI command sequencing using a supplied panel profile |
| `panel-touch` | NT36532 touch firmware parsing and contact decoding |
| `panel-backlight` | Backlight brightness interface |
| `panel-power` | Panel power interface |
| `adapter-KTZ8866` | Backlight and power adapter for a pair of KTZ8866 chips. The board supplies the register buses and settings. |

`kernel/Kbuild` builds the `nt36532` touch module. With `KTZ8866_SRC` and the board panel profile, the same module also registers the dual-DSI panel.

Set `PANEL_PROFILE` to the path of a board TOML profile when building with Cargo. The profile declares a `strategy`, currently `dsi-command-sequence-v1`. The build generates static Rust tables using Python 3.11 or later. A build without a profile keeps the driver library available with `SELECTED_PROFILE` set to `None`.

See the [DRM/MSM patch notes](patches/README.md) for seamless refresh switching and the kernel version used for verification.

### Touch firmware

Set `TOUCH_PROFILES_DIR` to the absolute path of a directory containing board
TOML profiles when building with Cargo. Profile generation uses Python 3.11 or
later. At runtime, `FirmwareImage::parse` matches the ILM and DLM CRC fields in
the image header against the compiled profiles. It rejects unknown pairs and
exposes the matching profile through `FirmwareImage::profile`.

Supply the firmware as an external file at runtime. On Unix, the `std` feature
provides `read_firmware_path` and `read_firmware_fd` for regular files. The latter
reads from offset zero and preserves the caller's file position. The
`no_std` API accepts a byte slice through `FirmwareImage::parse`.

A touch module can pass a startup strategy to `FirmwareImage::parse_with_strategy`.
`TouchStrategy` accepts `auto`, `packed12-v1`, and `big-endian16-v1`. An explicit
strategy uses the matching CRC profile when available, or the sole compiled
profile for that format. It can override an unknown or different CRC pair.

The Linux firmware adapter uses `request_firmware()` and requires
`CONFIG_FW_LOADER`. It takes a name relative to the kernel's
firmware search paths and releases the image buffer after controller startup.
The Unix file descriptor API remains available to userspace callers.

From the driver repository, build against a configured kernel build tree using
the same Rust toolchain that built the kernel. Set `TOUCH_PROFILES_DIR` to
the board's touch profile directory.

```sh
make -C "$KERNEL_BUILD" M="$PWD/kernel" MO="$PWD/target/kernel" \
    PANEL_PROFILE="$PANEL_PROFILE" TOUCH_PROFILES_DIR="$TOUCH_PROFILES_DIR" modules
```

Set `preserve_firmware_once` to `1` under
`/sys/module/nt36532/parameters/` before a modeset that preserves controller
power and reset state. This one-shot option reuses the running image at the
next panel prepare.

The SPI device's `touch_state` sysfs attribute accepts `pause` and `resume`.
Pause drains the interrupt and releases active input. Resume queues firmware
startup when the panel is prepared.

Set `KTZ8866_SRC` to a checkout of KTZ8866-driver at the revision pinned in
`crates/adapter-KTZ8866/Cargo.toml` to compile the paired backlight adapter
into the kernel build. The driver sources stay in the separate checkout.

The combined KTZ8866 adapter registers one Linux backlight device named
`ktz8866-backlight` for both halves of the panel. DRM enables it after panel
preparation and blanks it before unprepare. Set `boot_dark=1` when loading
the module to start at zero brightness.

With the DRM/MSM patch applied, Kbuild detects the `set_live_refresh` callback.
The live path sends the selected mode commands without resetting the panel or
resending DSC PPS.
