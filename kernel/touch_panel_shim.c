// SPDX-License-Identifier: GPL-2.0-only

#include <linux/err.h>
#include <linux/slab.h>
#include <linux/spinlock.h>
#include <linux/workqueue.h>
#include <drm/drm_panel.h>

#include "touch_panel_shim.h"

struct nt36532_touch_panel {
	struct drm_panel_follower follower;
	struct work_struct start_work;
	spinlock_t queue_lock;
	bool panel_prepared;
	void *context;
	void (*start)(void *, struct nt36532_touch_panel *);
	void (*stop)(void *);
};

static void nt36532_touch_start_work(struct work_struct *work)
{
	struct nt36532_touch_panel *touch;

	touch = container_of(work, struct nt36532_touch_panel, start_work);
	touch->start(touch->context, touch);
}

bool nt36532_touch_panel_request_reload(struct nt36532_touch_panel *touch)
{
	bool scheduled = false;

	spin_lock(&touch->queue_lock);
	if (touch->panel_prepared)
		scheduled = schedule_work(&touch->start_work);
	spin_unlock(&touch->queue_lock);
	return scheduled;
}

static int nt36532_touch_panel_prepared(struct drm_panel_follower *follower)
{
	struct nt36532_touch_panel *touch;

	touch = container_of(follower, struct nt36532_touch_panel, follower);
	spin_lock(&touch->queue_lock);
	touch->panel_prepared = true;
	schedule_work(&touch->start_work);
	spin_unlock(&touch->queue_lock);
	return 0;
}

static int nt36532_touch_panel_unpreparing(struct drm_panel_follower *follower)
{
	struct nt36532_touch_panel *touch;

	touch = container_of(follower, struct nt36532_touch_panel, follower);
	spin_lock(&touch->queue_lock);
	touch->panel_prepared = false;
	spin_unlock(&touch->queue_lock);
	cancel_work_sync(&touch->start_work);
	touch->stop(touch->context);
	return 0;
}

static const struct drm_panel_follower_funcs nt36532_touch_panel_ops = {
	.panel_prepared = nt36532_touch_panel_prepared,
	.panel_unpreparing = nt36532_touch_panel_unpreparing,
};

struct nt36532_touch_panel *nt36532_touch_panel_register(
	struct device *dev, void *context, void (*start)(void *, struct nt36532_touch_panel *),
	void (*stop)(void *))
{
	struct nt36532_touch_panel *touch;
	int ret;

	touch = kzalloc(sizeof(*touch), GFP_KERNEL);
	if (!touch)
		return ERR_PTR(-ENOMEM);

	touch->context = context;
	touch->start = start;
	touch->stop = stop;
	touch->follower.funcs = &nt36532_touch_panel_ops;
	spin_lock_init(&touch->queue_lock);
	INIT_WORK(&touch->start_work, nt36532_touch_start_work);

	ret = drm_panel_add_follower(dev, &touch->follower);
	if (ret) {
		kfree(touch);
		return ERR_PTR(ret);
	}

	return touch;
}

void nt36532_touch_panel_unregister(struct nt36532_touch_panel *touch)
{
	drm_panel_remove_follower(&touch->follower);
	cancel_work_sync(&touch->start_work);
	kfree(touch);
}
