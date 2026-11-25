/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef NT36532_PANEL_LINKS_SHIM_H
#define NT36532_PANEL_LINKS_SHIM_H

#include <linux/types.h>

struct mipi_dsi_device;
struct nt36532_panel_links;

struct nt36532_panel_link_config {
	u16 slice_width;
	u16 slice_height;
	u16 bits_per_pixel;
	u8 slices_per_channel;
	u8 bits_per_component;
	u8 lanes;
	u8 version_major;
	u8 version_minor;
	bool block_prediction;
	bool burst_mode;
	bool noncontinuous_clock;
};

struct nt36532_panel_links *nt36532_panel_links_create(
	struct mipi_dsi_device *primary, const struct nt36532_panel_link_config *config,
	void *channels[2]);
int nt36532_panel_links_attach(struct nt36532_panel_links *links);
int nt36532_panel_links_pack_pps(struct nt36532_panel_links *links, u8 payload[128]);

#endif
