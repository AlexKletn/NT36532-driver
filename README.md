# NT36532-driver

[Русская версия](README.ru.md)

[Contributing](CONTRIBUTING.md) · [GPL-2.0-only](LICENSE)

Rust workspace for a Linux driver for the display and touch functions of the Novatek NT36532 controller.

| Crate | Role |
| --- | --- |
| `panel-domain` | Planned display lifecycle and coordination |
| `panel-nt36532` | Planned NT36532 panel logic over DSI |
| `panel-touch` | Placeholder for NT36532 touch input over SPI |
| `panel-backlight` | Backlight brightness interface |
| `panel-power` | Panel power interface |
| `adapter-KTZ8866` | Adapter for both interfaces. The hardware backend is pending. |

`kernel/` contains scaffolding for Kbuild, the module entry point, and the DSI boundary between C and Rust.
