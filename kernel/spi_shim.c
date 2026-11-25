// SPDX-License-Identifier: GPL-2.0-only

#include <linux/slab.h>
#include <linux/spi/spi.h>

int nt36532_spi_transfer(struct spi_device *spi, const u8 *tx, u8 *rx, size_t len);

int nt36532_spi_transfer(struct spi_device *spi, const u8 *tx, u8 *rx, size_t len)
{
	struct spi_transfer transfer = { .len = len };
	u8 *tx_dma;
	u8 *rx_dma = NULL;
	int ret;

	/* Rust callers may pass stack buffers, which are unsuitable for DMA. */
	tx_dma = kmemdup(tx, len, GFP_KERNEL);
	if (!tx_dma)
		return -ENOMEM;

	if (rx) {
		rx_dma = kmalloc(len, GFP_KERNEL);
		if (!rx_dma) {
			kfree(tx_dma);
			return -ENOMEM;
		}
	}

	transfer.tx_buf = tx_dma;
	transfer.rx_buf = rx_dma;
	ret = spi_sync_transfer(spi, &transfer, 1);
	if (!ret && rx)
		memcpy(rx, rx_dma, len);

	kfree(rx_dma);
	kfree(tx_dma);
	return ret;
}
