// SPDX-License-Identifier: GPL-2.0-only

#include <drm/drm_mipi_dsi.h>
#include <linux/gpio/consumer.h>
#include <linux/module.h>
#include <linux/of.h>
#include <linux/regulator/consumer.h>

int nt36532_panel_driver_register(void);
void nt36532_panel_driver_unregister(void);
int nt36532_panel_probe(struct mipi_dsi_device *dsi, struct device *device,
			 struct gpio_desc *reset, struct regulator *vddio,
			 void **context);
void nt36532_panel_remove(void *context);

static int nt36532_dsi_probe(struct mipi_dsi_device *dsi)
{
	struct device *device = &dsi->dev;
	struct gpio_desc *reset;
	struct regulator *vddio;
	void *context;
	int ret;

	vddio = devm_regulator_get(device, "vddio");
	if (IS_ERR(vddio))
		return dev_err_probe(device, PTR_ERR(vddio), "cannot acquire panel VDDIO\n");
	reset = devm_gpiod_get(device, "reset", GPIOD_OUT_HIGH);
	if (IS_ERR(reset))
		return dev_err_probe(device, PTR_ERR(reset), "cannot acquire panel reset\n");

	ret = nt36532_panel_probe(dsi, device, reset, vddio, &context);
	if (ret)
		return dev_err_probe(device, ret, "cannot initialize NT36532 panel\n");

	mipi_dsi_set_drvdata(dsi, context);
	return 0;
}

static void nt36532_dsi_remove(struct mipi_dsi_device *dsi)
{
	nt36532_panel_remove(mipi_dsi_get_drvdata(dsi));
}

static const struct of_device_id nt36532_panel_match[] = {
	{ .compatible = "lenovo,spinel-display" },
	{ }
};
MODULE_DEVICE_TABLE(of, nt36532_panel_match);

static struct mipi_dsi_driver nt36532_panel_driver = {
	.probe = nt36532_dsi_probe,
	.remove = nt36532_dsi_remove,
	.driver = {
		.name = "nt36532-panel",
		.of_match_table = nt36532_panel_match,
	},
};

int nt36532_panel_driver_register(void)
{
	return mipi_dsi_driver_register(&nt36532_panel_driver);
}

void nt36532_panel_driver_unregister(void)
{
	mipi_dsi_driver_unregister(&nt36532_panel_driver);
}
