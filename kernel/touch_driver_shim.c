// SPDX-License-Identifier: GPL-2.0-only

#include <linux/gpio/consumer.h>
#include <linux/module.h>
#include <linux/of.h>
#include <linux/property.h>
#include <linux/spi/spi.h>

#include "touch_driver_shim.h"

static char *fw_name;
module_param(fw_name, charp, 0444);
MODULE_PARM_DESC(fw_name, "External touchscreen firmware name relative to the firmware search path");

static char *touch_strategy = "auto";
module_param(touch_strategy, charp, 0444);
MODULE_PARM_DESC(touch_strategy, "Touch strategy: auto, packed12-v1, or big-endian16-v1");

static int nt36532_spi_probe(struct spi_device *spi)
{
	struct device *dev = &spi->dev;
	struct gpio_desc *reset;
	void *context;
	int ret;

	if (!fw_name || !*fw_name)
		return dev_err_probe(dev, -EINVAL, "set fw_name to an external firmware file\n");
	if (spi->irq <= 0)
		return dev_err_probe(dev, -EINVAL, "missing touch interrupt\n");

	spi->mode = SPI_MODE_0;
	spi->bits_per_word = 8;
	ret = spi_setup(spi);
	if (ret)
		return dev_err_probe(dev, ret, "cannot set up touch SPI\n");

	reset = devm_gpiod_get(dev, "reset", GPIOD_OUT_HIGH);
	if (IS_ERR(reset))
		return dev_err_probe(dev, PTR_ERR(reset), "cannot acquire touch reset GPIO\n");

	ret = nt36532_touch_probe(spi, dev, reset, spi->irq, fw_name, touch_strategy,
				  device_property_read_bool(dev, "novatek,pen-support"),
				  device_property_read_bool(dev, "novatek,stylus-resol-double"),
				  &context);
	if (!ret)
		spi_set_drvdata(spi, context);
	return ret;
}

static void nt36532_spi_remove(struct spi_device *spi)
{
	nt36532_touch_remove(spi_get_drvdata(spi));
}

static const struct of_device_id nt36532_touch_of_match[] = {
	{ .compatible = "novatek,nt36532-ts" },
	{ }
};
MODULE_DEVICE_TABLE(of, nt36532_touch_of_match);

static ssize_t pen_tool_show(struct device *dev, struct device_attribute *attr,
			     char *buf)
{
	return sysfs_emit(buf, "%s\n",
			  nt36532_touch_is_eraser(dev_get_drvdata(dev)) ? "eraser" : "pen");
}

static ssize_t pen_tool_store(struct device *dev, struct device_attribute *attr,
			      const char *buf, size_t count)
{
	int ret;

	if (!sysfs_streq(buf, "toggle"))
		return -EINVAL;
	ret = nt36532_touch_toggle_pen_tool(dev_get_drvdata(dev));
	return ret ? ret : count;
}
static DEVICE_ATTR_RW(pen_tool);

static ssize_t touch_state_show(struct device *dev, struct device_attribute *attr,
				char *buf)
{
	return sysfs_emit(buf, "%s\n", nt36532_touch_state(dev_get_drvdata(dev)));
}

static ssize_t touch_state_store(struct device *dev, struct device_attribute *attr,
				 const char *buf, size_t count)
{
	if (sysfs_streq(buf, "pause"))
		nt36532_touch_set_paused(dev_get_drvdata(dev), true);
	else if (sysfs_streq(buf, "resume"))
		nt36532_touch_set_paused(dev_get_drvdata(dev), false);
	else
		return -EINVAL;
	return count;
}
static DEVICE_ATTR_RW(touch_state);

static ssize_t firmware_history_show(struct device *dev,
				     struct device_attribute *attr, char *buf)
{
	u8 primary[64], secondary[64];
	int count = 0;
	int available;

	available = nt36532_touch_read_history(dev_get_drvdata(dev), &primary, &secondary);
	if (available < 0)
		return available;
	if (available & 1)
		count += sysfs_emit_at(buf, count, "0x121afc: %*phN\n",
				       (int)sizeof(primary), primary);
	else
		count += sysfs_emit_at(buf, count, "0x121afc: unavailable\n");
	if (available & 2)
		count += sysfs_emit_at(buf, count, "0x121b3c: %*phN\n",
				       (int)sizeof(secondary), secondary);
	else
		count += sysfs_emit_at(buf, count, "0x121b3c: unavailable\n");
	return count;
}
static DEVICE_ATTR_ADMIN_RO(firmware_history);

static struct attribute *nt36532_touch_attrs[] = {
	&dev_attr_pen_tool.attr,
	&dev_attr_touch_state.attr,
	&dev_attr_firmware_history.attr,
	NULL,
};
ATTRIBUTE_GROUPS(nt36532_touch);

static struct spi_driver nt36532_touch_driver = {
	.probe = nt36532_spi_probe,
	.remove = nt36532_spi_remove,
	.driver = {
		.name = "novatek-nt36532-ts",
		.of_match_table = nt36532_touch_of_match,
		.dev_groups = nt36532_touch_groups,
	},
};

int nt36532_touch_driver_register(void)
{
	return spi_register_driver(&nt36532_touch_driver);
}

void nt36532_touch_driver_unregister(void)
{
	spi_unregister_driver(&nt36532_touch_driver);
}

int nt36532_touch_set_reset(struct gpio_desc *reset, bool asserted)
{
	return gpiod_set_value_cansleep(reset, asserted);
}

static bool preserve_firmware_once;
module_param(preserve_firmware_once, bool, 0644);
MODULE_PARM_DESC(preserve_firmware_once, "Reuse running touch firmware across the next panel modeset");

bool nt36532_touch_take_preserve_request(void)
{
	bool preserve;

	kernel_param_lock(THIS_MODULE);
	preserve = preserve_firmware_once;
	preserve_firmware_once = false;
	kernel_param_unlock(THIS_MODULE);
	return preserve;
}

static bool capture_reset_history;
module_param(capture_reset_history, bool, 0600);
MODULE_PARM_DESC(capture_reset_history, "Capture firmware history on the next 0xfd report");

bool nt36532_touch_take_history_request(void)
{
	bool capture;

	kernel_param_lock(THIS_MODULE);
	capture = capture_reset_history;
	capture_reset_history = false;
	kernel_param_unlock(THIS_MODULE);
	return capture;
}
