// SPDX-License-Identifier: GPL-2.0-only

#include <drm/display/drm_dsc.h>
#include <drm/drm_connector.h>
#include <drm/drm_modes.h>
#include <drm/drm_mipi_dsi.h>
#include <linux/string.h>
#include <linux/gpio/consumer.h>
#include <linux/regulator/consumer.h>
#include <video/mipi_display.h>

int nt36532_dsi_write(struct mipi_dsi_device *dsi, u8 packet_type,
		      bool low_power, const u8 *payload, size_t length);
int nt36532_dsi_send_pps(struct mipi_dsi_device *dsi, const u8 *payload);

int nt36532_dsi_write(struct mipi_dsi_device *dsi, u8 packet_type,
		      bool low_power, const u8 *payload, size_t length)
{
	struct mipi_dsi_msg message = {
		.channel = dsi->channel,
		.type = packet_type,
		.flags = low_power ? MIPI_DSI_MSG_USE_LPM : 0,
		.tx_len = length,
		.tx_buf = payload,
	};
	ssize_t result;

	if ((packet_type == MIPI_DSI_DCS_SHORT_WRITE && length != 1) ||
	    (packet_type == MIPI_DSI_DCS_LONG_WRITE && length < 2) ||
	    (packet_type != MIPI_DSI_DCS_SHORT_WRITE &&
	     packet_type != MIPI_DSI_DCS_LONG_WRITE))
		return -EINVAL;

	if (!dsi->host || !dsi->host->ops || !dsi->host->ops->transfer)
		return -ENODEV;

	/* The DCS helpers choose packet type from length; the panel profile
	 * needs to preserve explicit long writes even for short payloads.
	 */
	result = dsi->host->ops->transfer(dsi->host, &message);
	return result < 0 ? result : 0;
}

int nt36532_dsi_send_pps(struct mipi_dsi_device *dsi, const u8 *payload)
{
	struct drm_dsc_picture_parameter_set pps;

	static_assert(sizeof(pps) == 128);
	memcpy(&pps, payload, sizeof(pps));
	return mipi_dsi_picture_parameter_set(dsi, &pps);
}

int nt36532_panel_gpio_set(struct gpio_desc *reset, bool asserted);
int nt36532_panel_set_vddio(struct regulator *vddio, bool enabled);

int nt36532_panel_gpio_set(struct gpio_desc *reset, bool asserted)
{
	return gpiod_set_value_cansleep(reset, asserted);
}

int nt36532_panel_set_vddio(struct regulator *vddio, bool enabled)
{
	return enabled ? regulator_enable(vddio) : regulator_disable(vddio);
}

int nt36532_drm_add_mode(struct drm_connector *connector, int clock_khz,
			 const u16 horizontal[4], const u16 vertical[4],
			 bool preferred);

int nt36532_drm_add_mode(struct drm_connector *connector, int clock_khz,
			 const u16 horizontal[4], const u16 vertical[4],
			 bool preferred)
{
	struct drm_display_mode template = {
		.clock = clock_khz,
		.hdisplay = horizontal[0],
		.hsync_start = horizontal[1],
		.hsync_end = horizontal[2],
		.htotal = horizontal[3],
		.vdisplay = vertical[0],
		.vsync_start = vertical[1],
		.vsync_end = vertical[2],
		.vtotal = vertical[3],
		.width_mm = connector->display_info.width_mm,
		.height_mm = connector->display_info.height_mm,
		.type = DRM_MODE_TYPE_DRIVER | (preferred ? DRM_MODE_TYPE_PREFERRED : 0),
	};
	struct drm_display_mode *mode = drm_mode_duplicate(connector->dev, &template);

	if (!mode)
		return -ENOMEM;
	drm_mode_set_name(mode);
	drm_mode_probed_add(connector, mode);
	return 1;
}
