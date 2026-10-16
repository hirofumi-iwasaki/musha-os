// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Pure PAT selection/update planning and bounded firmware translation auditing.
use crate::Error;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Indices {
    pub wb: u8,
    pub uc: u8,
    pub framebuffer: u8,
}
impl Indices {
    /// RAM remains WB and DMA/device registers remain strong UC. Only the
    /// framebuffer may select an existing WC entry; otherwise it stays UC.
    pub fn select(pat: u64) -> Result<Self, Error> {
        let find =
            |kind: u8| (0u8..8).find(|i| ((pat >> (usize::from(*i) * 8)) & 255) == u64::from(kind));
        let wb = find(6).ok_or(Error::Unsupported)?;
        let uc = find(0).ok_or(Error::Unsupported)?;
        Ok(Self {
            wb,
            uc,
            framebuffer: find(1).unwrap_or(uc),
        })
    }
    pub fn framebuffer_wc(self) -> bool {
        self.framebuffer != self.uc
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const DEFAULT: u64 = 0x0007_0406_0007_0406;
    #[test]
    fn reset_pat_falls_back_to_strong_uc_without_treating_uc_minus_as_wc() {
        assert_eq!(
            Indices::select(DEFAULT),
            Ok(Indices {
                wb: 0,
                uc: 3,
                framebuffer: 3
            })
        );
        assert!(!Indices::select(DEFAULT).unwrap().framebuffer_wc());
    }
    #[test]
    fn every_available_wc_slot_is_selected_without_changing_ram_or_device_indices() {
        for slot in 0..8 {
            let pat = (DEFAULT & !(255u64 << (slot * 8))) | (1u64 << (slot * 8));
            let selected = Indices::select(pat).unwrap();
            assert_eq!(selected.framebuffer, slot as u8);
            assert!(selected.framebuffer_wc());
            assert_eq!((pat >> (selected.wb * 8)) & 255, 6);
            assert_eq!((pat >> (selected.uc * 8)) & 255, 0);
        }
    }
    #[test]
    fn missing_required_ram_or_device_type_is_rejected() {
        assert_eq!(
            Indices::select(0x0101_0101_0101_0101),
            Err(Error::Unsupported)
        );
        assert_eq!(
            Indices::select(0x0606_0606_0606_0606),
            Err(Error::Unsupported)
        );
        assert_eq!(Indices::select(0), Err(Error::Unsupported));
    }
}

/// A single IA32_PAT write: preserve seven bytes and dedicate entry 1 to GOP WC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Update {
    pub before: u64,
    pub requested: u64,
    pub indices: Indices,
}
impl Update {
    pub fn entry1_wc(before: u64) -> Result<Self, Error> {
        for index in 0..8 {
            if !matches!((before >> (index * 8)) & 255, 0 | 1 | 4 | 5 | 6 | 7) {
                return Err(Error::Unsupported);
            }
        }
        let requested = (before & !(255 << 8)) | (1 << 8);
        // Our CR3 and nonleaf entries use selector 0. Keep page-walk accesses
        // WB, matching the WB identity mapping of the page-table RAM itself.
        if requested & 255 != 6 {
            return Err(Error::Unsupported);
        }
        let mut indices = Indices::select(requested)?;
        indices.framebuffer = 1;
        Ok(Self {
            before,
            requested,
            indices,
        })
    }
    pub fn verified(self, readback: u64) -> bool {
        readback == self.requested
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditError {
    Address,
    TableMemory,
    Entry1InUse,
    Cycle,
    Budget,
    Entry,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audit {
    pub tables: usize,
    pub leaves: usize,
}
/// Audit all present firmware translations, including aliases, huge leaves and
/// paging-structure cache types. Never follow a pointer before `table_memory`
/// approves its complete 4 KiB RAM page. Four-level paging/PCIDE=0 only.
/// The callback contract includes readable identity mappings (UEFI x64 entry).
/// Repeated shared subtrees consume the same bounded work budget; no allocation.
pub fn audit_firmware(
    cr3: u64,
    physical_bits: u8,
    max_tables: usize,
    mut table_memory: impl FnMut(u64) -> bool,
    mut read: impl FnMut(u64, usize) -> Result<u64, AuditError>,
) -> Result<Audit, AuditError> {
    if !(36..=52).contains(&physical_bits) {
        return Err(AuditError::Address);
    }
    if (cr3 >> 3) & 3 == 1 {
        return Err(AuditError::Entry1InUse);
    }
    let mut audit = Audit {
        tables: 0,
        leaves: 0,
    };
    walk(
        cr3 & 0x000f_ffff_ffff_f000,
        4,
        physical_bits,
        max_tables,
        &mut [0; 4],
        &mut audit,
        &mut table_memory,
        &mut read,
    )?;
    Ok(audit)
}
fn walk(
    table: u64,
    level: usize,
    bits: u8,
    budget: usize,
    ancestors: &mut [u64; 4],
    audit: &mut Audit,
    memory: &mut impl FnMut(u64) -> bool,
    read: &mut impl FnMut(u64, usize) -> Result<u64, AuditError>,
) -> Result<(), AuditError> {
    if table == 0 || table & 4095 != 0 || table >= (1u64 << bits) || table >= (1u64 << 47) {
        return Err(AuditError::Address);
    }
    if ancestors[..4 - level].contains(&table) {
        return Err(AuditError::Cycle);
    }
    if audit.tables == budget {
        return Err(AuditError::Budget);
    }
    if !memory(table) {
        return Err(AuditError::TableMemory);
    }
    ancestors[4 - level] = table;
    audit.tables += 1;
    for index in 0..512 {
        let entry = read(table, index)?;
        if entry & 1 == 0 {
            continue;
        }
        let huge = level > 1 && entry & 128 != 0;
        if huge && level == 4 {
            return Err(AuditError::Entry);
        }
        let leaf = level == 1 || huge;
        let pat = if leaf {
            (entry >> if huge { 12 } else { 7 }) & 1
        } else {
            0
        };
        if ((entry >> 3) & 3) | (pat << 2) == 1 {
            return Err(AuditError::Entry1InUse);
        }
        if leaf {
            audit.leaves += 1;
        } else {
            walk(
                entry & 0x000f_ffff_ffff_f000,
                level - 1,
                bits,
                budget,
                ancestors,
                audit,
                memory,
                read,
            )?;
        }
    }
    Ok(())
}

/// PAT/PCD/PWT bits for a 4 KiB leaf. Permissions/address bits are untouched.
pub fn pte_bits(index: u8) -> Result<u64, Error> {
    if index > 7 {
        return Err(Error::Invalid);
    }
    Ok(((index as u64 & 1) << 3)
        | (((index as u64 >> 1) & 1) << 4)
        | (((index as u64 >> 2) & 1) << 7))
}
pub fn pte_index(leaf: u64) -> u8 {
    (((leaf >> 3) & 3) | (((leaf >> 7) & 1) << 2)) as u8
}

#[cfg(test)]
mod update_tests {
    use super::*;
    const DEFAULT: u64 = 0x0007_0406_0007_0406;
    #[test]
    fn leaf_bits_keep_wc_distinct_from_wb_uc_and_permissions() {
        for index in 0..8 {
            let bits = pte_bits(index).unwrap();
            assert_eq!(pte_index(bits | 0x12345003 | (1 << 63)), index);
            assert_eq!(bits & !0x98, 0);
        }
        assert_eq!(pte_bits(1), Ok(8));
        assert_eq!(pte_bits(3), Ok(24));
        assert_eq!(pte_bits(8), Err(Error::Invalid));
    }
    #[test]
    fn alias_in_upper_half_and_nonpresent_garbage_are_audited() {
        assert_eq!(
            audit_with(&[(0x1000, 511, 0x2003), (0x2000, 511, 0x89)], 4),
            Err(AuditError::Entry1InUse)
        );
        assert_eq!(
            audit_with(&[(0x1000, 511, 0x2008)], 4),
            Ok(Audit {
                tables: 1,
                leaves: 0
            })
        );
    }
    #[test]
    fn update_changes_only_entry1_and_verifies_the_whole_msr() {
        let p = Update::entry1_wc(DEFAULT).unwrap();
        assert_eq!(p.requested, 0x0007_0406_0007_0106);
        assert_eq!(
            p.indices,
            Indices {
                wb: 0,
                uc: 3,
                framebuffer: 1
            }
        );
        assert!(p.verified(p.requested));
        for bit in 0..64 {
            assert!(!p.verified(p.requested ^ (1 << bit)));
        }
        assert_eq!(p.before, DEFAULT);
    }
    #[test]
    fn replacement_requires_ram_and_dma_types_outside_entry1() {
        assert!(Update::entry1_wc(0x0000_0000_0000_0600).is_err());
        assert!(Update::entry1_wc(0x0606_0606_0606_0006).is_err());
        for slot in 0..8 {
            for invalid in [2u64, 3, 8, 255] {
                let pat = (DEFAULT & !(255u64 << (slot * 8))) | (invalid << (slot * 8));
                assert!(Update::entry1_wc(pat).is_err());
            }
        }
        assert_eq!(
            Update::entry1_wc((DEFAULT & !255) | 4),
            Err(Error::Unsupported)
        );
    }
    fn audit_with(entries: &[(u64, usize, u64)], budget: usize) -> Result<Audit, AuditError> {
        audit_firmware(
            0x1000,
            48,
            budget,
            |p| [0x1000, 0x2000, 0x3000, 0x4000].contains(&p),
            |p, i| {
                Ok(entries
                    .iter()
                    .find(|e| e.0 == p && e.1 == i)
                    .map_or(0, |e| e.2))
            },
        )
    }
    #[test]
    fn old_firmware_4k_and_huge_entry1_and_walk_cache_are_rejected() {
        let path = [
            (0x1000, 0, 0x2003),
            (0x2000, 0, 0x3003),
            (0x3000, 0, 0x4003),
        ];
        let mut entries = path.to_vec();
        entries.push((0x4000, 9, 0x9009));
        assert_eq!(audit_with(&entries, 4), Err(AuditError::Entry1InUse));
        for (p, huge) in [(0x2000, 0x89), (0x3000, 0x89), (0x1000, 0x2009)] {
            let mut entries = path.to_vec();
            entries.retain(|e| e.0 != p);
            entries.push((p, 0, huge));
            assert_eq!(audit_with(&entries, 4), Err(AuditError::Entry1InUse));
        }
        assert_eq!(
            audit_firmware(0x1008, 48, 4, |_| panic!(), |_, _| panic!()),
            Err(AuditError::Entry1InUse)
        );
    }
    #[test]
    fn pat_bit_positions_do_not_confuse_index5_with_index1() {
        assert_eq!(
            audit_with(&[(0x1000, 0, 0x2003), (0x2000, 0, 0x1089)], 4),
            Ok(Audit {
                tables: 2,
                leaves: 1
            })
        );
        assert_eq!(
            audit_with(
                &[
                    (0x1000, 0, 0x2003),
                    (0x2000, 0, 0x3003),
                    (0x3000, 0, 0x4003),
                    (0x4000, 0, 0x9089)
                ],
                4
            ),
            Ok(Audit {
                tables: 4,
                leaves: 1
            })
        );
    }
    #[test]
    fn audit_bounds_bad_pointers_cycles_and_work_before_dereference() {
        assert_eq!(
            audit_with(&[(0x1000, 0, 0x9003)], 4),
            Err(AuditError::TableMemory)
        );
        assert_eq!(
            audit_with(&[(0x1000, 0, 0x1003)], 4),
            Err(AuditError::Cycle)
        );
        assert_eq!(
            audit_with(&[(0x1000, 0, 0x2003)], 1),
            Err(AuditError::Budget)
        );
        assert_eq!(audit_with(&[(0x1000, 0, 0x83)], 4), Err(AuditError::Entry));
        assert_eq!(
            audit_firmware(
                0x1000,
                48,
                4,
                |_| false,
                |_, _| panic!("read before validation")
            ),
            Err(AuditError::TableMemory)
        );
    }
}
