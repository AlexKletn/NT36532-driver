/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef NT36532_KTZ_I2C_SHIM_H
#define NT36532_KTZ_I2C_SHIM_H

#include <linux/types.h>

struct device;
struct i2c_client;
struct gpio_desc;

struct i2c_client *nt36532_ktz_i2c_get(struct device *device, const char *property);
int nt36532_ktz_i2c_write(struct i2c_client *client, u8 reg, u8 value);
struct gpio_desc *nt36532_ktz_gpio_get(struct device *device, const char *name);

#endif
