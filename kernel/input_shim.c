// SPDX-License-Identifier: GPL-2.0-only

#include <linux/device.h>
#include <linux/err.h>
#include <linux/input.h>
#include <linux/input/mt.h>
#include <linux/input/touchscreen.h>
#include <linux/of.h>
#include <linux/slab.h>

#include "input_shim.h"

struct nt36532_input {
	struct input_dev *touch;
	struct input_dev *pen;
	struct touchscreen_properties touch_properties;
	struct touchscreen_properties pen_properties;
};

static void nt36532_set_pen_resolution(struct device *device, struct input_dev *pen)
{
	struct device_node *panel;
	u32 width_mm, height_mm;

	panel = of_parse_phandle(device->of_node, "panel", 0);
	if (!panel)
		return;

	if (!of_property_read_u32(panel, "width-mm", &width_mm) && width_mm)
		input_abs_set_res(pen, ABS_X, (input_abs_get_max(pen, ABS_X) + 1) / width_mm);
	if (!of_property_read_u32(panel, "height-mm", &height_mm) && height_mm)
		input_abs_set_res(pen, ABS_Y, (input_abs_get_max(pen, ABS_Y) + 1) / height_mm);
	of_node_put(panel);
}

static int nt36532_create_pen(struct device *device, struct nt36532_input *input,
			     bool double_resolution)
{
	struct input_dev *pen = devm_input_allocate_device(device);
	struct touchscreen_properties *properties = &input->pen_properties;
	unsigned int max_x, max_y;

	if (!pen)
		return -ENOMEM;

	*properties = input->touch_properties;
	max_x = properties->max_x;
	max_y = properties->max_y;
	if (double_resolution) {
		max_x = 2 * max_x + 1;
		max_y = 2 * max_y + 1;
	}
	if (properties->swap_x_y) {
		input_set_abs_params(pen, ABS_X, 0, max_y, 0, 0);
		input_set_abs_params(pen, ABS_Y, 0, max_x, 0, 0);
	} else {
		input_set_abs_params(pen, ABS_X, 0, max_x, 0, 0);
		input_set_abs_params(pen, ABS_Y, 0, max_y, 0, 0);
	}
	properties->max_x = max_x;
	properties->max_y = max_y;

	pen->name = "Novatek NT36532 Pen";
	pen->id.bustype = BUS_SPI;
	input_set_abs_params(pen, ABS_PRESSURE, 0, 4095, 0, 0);
	input_set_abs_params(pen, ABS_DISTANCE, 0, 1, 0, 0);
	input_set_abs_params(pen, ABS_TILT_X, -90, 90, 0, 0);
	input_set_abs_params(pen, ABS_TILT_Y, -90, 90, 0, 0);
	input_abs_set_res(pen, ABS_TILT_X, 1);
	input_abs_set_res(pen, ABS_TILT_Y, 1);
	nt36532_set_pen_resolution(device, pen);
	__set_bit(INPUT_PROP_DIRECT, pen->propbit);
	input_set_capability(pen, EV_KEY, BTN_TOUCH);
	input_set_capability(pen, EV_KEY, BTN_TOOL_PEN);
	input_set_capability(pen, EV_KEY, BTN_TOOL_RUBBER);
	input_set_capability(pen, EV_KEY, BTN_STYLUS);
	input_set_capability(pen, EV_KEY, BTN_STYLUS2);
	input->pen = pen;
	return input_register_device(pen);
}

struct nt36532_input *nt36532_input_create(struct device *device, bool with_pen,
					   bool double_pen_resolution,
					   u16 limits[4])
{
	struct nt36532_input *input;
	struct input_dev *touch;
	int ret;

	input = devm_kzalloc(device, sizeof(*input), GFP_KERNEL);
	if (!input)
		return ERR_PTR(-ENOMEM);

	touch = devm_input_allocate_device(device);
	if (!touch)
		return ERR_PTR(-ENOMEM);
	input->touch = touch;
	touch->name = "Novatek NT36532 Touchscreen";
	touch->id.bustype = BUS_SPI;
	input_set_abs_params(touch, ABS_MT_POSITION_X, 0, 2943, 0, 0);
	input_set_abs_params(touch, ABS_MT_POSITION_Y, 0, 1839, 0, 0);
	input_set_abs_params(touch, ABS_MT_TOUCH_MAJOR, 0, 255, 0, 0);
	touchscreen_parse_properties(touch, true, &input->touch_properties);
	if (input->touch_properties.max_x > U16_MAX ||
	    input->touch_properties.max_y > U16_MAX ||
	    (with_pen && double_pen_resolution &&
	     (input->touch_properties.max_x > U16_MAX / 2 ||
	      input->touch_properties.max_y > U16_MAX / 2)))
		return ERR_PTR(-EINVAL);

	ret = input_mt_init_slots(touch, 10, INPUT_MT_DIRECT | INPUT_MT_DROP_UNUSED);
	if (ret)
		return ERR_PTR(ret);
	ret = input_register_device(touch);
	if (ret)
		return ERR_PTR(ret);
	if (with_pen) {
		ret = nt36532_create_pen(device, input, double_pen_resolution);
		if (ret)
			return ERR_PTR(ret);
	}

	limits[0] = input->touch_properties.max_x;
	limits[1] = input->touch_properties.max_y;
	limits[2] = input->pen_properties.max_x;
	limits[3] = input->pen_properties.max_y;
	return input;
}

void nt36532_input_contact(struct nt36532_input *input, u8 slot, u16 x, u16 y,
			   u8 width)
{
	input_mt_slot(input->touch, slot);
	input_mt_report_slot_state(input->touch, MT_TOOL_FINGER, true);
	touchscreen_report_pos(input->touch, &input->touch_properties, x, y, true);
	input_report_abs(input->touch, ABS_MT_TOUCH_MAJOR, width);
}

void nt36532_input_finish_contacts(struct nt36532_input *input)
{
	input_mt_sync_frame(input->touch);
	input_sync(input->touch);
}

void nt36532_input_pen_position(struct nt36532_input *input, u16 x, u16 y,
				s8 raw_tilt_x, s8 raw_tilt_y)
{
	struct touchscreen_properties *properties = &input->pen_properties;
	int tilt_x = raw_tilt_x;
	int tilt_y = raw_tilt_y;

	if (!input->pen)
		return;

	touchscreen_report_pos(input->pen, properties, x, y, false);
	if (properties->invert_x)
		tilt_x = -tilt_x;
	if (properties->invert_y)
		tilt_y = -tilt_y;
	if (properties->swap_x_y)
		swap(tilt_x, tilt_y);
	input_report_abs(input->pen, ABS_TILT_X, tilt_x);
	input_report_abs(input->pen, ABS_TILT_Y, tilt_y);
}

void nt36532_input_pen_state(struct nt36532_input *input, u16 pressure,
			     u16 distance, bool stylus, bool stylus2, bool eraser)
{
	struct input_dev *pen = input->pen;

	if (!pen)
		return;

	input_report_abs(pen, ABS_PRESSURE, pressure);
	input_report_abs(pen, ABS_DISTANCE, distance);
	input_report_key(pen, BTN_TOUCH, pressure != 0);
	input_report_key(pen, BTN_TOOL_PEN, !eraser && (pressure || distance));
	input_report_key(pen, BTN_TOOL_RUBBER, eraser && (pressure || distance));
	input_report_key(pen, BTN_STYLUS, stylus);
	input_report_key(pen, BTN_STYLUS2, stylus2);
	input_sync(pen);
}
