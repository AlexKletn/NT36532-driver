/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef NT36532_TOUCH_IRQ_SHIM_H
#define NT36532_TOUCH_IRQ_SHIM_H

#include <linux/types.h>

struct device;
struct nt36532_touch_irq;

struct nt36532_touch_irq *nt36532_touch_irq_create(
	struct device *dev, int irq, void *context,
	void (*interrupt)(void *), void (*watchdog)(void *));
void nt36532_touch_irq_enable(struct nt36532_touch_irq *source);
void nt36532_touch_irq_disable(struct nt36532_touch_irq *source);
void nt36532_touch_irq_arm_watchdog(struct nt36532_touch_irq *source,
				    unsigned int delay_ms);
void nt36532_touch_irq_destroy(struct nt36532_touch_irq *source);

#endif
