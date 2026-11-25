// SPDX-License-Identifier: GPL-2.0-only

#include <drm/display/drm_dsc.h>
#include <drm/display/drm_dsc_helper.h>
#include <drm/drm_mipi_dsi.h>
#include <linux/device.h>
#include <linux/err.h>
#include <linux/of.h>
#include <linux/of_graph.h>
#include <linux/slab.h>

#include "panel_links_shim.h"

struct nt36532_panel_links {
	struct mipi_dsi_device *channels[2];
	struct drm_dsc_config dsc[2];
	bool attached;
	bool pps_ready;
};

static void nt36532_panel_links_clear_config(void *data)
{
	struct nt36532_panel_links *links = data;
	unsigned int index;

	for (index = 0; index < ARRAY_SIZE(links->channels); index++)
		links->channels[index]->dsc = NULL;
}

struct nt36532_panel_links *nt36532_panel_links_create(
	struct mipi_dsi_device *primary, const struct nt36532_panel_link_config *config,
	void *channels[2])
{
	const struct mipi_dsi_device_info secondary_info = { .type = "dsi-secondary" };
	struct device *device = &primary->dev;
	struct nt36532_panel_links *links;
	struct mipi_dsi_host *secondary_host;
	struct device_node *secondary_node;
	unsigned int index;
	int ret;

	if (!config->lanes || config->lanes > 4 || !config->slice_width ||
	    !config->slice_height || !config->slices_per_channel ||
	    !config->bits_per_component || !config->bits_per_pixel)
		return ERR_PTR(-EINVAL);

	secondary_node = of_graph_get_remote_node(device->of_node, 1, -1);
	if (!secondary_node)
		return ERR_PTR(-ENODEV);
	secondary_host = of_find_mipi_dsi_host_by_node(secondary_node);
	of_node_put(secondary_node);
	if (!secondary_host)
		return ERR_PTR(-EPROBE_DEFER);
	if (secondary_host == primary->host)
		return ERR_PTR(-EINVAL);

	links = devm_kzalloc(device, sizeof(*links), GFP_KERNEL);
	if (!links)
		return ERR_PTR(-ENOMEM);
	links->channels[0] = primary;
	links->channels[1] = devm_mipi_dsi_device_register_full(device, secondary_host,
							      &secondary_info);
	if (IS_ERR(links->channels[1]))
		return ERR_CAST(links->channels[1]);

	for (index = 0; index < ARRAY_SIZE(links->channels); index++) {
		struct mipi_dsi_device *channel = links->channels[index];
		struct drm_dsc_config *dsc = &links->dsc[index];

		dsc->dsc_version_major = config->version_major;
		dsc->dsc_version_minor = config->version_minor;
		dsc->slice_width = config->slice_width;
		dsc->slice_height = config->slice_height;
		dsc->slice_count = config->slices_per_channel;
		dsc->bits_per_component = config->bits_per_component;
		dsc->bits_per_pixel = config->bits_per_pixel;
		dsc->block_pred_enable = config->block_prediction;
		channel->lanes = config->lanes;
		channel->format = MIPI_DSI_FMT_RGB888;
		channel->mode_flags = MIPI_DSI_MODE_VIDEO;
		if (config->burst_mode)
			channel->mode_flags |= MIPI_DSI_MODE_VIDEO_BURST;
		if (config->noncontinuous_clock)
			channel->mode_flags |= MIPI_DSI_CLOCK_NON_CONTINUOUS;
		channel->dsc = dsc;
		channels[index] = channel;
	}
	ret = devm_add_action_or_reset(device, nt36532_panel_links_clear_config, links);
	if (ret)
		return ERR_PTR(ret);

	return links;
}

int nt36532_panel_links_attach(struct nt36532_panel_links *links)
{
	struct device *device = &links->channels[0]->dev;
	unsigned int index;
	void *group;
	int ret;

	if (links->attached)
		return -EBUSY;

	group = devres_open_group(device, NULL, GFP_KERNEL);
	if (!group)
		return -ENOMEM;
	for (index = 0; index < ARRAY_SIZE(links->channels); index++) {
		/* The second MSM attach can synchronously prepare the panel. */
		if (index == 1)
			smp_store_release(&links->pps_ready, true);
		ret = devm_mipi_dsi_attach(device, links->channels[index]);
		if (ret) {
			smp_store_release(&links->pps_ready, false);
			/* Probe tears down the DRM session before devres detaches DSI. */
			devres_remove_group(device, group);
			return ret;
		}
	}
	devres_remove_group(device, group);
	links->attached = true;
	return 0;
}

int nt36532_panel_links_pack_pps(struct nt36532_panel_links *links, u8 payload[128])
{
	struct drm_dsc_picture_parameter_set pps = {};
	struct drm_dsc_config channel;

	static_assert(sizeof(pps) == 128);
	if (!smp_load_acquire(&links->pps_ready))
		return -EAGAIN;
	channel = links->dsc[0];
	if (!channel.pic_width || !channel.pic_height)
		return -EAGAIN;
	if (channel.pic_width != 2U * channel.slice_width * channel.slice_count)
		return -EINVAL;

	channel.pic_width /= 2;
	drm_dsc_pps_payload_pack(&pps, &channel);
	memcpy(payload, &pps, sizeof(pps));
	return 0;
}
