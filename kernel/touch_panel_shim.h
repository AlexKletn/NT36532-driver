/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef NT36532_TOUCH_PANEL_SHIM_H
#define NT36532_TOUCH_PANEL_SHIM_H

#include <linux/types.h>

struct device;
struct nt36532_touch_panel;

struct nt36532_touch_panel *nt36532_touch_panel_register(
	struct device *dev, void *context, void (*start)(void *, struct nt36532_touch_panel *),
	void (*stop)(void *));
bool nt36532_touch_panel_request_reload(struct nt36532_touch_panel *touch);
void nt36532_touch_panel_unregister(struct nt36532_touch_panel *touch);

#endif
