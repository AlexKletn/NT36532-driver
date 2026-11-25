// SPDX-License-Identifier: GPL-2.0-only

#include <linux/device.h>
#include <linux/err.h>
#include <linux/gpio/consumer.h>
#include <linux/i2c.h>
#include <linux/of.h>

#include "ktz_i2c_shim.h"

static void nt36532_ktz_i2c_put(void *data)
{
	struct i2c_client *client = data;

	put_device(&client->dev);
}

struct i2c_client *nt36532_ktz_i2c_get(struct device *device, const char *property)
{
	struct device_node *node;
	struct i2c_client *client;
	int ret;

	node = of_parse_phandle(device->of_node, property, 0);
	if (!node)
		return ERR_PTR(-ENODEV);

	client = of_find_i2c_device_by_node(node);
	of_node_put(node);
	if (!client)
		return ERR_PTR(-EPROBE_DEFER);

	ret = devm_add_action_or_reset(device, nt36532_ktz_i2c_put, client);
	if (ret)
		return ERR_PTR(ret);
	return client;
}

int nt36532_ktz_i2c_write(struct i2c_client *client, u8 reg, u8 value)
{
	return i2c_smbus_write_byte_data(client, reg, value);
}

struct gpio_desc *nt36532_ktz_gpio_get(struct device *device, const char *name)
{
	return devm_gpiod_get(device, name, GPIOD_OUT_LOW);
}
