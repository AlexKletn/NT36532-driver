/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef NT36532_DRM_PANEL_SHIM_H
#define NT36532_DRM_PANEL_SHIM_H

#include <linux/types.h>

struct device;
struct nt36532_drm_panel;

struct nt36532_drm_panel *nt36532_drm_panel_register(
	struct device *device, void *context, int (*prepare)(void *, u16),
	int (*unprepare)(void *), int (*set_brightness)(void *, u16),
	u16 max_brightness, u8 bits_per_component, int (*set_live_refresh)(void *, u16));
void nt36532_drm_panel_unregister(struct nt36532_drm_panel *panel);

#endif
