// SPDX-License-Identifier: GPL-2.0-only

#include <drm/drm_connector.h>
#include <drm/drm_crtc.h>
#include <drm/drm_modes.h>
#include <drm/drm_panel.h>
#include <linux/backlight.h>
#include <linux/device.h>
#include <linux/err.h>
#include <linux/moduleparam.h>
#include <linux/mutex.h>
#include <linux/property.h>

#include "drm_panel_shim.h"

static bool boot_dark;
module_param(boot_dark, bool, 0444);
MODULE_PARM_DESC(boot_dark, "Start the backlight at zero brightness");

struct nt36532_drm_panel {
	struct drm_panel panel;
	struct mutex callback_lock;
	void *context;
	int (*prepare)(void *, u16);
	struct drm_connector *connector;
	u8 bits_per_component;
	int (*unprepare)(void *);
	int (*set_brightness)(void *, u16);
	int (*set_live_refresh)(void *, u16);
};

int nt36532_panel_add_modes(struct drm_connector *connector);

static u16 nt36532_drm_panel_refresh(const struct nt36532_drm_panel *display)
{
	struct drm_connector *connector = display->connector;
	struct drm_crtc *crtc;
	int refresh;

	if (!connector || !connector->state)
		return 0;

	crtc = connector->state->crtc;
	if (!crtc || !crtc->state)
		return 0;

	refresh = drm_mode_vrefresh(&crtc->state->adjusted_mode);
	return refresh > 0 && refresh <= U16_MAX ? refresh : 0;
}

static int nt36532_drm_panel_prepare(struct drm_panel *panel)
{
	struct nt36532_drm_panel *display =
		container_of(panel, struct nt36532_drm_panel, panel);
	int ret = -ENODEV;

	mutex_lock(&display->callback_lock);
	if (display->context)
		ret = display->prepare(display->context, nt36532_drm_panel_refresh(display));
	mutex_unlock(&display->callback_lock);
	return ret;
}

static int nt36532_drm_panel_unprepare(struct drm_panel *panel)
{
	struct nt36532_drm_panel *display =
		container_of(panel, struct nt36532_drm_panel, panel);
	int ret = 0;

	mutex_lock(&display->callback_lock);
	if (display->context)
		ret = display->unprepare(display->context);
	mutex_unlock(&display->callback_lock);
	return ret;
}

static int nt36532_drm_panel_get_modes(struct drm_panel *panel,
				       struct drm_connector *connector)
{
	struct nt36532_drm_panel *display =
		container_of(panel, struct nt36532_drm_panel, panel);
	struct drm_connector *previous = NULL;
	u32 width, height;

	mutex_lock(&display->callback_lock);
	if (display->context && display->connector != connector) {
		drm_connector_get(connector);
		previous = display->connector;
		display->connector = connector;
	}
	mutex_unlock(&display->callback_lock);
	if (previous)
		drm_connector_put(previous);

	if (!device_property_read_u32(panel->dev, "width-mm", &width) &&
	    !device_property_read_u32(panel->dev, "height-mm", &height) &&
	    width <= U16_MAX && height <= U16_MAX) {
		connector->display_info.width_mm = width;
		connector->display_info.height_mm = height;
	}
	connector->display_info.bpc = display->bits_per_component;
	return nt36532_panel_add_modes(connector);
}

static int nt36532_backlight_update(struct backlight_device *backlight)
{
	struct nt36532_drm_panel *display = bl_get_data(backlight);
	int ret = -ENODEV;

	mutex_lock(&display->callback_lock);
	if (display->context)
		ret = display->set_brightness(display->context,
					      backlight_get_brightness(backlight));
	mutex_unlock(&display->callback_lock);
	return ret;
}

static const struct backlight_ops nt36532_backlight_ops = {
	.options = BL_CORE_SUSPENDRESUME,
	.update_status = nt36532_backlight_update,
};

#ifdef NT36532_HAS_DRM_LIVE_REFRESH
static int nt36532_drm_set_live_refresh(struct drm_panel *panel, u16 refresh)
{
	struct nt36532_drm_panel *display =
		container_of(panel, struct nt36532_drm_panel, panel);
	int ret = -ENODEV;

	mutex_lock(&display->callback_lock);
	if (display->context)
		ret = display->set_live_refresh(display->context, refresh);
	mutex_unlock(&display->callback_lock);
	return ret;
}
#endif

static const struct drm_panel_funcs nt36532_panel_ops = {
	.prepare = nt36532_drm_panel_prepare,
	.unprepare = nt36532_drm_panel_unprepare,
	.get_modes = nt36532_drm_panel_get_modes,
#ifdef NT36532_HAS_DRM_LIVE_REFRESH
	.set_live_refresh = nt36532_drm_set_live_refresh,
#endif
};

struct nt36532_drm_panel *nt36532_drm_panel_register(
	struct device *device, void *context, int (*prepare)(void *, u16),
	int (*unprepare)(void *), int (*set_brightness)(void *, u16),
	u16 max_brightness, u8 bits_per_component, int (*set_live_refresh)(void *, u16))
{
	struct backlight_properties properties = {
		.type = BACKLIGHT_RAW,
		.brightness = boot_dark ? 0 : max_brightness,
		.max_brightness = max_brightness,
		.power = BACKLIGHT_POWER_OFF,
	};
	struct nt36532_drm_panel *display;
	struct backlight_device *backlight;

	display = devm_drm_panel_alloc(device, struct nt36532_drm_panel, panel,
				       &nt36532_panel_ops, DRM_MODE_CONNECTOR_DSI);
	if (IS_ERR(display))
		return display;

	mutex_init(&display->callback_lock);
	display->context = context;
	display->prepare = prepare;
	display->bits_per_component = bits_per_component;
	display->unprepare = unprepare;
	display->set_brightness = set_brightness;
	display->set_live_refresh = set_live_refresh;
	backlight = backlight_device_register("ktz8866-backlight", device, display,
					      &nt36532_backlight_ops, &properties);
	if (IS_ERR(backlight))
		return ERR_CAST(backlight);
	display->panel.backlight = backlight;
	display->panel.prepare_prev_first = true;
	drm_panel_add(&display->panel);
	return display;
}

void nt36532_drm_panel_unregister(struct nt36532_drm_panel *display)
{
	struct backlight_device *backlight;
	struct drm_connector *connector;

	drm_panel_remove(&display->panel);
	if (READ_ONCE(display->panel.enabled))
		drm_panel_disable(&display->panel);
	if (READ_ONCE(display->panel.prepared))
		drm_panel_unprepare(&display->panel);

	mutex_lock(&display->panel.follower_lock);
	backlight = display->panel.backlight;
	display->panel.backlight = NULL;
	mutex_unlock(&display->panel.follower_lock);
	backlight_device_unregister(backlight);

	mutex_lock(&display->callback_lock);
	if (display->context) {
		display->unprepare(display->context);
		display->context = NULL;
	}
	connector = display->connector;
	display->connector = NULL;
	mutex_unlock(&display->callback_lock);
	if (connector)
		drm_connector_put(connector);
}
