# Bluetooth crypto dependency ledger

Status: current

## English

Normal boot enables internal bluetooth-security. RustCrypto AES/CMAC/P-256, subtle and zeroize are pinned; no primitive is invented. This is not certification or an audit of the new SMP/runtime. See [stage6 checkpoint](stage6-0.4.0-progress.md).

| Package | Version | Original terms | Selected |
|---|---|---|---|
| aes | 0.8.4 | MIT OR Apache-2.0 | MIT |
| base16ct | 0.2.0 | Apache-2.0 OR MIT | MIT |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | MIT |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | MIT |
| cipher | 0.4.4 | MIT OR Apache-2.0 | MIT |
| cmac | 0.7.2 | MIT OR Apache-2.0 | MIT |
| const-oid | 0.9.6 | Apache-2.0 OR MIT | MIT |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | MIT |
| crypto-bigint | 0.5.5 | Apache-2.0 OR MIT | MIT |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 | MIT |
| dbl | 0.3.2 | MIT OR Apache-2.0 | MIT |
| der | 0.7.10 | Apache-2.0 OR MIT | MIT |
| digest | 0.10.7 | MIT OR Apache-2.0 | MIT |
| elliptic-curve | 0.13.8 | Apache-2.0 OR MIT | MIT |
| ff | 0.13.1 | MIT/Apache-2.0 | MIT |
| generic-array | 0.14.7 | MIT | MIT |
| group | 0.13.0 | MIT/Apache-2.0 | MIT |
| hkdf | 0.12.4 | MIT OR Apache-2.0 | MIT |
| hmac | 0.12.1 | MIT OR Apache-2.0 | MIT |
| inout | 0.1.4 | MIT OR Apache-2.0 | MIT |
| p256 | 0.13.2 | Apache-2.0 OR MIT | MIT |
| primeorder | 0.13.6 | Apache-2.0 OR MIT | MIT |
| rand_core | 0.6.4 | MIT OR Apache-2.0 | MIT |
| sec1 | 0.7.3 | Apache-2.0 OR MIT | MIT |
| subtle | 2.6.1 | BSD-3-Clause | BSD-3-Clause |
| typenum | 1.20.1 | MIT OR Apache-2.0 | MIT |
| version_check | 0.9.5 | MIT/Apache-2.0 | MIT |
| zeroize | 1.8.2 | Apache-2.0 OR MIT | MIT |

These28 packages form the activated UEFI dependency closure rooted at AES/CMAC/P-256/subtle/zeroize, including build helpers. Not every package links into every target. Individual Cargo metadata and original terms were inspected: MIT where offered, subtle BSD-3-Clause, generic-array MIT only. Additional group/rand_core COPYRIGHT files are retained. Original text, source URLs/checksums and hashes are in [COPYING](../third_party/bluetooth-crypto-licenses/COPYING), [UPSTREAM.json](../third_party/bluetooth-crypto-licenses/UPSTREAM.json) and [SHA256SUMS](../third_party/bluetooth-crypto-licenses/SHA256SUMS). Cargo.lock pins source; no implementation source is vendored/relicensed.

Distribution drive/image DOCS now includes all three files and NOTICE points to these terms; build-esp stages COPYING alongside the binary. Retain the materials with binaries. Development packaging fixtures pass; no public asset was generated. C/Rust SDK public APIs do not export this internal stack.

Primary sources checked2026-10-15: [p2560.13.2](https://docs.rs/crate/p256/0.13.2/source/), [cmac0.7.2](https://docs.rs/crate/cmac/0.7.2/source/), [zeroize1.8.2](https://docs.rs/crate/zeroize/1.8.2/source/), [Core5.4 SMP](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-54/out/en/host/security-manager-specification.html), [official P-256 samples](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-54/out/en/br-edr-controller/sample-data.html), [UEFI RNG](https://uefi.org/specs/UEFI/2.9_A/37_Secure_Technologies.html). Specifications are design references, not copied implementation/qualification. Official ECDH/f4/f5/f6/g2/CMAC known answers are fixtures; no fixed random/scalar provider enters normal boot.

[zkSecurity2025-04-07](https://reports.zksecurity.xyz/reports/near-p256/) covers commit32343a78f1522aa5bd856556f114053d4bb938e0, primarily32-bit arithmetic/ECDSA verification. ECDH,64-bit-specific implementation and constant-time analysis were outside its focus. p2560.13.2 is not that exact revision. **Do not claim audited ECDH/x86-64/new SMP.** Public-release security review remains open. SecretKey/SharedSecret Drop and explicit scalar zeroize were inspected locally; this does not guarantee erasure of every compiler temporary/register.

## 日本語

通常bootで内部bluetooth-securityを有効化しました。固定版RustCrypto AES/CMAC/P-256、subtle、zeroizeを利用し暗号primitiveは自作しません。新SMP/runtimeの監査・認証ではありません。[工程6記録](stage6-0.4.0-progress.md)参照。

上表28依存はAES/CMAC/P-256/subtle/zeroizeを起点とするUEFI向け依存とbuild helperで、全packageが全targetへlinkされるわけではありません。各Cargo metadataと原licenseを確認し、選択可能な場合MIT、subtleはBSD-3-Clause、generic-arrayはMITを採用。group/rand_coreの追加COPYRIGHTも原文保持。COPYING/UPSTREAM.json/SHA256SUMSに採用条件・source URL・checksum/hashを記録しCargo.lockで固定。実装sourceのvendor/再licenseは行いません。

配布toolは原license一式をdrive/imageのDOCSへ含めNOTICEも参照。build-espもbinaryとCOPYINGを配置します。binary配布時はこれらを保持してください。開発用package fixtureで確認し公開assetは未生成。C/Rust SDKの公開APIへ内部stackは追加しません。

上記一次仕様と固定sourceを2026-10-15に確認。SIG/UEFIは設計参照で実装コピー/認証ではありません。公式ECDH/f4/f5/f6/g2/CMAC既知解を試験し、固定乱数/scalarはfixtureだけで本番bootには使いません。

zkSecurityの2025-04-07報告は上記固定commitの主に32bit演算/ECDSA検証を対象とし、ECDH/64bit固有実装/定数時間解析はfocus外。p2560.13.2も同一監査revisionではありません。**今回のECDH/x86-64や新SMPが監査済みとは主張しません。** 公開前security reviewは残件。SecretKey/SharedSecret Dropとworking scalarの明示zeroizeを原sourceで確認しましたが、全compiler一時値/registerの消去保証ではありません。
