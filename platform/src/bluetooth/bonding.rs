// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Bridge a verified SMP result to the existing fixed PAIRING.DAT codec. The
//! callback must use SETTINGS write_owned + sync. No filename/config write API.
use super::{Error, legacy::VerifiedBond};
use crate::pairing::{self, Bond, CAPACITY, MAX_BONDS};
use zeroize::Zeroize;
const EMPTY: Bond = Bond {
    address_type: 0,
    address: [0; 6],
    key_size: 0,
    flags: 0,
    ediv: 0,
    random: [0; 8],
    ltk: [0; 16],
    irk: [0; 16],
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveError {
    Record(pairing::Error),
    Full,
    Security,
    PersistenceUnknown,
}
impl VerifiedBond {
    /// Only returns the new consumed hash after the writer confirms durability.
    /// On write/sync failure RAM must keep its old value; disk contents may still
    /// be changed (the existing writer provides no power-failure atomicity).
    pub fn save(
        self,
        old: Option<&[u8]>,
        raw_config: Option<[u8; 32]>,
        write: impl FnOnce(&[u8; CAPACITY]) -> Result<(), Error>,
    ) -> Result<Option<[u8; 32]>, SaveError> {
        let mut bonds = [EMPTY; MAX_BONDS];
        let result = (|| {
            let (mut n, consumed) = match old {
                Some(b) => pairing::decode_state(b, &mut bonds).map_err(SaveError::Record)?,
                None => (0, None),
            };
            let index = bonds[..n].iter().position(|b| {
                b.address_type == self.bond.address_type && b.address == self.bond.address
            });
            let index = if let Some(i) = index {
                // Do not silently replace authenticated/SC keys with weaker keys.
                if bonds[i].flags & !self.bond.flags != 0 || bonds[i].key_size > self.bond.key_size
                {
                    return Err(SaveError::Security);
                }
                i
            } else {
                if n == MAX_BONDS {
                    return Err(SaveError::Full);
                }
                n += 1;
                n - 1
            };
            bonds[index] = self.bond;
            let next = raw_config.or(consumed);
            let mut bytes = [0; CAPACITY];
            let encoded =
                pairing::encode_state(&bonds[..n], next, &mut bytes).map_err(SaveError::Record);
            let written =
                encoded.and_then(|_| write(&bytes).map_err(|_| SaveError::PersistenceUnknown));
            bytes.zeroize();
            written?;
            Ok(next)
        })();
        for bond in &mut bonds {
            bond.ltk.zeroize();
            bond.irk.zeroize();
        }
        result
    }
}
impl Drop for VerifiedBond {
    fn drop(&mut self) {
        self.bond.ltk.zeroize();
        self.bond.irk.zeroize();
    }
}
