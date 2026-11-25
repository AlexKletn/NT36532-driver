// SPDX-License-Identifier: GPL-2.0-only

#include <linux/device.h>
#include <linux/err.h>
#include <linux/interrupt.h>
#include <linux/jiffies.h>
#include <linux/slab.h>
#include <linux/workqueue.h>

#include "touch_irq_shim.h"

struct nt36532_touch_irq {
	struct delayed_work watchdog_work;
	void (*interrupt)(void *);
	void (*watchdog)(void *);
	void *context;
	int irq;
};

static irqreturn_t nt36532_touch_irq_thread(int irq, void *data)
{
	struct nt36532_touch_irq *source = data;

	source->interrupt(source->context);
	return IRQ_HANDLED;
}

static void nt36532_touch_watchdog(struct work_struct *work)
{
	struct nt36532_touch_irq *source;

	source = container_of(to_delayed_work(work), struct nt36532_touch_irq,
			      watchdog_work);
	source->watchdog(source->context);
}

struct nt36532_touch_irq *nt36532_touch_irq_create(
	struct device *dev, int irq, void *context,
	void (*interrupt)(void *), void (*watchdog)(void *))
{
	struct nt36532_touch_irq *source;
	int ret;

	source = kzalloc(sizeof(*source), GFP_KERNEL);
	if (!source)
		return ERR_PTR(-ENOMEM);
	source->irq = irq;
	source->context = context;
	source->interrupt = interrupt;
	source->watchdog = watchdog;
	INIT_DELAYED_WORK(&source->watchdog_work, nt36532_touch_watchdog);
	disable_delayed_work(&source->watchdog_work);

	ret = request_threaded_irq(irq, NULL, nt36532_touch_irq_thread,
				   IRQF_ONESHOT | IRQF_NO_AUTOEN,
				   dev_name(dev), source);
	if (ret) {
		kfree(source);
		return ERR_PTR(ret);
	}
	return source;
}

void nt36532_touch_irq_enable(struct nt36532_touch_irq *source)
{
	enable_delayed_work(&source->watchdog_work);
	enable_irq(source->irq);
}

void nt36532_touch_irq_disable(struct nt36532_touch_irq *source)
{
	disable_irq(source->irq);
	disable_delayed_work_sync(&source->watchdog_work);
}

void nt36532_touch_irq_arm_watchdog(struct nt36532_touch_irq *source,
				    unsigned int delay_ms)
{
	mod_delayed_work(system_wq, &source->watchdog_work,
			 msecs_to_jiffies(delay_ms));
}

void nt36532_touch_irq_destroy(struct nt36532_touch_irq *source)
{
	free_irq(source->irq, source);
	disable_delayed_work_sync(&source->watchdog_work);
	kfree(source);
}
