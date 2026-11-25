# DRM/MSM seamless refresh switching

[Русская версия](README.ru.md)

The [patch](drm-msm-seamless-refresh.patch) adds experimental live refresh switching for the panel. It updates the DRM/MSM display pipeline timings and calls the panel's `set_live_refresh` callback. Enable the path with `msm.spinel_seamless=1`.

The tablet project pins [Linux v7.3.6](https://github.com/gregkh/linux/tree/v7.3.6) at commit `1a0388846138eda9f6eb4258ff227a223afc2f52`. Hardware testing was limited to the Lenovo TB371FC running an adapted Linux 7.3.6 tree. Build #5 switched through 120 → 60 → 30 → 144 → 120 Hz while updating display timings live.

This revision applies to the clean `v7.3.6` checkout. The patched ARM64 kernel and `nt36532.ko` were built together. Module symbol validation passed against the patched kernel's `Module.symvers`. The rebased patch awaits a hardware test.
