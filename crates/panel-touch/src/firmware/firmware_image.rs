// SPDX-License-Identifier: GPL-2.0-only
use crate::{FirmwareError, FirmwareProfile, Partition, TOUCH_PROFILES, TouchStrategy};

pub struct FirmwareImage<'a> {
    bytes: &'a [u8],
    partitions: [Partition; MAX_PARTITIONS],
    count: usize,
    cascade: bool,
    profile: &'static FirmwareProfile,
}

impl<'a> FirmwareImage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, FirmwareError> {
        Self::parse_with_strategy(bytes, TouchStrategy::Auto)
    }

    pub fn parse_with_strategy(
        bytes: &'a [u8],
        strategy: TouchStrategy,
    ) -> Result<Self, FirmwareError> {
        let mut partitions = [Partition::default(); MAX_PARTITIONS];
        let (count, cascade) =
            parse(bytes, &mut partitions).ok_or(FirmwareError::InvalidPartitionTable)?;
        let crc = [partitions[0].crc, partitions[1].crc];
        let profile = select_profile(crc, strategy)?;
        Ok(Self {
            bytes,
            partitions,
            count,
            cascade,
            profile,
        })
    }

    pub fn profile(&self) -> &'static FirmwareProfile {
        self.profile
    }

    pub fn partitions(&self) -> &[Partition] {
        &self.partitions[..self.count]
    }

    pub fn is_cascade(&self) -> bool {
        self.cascade
    }

    pub fn partition_data(&self, index: usize) -> Option<&'a [u8]> {
        let partition = self.partitions().get(index)?;
        let start = partition.file_offset as usize;
        self.bytes
            .get(start..start.checked_add(partition.byte_count as usize)?)
    }
}

fn select_profile(
    crc: [u32; 2],
    strategy: TouchStrategy,
) -> Result<&'static FirmwareProfile, FirmwareError> {
    let known = TOUCH_PROFILES.iter().find(|profile| profile.crc == crc);
    let Some(format) = strategy.contact_format() else {
        return known.ok_or(FirmwareError::UnknownProfile { crc });
    };
    if let Some(profile) = known.filter(|profile| profile.contact_format == format) {
        return Ok(profile);
    }
    let mut candidates = TOUCH_PROFILES
        .iter()
        .filter(|profile| profile.contact_format == format);
    let profile = candidates
        .next()
        .ok_or(FirmwareError::StrategyUnavailable)?;
    if candidates.next().is_some() {
        return Err(FirmwareError::AmbiguousStrategy);
    }
    Ok(profile)
}

const PRIMARY_PARTITIONS: usize = 2;
const MAX_PARTITIONS: usize = 32;
const INFO_START: usize = 0x30;
const ENTRY_BYTES: usize = 0x10;

fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

fn entry(bytes: &[u8], offset: usize) -> Option<Partition> {
    let entry = bytes.get(offset..offset.checked_add(ENTRY_BYTES)?)?;
    Some(Partition {
        sram_address: word(entry, 0)?,
        byte_count: word(entry, 4)?,
        file_offset: word(entry, 8)?,
        crc: word(entry, 12)?,
    })
}

fn parse(bytes: &[u8], parts: &mut [Partition; MAX_PARTITIONS]) -> Option<(usize, bool)> {
    if bytes.len() < 0x40 {
        return None;
    }
    let header_len = word(bytes, 0)? as usize;
    if !(INFO_START..=bytes.len()).contains(&header_len) {
        return None;
    }
    let cascade = bytes[0x20] & 2 != 0;
    let info_end = if cascade { header_len / 2 } else { header_len };
    let info_count =
        info_end.saturating_sub(INFO_START).div_ceil(ENTRY_BYTES) + usize::from(cascade);
    let overlay_flags = bytes[0x28];
    let overlay_count = if overlay_flags & 0x10 != 0 {
        usize::from(overlay_flags & 0x0f)
    } else {
        0
    };
    let count = PRIMARY_PARTITIONS + info_count + overlay_count;
    if count > MAX_PARTITIONS {
        return None;
    }

    for (index, part) in parts[..PRIMARY_PARTITIONS].iter_mut().enumerate() {
        let offset = index * 12;
        *part = Partition {
            file_offset: word(bytes, offset)?,
            sram_address: word(bytes, offset + 4)?,
            byte_count: word(bytes, offset + 8)?,
            crc: word(bytes, 0x18 + index * 4)?,
        };
        if part.byte_count == 0 {
            return None;
        }
    }
    let overlay_start = parts[1].file_offset as usize;
    let (info, overlays) = parts[PRIMARY_PARTITIONS..count].split_at_mut(info_count);
    for (index, part) in info.iter_mut().enumerate() {
        let offset = if cascade && index + 1 == info_count {
            header_len - ENTRY_BYTES
        } else {
            INFO_START + ENTRY_BYTES * index
        };
        *part = entry(bytes, offset)?;
    }
    for (index, part) in overlays.iter_mut().enumerate() {
        *part = entry(bytes, overlay_start.checked_add(ENTRY_BYTES * index)?)?;
    }

    let mut used = 0;
    for index in 0..count {
        let part = parts[index];
        if part.byte_count == 0 {
            continue;
        }
        let transfer_bytes = part.byte_count.checked_add(1)?;
        let end = u64::from(part.file_offset) + u64::from(transfer_bytes);
        if end > bytes.len() as u64 || part.sram_address.checked_add(transfer_bytes).is_none() {
            return None;
        }
        parts[used] = Partition {
            byte_count: transfer_bytes,
            ..part
        };
        used += 1;
    }
    Some((used, cascade))
}
