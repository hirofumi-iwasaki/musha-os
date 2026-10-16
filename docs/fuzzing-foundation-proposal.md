# Host fuzzing foundation / host fuzz基盤案

Status: proposal, 2026-10-16. Documentation only; no targets, dependency installation, fuzz run or CI change performed. Related: [security policy](security-policy.md).

## English

Use a separate `fuzz/` Cargo workspace and host `std`/libFuzzer executable depending on the existing `no_std` parser crates. Never fuzz the UEFI binary, MMIO, a real drive or DMA. This preserves the release workspace, pinned release toolchain and abort profile. Before adoption, pin a reviewed nightly, cargo-fuzz version, `libfuzzer-sys` version/lockfile and license records. The upstream manifest currently names 0.4.13 with `(MIT OR Apache-2.0) AND NCSA`; this is a candidate dependency, not an accepted addition. Keep `fuzz/Cargo.lock` and reviewed seed corpus in Git; runtime crash artifacts are retained privately, minimized and converted into regression tests before disclosure.

Proposed `fuzz/Cargo.toml` skeleton (add two further `[[bin]]` entries with the same flags for `elf_note` and `config`):

```toml
[package]
name = "musha-fuzz"
version = "0.0.0"
edition = "2024"
publish = false
[package.metadata]
cargo-fuzz = true
[workspace]
members = ["."]
[dependencies]
libfuzzer-sys = "=0.4.13" # candidate; review/pin toolchain and lock before adoption
musha-fs = { path = "../fs" }
musha-memory = { path = "../memory" }
musha-platform = { path = "../platform", default-features = false }
[[bin]]
name = "fat32"
path = "fuzz_targets/fat32.rs"
test = false
doc = false
bench = false
```

All three target files start with `#![no_main]` and `use libfuzzer_sys::fuzz_target;`. Bodies use the production entry points, not a second parser:

```rust
// config.rs: production parser includes the 4096-byte bound/ASCII rules.
fuzz_target!(|data: &[u8]| { let _ = musha_platform::config::parse(data); });
// elf_note.rs: parses ELF + experimental r1/r2 NOTE, no map or execution.
fuzz_target!(|data: &[u8]| { let _ = musha_memory::elf::Plan::parse(data); });
// fat32.rs: immutable bounded fake block device; no filesystem writes.
fuzz_target!(|data: &[u8]| {
    let Some((&mode, data)) = data.split_first() else { return; };
    if data.len() > 1_048_576 { return; }
    let size = if mode & 1 != 0 { 4096 } else { 512 };
    let blocks = data.len() / size;
    let mut reads = 0u32;
    let read = |lba: u64, out: &mut [u8]| {
        reads += 1;
        if reads > 256 { return Err(musha_fs::Error::Limit); }
        let start = usize::try_from(lba).ok().and_then(|v| v.checked_mul(size))
            .ok_or(musha_fs::Error::Io)?;
        let end = start.checked_add(out.len()).ok_or(musha_fs::Error::Io)?;
        let sector = data.get(start..end).ok_or(musha_fs::Error::Io)?;
        out.copy_from_slice(sector);
        Ok(())
    };
    let _ = musha_fs::fat32::FileSystem::discover(blocks as u64, size, read);
});
```

Seeds: prefix each FAT32 disk fixture with one sector-size selector byte (0=512, 1=4096); generated MBR/GPT/superfloppy FAT32 fixtures including loops/overflow/duplicate names; C/Rust SDK ELF and malformed NOTE cases; empty/4096/4097-byte CONFIG, CRLF, malformed/non-ASCII and all log levels. No real `PAIRING.DAT`, keys or private user files. Parse rejection is expected; crashes, sanitizer findings, hangs and violated bounds fail. Later add bounded `read_at` after accepted FAT32 discovery, exFAT and the other policy targets; this first sketch does not cover them.

CI proposal: Linux x86-64, one worker per target, ASan, each target 30 seconds and whole job 5 minutes, per-input timeout 2 seconds/RSS 256 MiB. Example after pinned tools and corpus exist: `cargo +"$PINNED_NIGHTLY" fuzz run fat32 --target x86_64-unknown-linux-gnu -- -max_total_time=30 -timeout=2 -rss_limit_mb=256 -max_len=1048577`; use max_len 65536 for ELF and 4097 for CONFIG. Preserve failure artifacts/log/toolchain/seed, do not rerun until green to hide a failure. A bounded clean run is coverage evidence, not proof of safety. Existing host tests and deterministic `input/examples/hid_fuzz.rs` stay independent. Templates have not been compiled or run in this documentation task.

Primary references checked 2026-10-16: [Rust Fuzz Book tutorial](https://rust-fuzz.github.io/book/cargo-fuzz/tutorial.html), [CI guide](https://rust-fuzz.github.io/book/cargo-fuzz/ci.html), [libFuzzer limits](https://www.llvm.org/docs/LibFuzzer.html), [upstream dependency/license manifest](https://github.com/rust-fuzz/libfuzzer/blob/main/Cargo.toml). Repository entry points: `fs/src/fat32.rs:36`, `memory/src/elf.rs:53`, `platform/src/config.rs:33`.

## 日本語

`fuzz/` を独立 Cargo workspace とし、host `std`／libFuzzer 実行ファイルから既存 `no_std` parser crate を呼ぶ。UEFI binary・MMIO・実媒体・DMA は対象にしない。公開 workspace／固定 toolchain／abort profile を変えず、採用前に nightly・cargo-fuzz・libfuzzer-sys・lockfile・license を確認して固定する。上流現 manifest の候補は 0.4.13、license は `(MIT OR Apache-2.0) AND NCSA` で、まだ依存追加承認や導入をしたものではない。上記共通 TOML に elf_note/config の bin 定義も同じ flag で追加し、各 target file は共通の no_main/import と対応する上記 body を使う。

3 target は実 parser を直接使用する。CONFIG は本来の4096 byte／ASCII制約、ELF は実r1/r2 NOTE検査だけで実行・mapping無し、FAT32 は先頭1byteをsector-size selector（0=512、1=4096）として除いたpayload上限1MiB／512又は4096 byte sector／最大256 readの不変な仮想block読出しとする。上記雛形は計画だけで、本作業ではcompile／実行していない。seed は生成MBR／GPT／superfloppyと循環・overflow・重複名、C/Rust SDK ELFと不正NOTE、空／4096／4097 byte・CRLF・非ASCII・全loglevel CONFIG。実PAIRING.DAT／秘密鍵／私物fileを入れない。拒否は正常、crash／sanitizer／hang／境界違反は不合格。成功FAT32後のbounded read_at・exFAT・他policy対象は後続追加であり、この初期案で網羅したとは扱わない。

CI 案はLinux x86-64／各target1worker／ASan、各30秒・job全体5分・入力1件2秒・RSS256MiB。上記commandを使い、最大入力はFAT32 1MiB+selector1byte、ELF 65536、CONFIG4097 byte。失敗artifact／log／toolchain／seedを保存し、成功まで再実行して失敗を隠さない。短時間合格はcoverage証拠で安全証明ではない。既存host試験と決定的HID mutationは独立に維持する。参照は上記一次資料（2026-10-16確認）。コード・依存・CI変更、fuzz実行は本書では行っていない。
