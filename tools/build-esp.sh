#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
set -eu
cd "$(dirname "$0")/.."
cargo build --locked --release --target x86_64-unknown-uefi
mkdir -p out/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/release/musha-boot.efi out/esp/EFI/BOOT/BOOTX64.EFI
printf '%s\n' 'Created out/esp/EFI/BOOT/BOOTX64.EFI'
