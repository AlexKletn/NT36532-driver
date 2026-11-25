/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef NT36532_TOUCH_DRIVER_SHIM_H
#define NT36532_TOUCH_DRIVER_SHIM_H

#include <linux/types.h>

struct device;
struct gpio_desc;
struct spi_device;

int nt36532_touch_driver_register(void);
bool nt36532_touch_take_preserve_request(void);
bool nt36532_touch_take_history_request(void);
int nt36532_touch_set_reset(struct gpio_desc *reset, bool asserted);
void nt36532_touch_driver_unregister(void);

int nt36532_touch_probe(struct spi_device *spi, struct device *dev,
			struct gpio_desc *reset, int irq, const char *firmware,
			const char *strategy, bool with_pen, bool double_pen_resolution,
			void **context);
void nt36532_touch_remove(void *context);
bool nt36532_touch_is_eraser(const void *context);
int nt36532_touch_toggle_pen_tool(const void *context);
const char *nt36532_touch_state(const void *context);
void nt36532_touch_set_paused(const void *context, bool paused);

int nt36532_touch_read_history(const void *context, u8 (*primary)[64],
			      u8 (*secondary)[64]);

#endif
