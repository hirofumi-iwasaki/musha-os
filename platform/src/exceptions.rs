// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Exception stubs normalize absent error codes to zero; UI must label them absent.
pub const fn hardware_error(vector: u64) -> bool {
    matches!(vector, 8 | 10 | 11 | 12 | 13 | 14 | 17 | 21 | 29 | 30)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_error_code_is_not_a_hardware_zero() {
        for v in [
            0, 1, 2, 3, 4, 5, 6, 7, 9, 15, 16, 18, 19, 20, 22, 28, 31, 255,
        ] {
            assert!(!hardware_error(v));
        }
        for v in [8, 10, 11, 12, 13, 14, 17, 21, 29, 30] {
            assert!(hardware_error(v));
        }
    }
}
