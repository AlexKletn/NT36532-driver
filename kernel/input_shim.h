/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef NT36532_INPUT_SHIM_H
#define NT36532_INPUT_SHIM_H

#include <linux/types.h>

struct device;
struct nt36532_input;

struct nt36532_input *nt36532_input_create(struct device *device, bool with_pen,
					   bool double_pen_resolution,
					   u16 limits[4]);
void nt36532_input_contact(struct nt36532_input *input, u8 slot, u16 x, u16 y,
			   u8 width);
void nt36532_input_finish_contacts(struct nt36532_input *input);
void nt36532_input_pen_position(struct nt36532_input *input, u16 x, u16 y,
				s8 tilt_x, s8 tilt_y);
void nt36532_input_pen_state(struct nt36532_input *input, u16 pressure,
			     u16 distance, bool stylus, bool stylus2, bool eraser);

#endif
