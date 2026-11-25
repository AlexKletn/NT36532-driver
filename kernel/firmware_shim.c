// SPDX-License-Identifier: GPL-2.0-only

#include <linux/err.h>
#include <linux/firmware.h>

const struct firmware *nt36532_request_firmware(const char *name, struct device *device,
						const u8 **data, size_t *size);

const struct firmware *nt36532_request_firmware(const char *name, struct device *device,
						const u8 **data, size_t *size)
{
	const struct firmware *firmware;
	int ret;

	ret = request_firmware(&firmware, name, device);
	if (ret)
		return ERR_PTR(ret);
	*data = firmware->data;
	*size = firmware->size;
	return firmware;
}
