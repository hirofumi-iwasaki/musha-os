// SPDX-License-Identifier: Apache-2.0
//! Pure range policy for the C1 probe, not a general page-table/copyin API.
pub fn offset(pointer: u64, length: u64, base: u64, bytes: u64) -> Option<usize> {
    let end = base.checked_add(bytes)?;
    if base == 0 || end > 1u64 << 47 || bytes == 0 {
        return None;
    }
    let last = pointer.checked_add(length)?;
    if pointer < base || pointer >= end || last > end {
        return None;
    }
    usize::try_from(pointer - base).ok()
}
pub fn valid_return(
    rip: u64,
    rsp: u64,
    code: u64,
    code_bytes: u64,
    stack: u64,
    stack_bytes: u64,
) -> bool {
    // RSP points just above usable bytes. Require at least one push to fit;
    // reject the guard and all noncanonical/wrapping values before IRETQ.
    offset(rip, 1, code, code_bytes).is_some()
        && rsp
            .checked_sub(8)
            .is_some_and(|p| offset(p, 8, stack, stack_bytes).is_some())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copy_bounds() {
        assert_eq!(offset(0x1000, 4096, 0x1000, 4096), Some(0));
        assert_eq!(offset(0x1fff, 1, 0x1000, 4096), Some(4095));
        for (p, n) in [
            (0, 1),
            (0xfff, 1),
            (0x2000, 1),
            (0x1fff, 2),
            (u64::MAX, 2),
            (0x1000, u64::MAX),
        ] {
            assert_eq!(offset(p, n, 0x1000, 4096), None);
        }
        assert_eq!(offset(1, 1, 0, 4096), None);
        assert_eq!(offset(u64::MAX, 1, u64::MAX, 2), None);
        assert_eq!(offset(1 << 47, 1, 1 << 47, 4096), None);
    }
    #[test]
    fn return_bounds() {
        let ok = |pc, sp| valid_return(pc, sp, 0x400000, 128, 0x6000, 4096);
        assert!(ok(0x400000, 0x7000));
        assert!(ok(0x40007f, 0x6008));
        for pc in [0, 0x3fffff, 0x400080, 1 << 47, u64::MAX] {
            assert!(!ok(pc, 0x7000));
        }
        for sp in [0, 0x6000, 0x6007, 0x7001, 1 << 47, u64::MAX] {
            assert!(!ok(0x400000, sp));
        }
    }
}
