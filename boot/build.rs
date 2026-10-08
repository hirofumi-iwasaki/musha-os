// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use std::{env, path::PathBuf, process::Command};
fn main() {
    if env::var("TARGET").unwrap() != "x86_64-unknown-uefi" {
        return;
    }
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_owned();
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());
    let sources = [
        "core/def.c",
        "core/inet_chksum.c",
        "core/init.c",
        "core/ip.c",
        "core/mem.c",
        "core/memp.c",
        "core/netif.c",
        "core/pbuf.c",
        "core/stats.c",
        "core/sys.c",
        "core/timeouts.c",
        "core/udp.c",
        "core/ipv4/etharp.c",
        "core/ipv4/icmp.c",
        "core/ipv4/ip4.c",
        "core/ipv4/ip4_addr.c",
        "netif/ethernet.c",
    ];
    let mut all: Vec<PathBuf> = sources
        .iter()
        .map(|s| root.join("third_party/lwip/src").join(s))
        .collect();
    all.extend([root.join("net/port/shim.c"), root.join("net/port/compat.c")]);
    println!("cargo:rerun-if-changed={}", root.join("net/port").display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("third_party/lwip").display()
    );
    println!("cargo:rerun-if-env-changed=CC");
    for (index, source) in all.iter().enumerate() {
        let object = output.join(format!("lwip-{index}.obj"));
        let status = Command::new(env::var("CC").unwrap_or_else(|_| "clang".into()))
            .args([
                "--target=x86_64-pc-windows-msvc",
                "-std=c11",
                "-O2",
                "-ffreestanding",
                "-fno-builtin",
                "-fno-stack-protector",
                "-mno-red-zone",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-c",
            ])
            .arg("-I")
            .arg(root.join("net/port"))
            .arg("-I")
            .arg(root.join("third_party/lwip/src/include"))
            .arg(source)
            .arg("-o")
            .arg(&object)
            .status()
            .expect("Clang is required for the lwIP C port");
        assert!(
            status.success(),
            "lwIP C compilation failed: {}",
            source.display()
        );
        println!("cargo:rustc-link-arg={}", object.display());
    }
}
