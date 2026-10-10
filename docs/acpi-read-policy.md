# ACPI firmware-table read policy / ACPIファームウェア表の読取方針

## English

H9 decision, 2026-10-10: IMG_7449 stopped before reading the 20-byte RSDP. The overlapping descriptor is ACPI Reclaim Memory (type 9), attributes 0x2600F. Audit confirms bytes() rejected its RP capability; checksum validation had not run. FADT zero fields are unvalidated evidence, not absent hardware.

UEFI GetMemoryMap Attribute describes capabilities, not necessarily current settings. Source: [UEFI GetMemoryMap](https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html#getmemorymap). Firmware configuration-table data is consumed before ExitBootServices, using the firmware's mappings. This differs from the H7 post-exit RAM mapping policy: we do not install new mappings, alter permissions, or write firmware data. The firmware configuration-table contract supplies readable data; capability bits alone cannot disprove that contract. This change does not independently probe current hardware page permissions or recover from faulty firmware mappings.

Remove only the RP capability veto from the bounded ACPI read-range predicate. Retain nonzero address, nonempty/overflow-safe range, full containment in one descriptor, types LoaderData/BootServicesData/RuntimeServicesData/ACPIReclaim/ACPINVS (2/4/6/9/10). RuntimeServicesData remains valid for pre-exit configuration tables; no firmware pointer or slice survives into runtime. Reject unknown/reserved/MMIO ranges and partial coverage. Keep all signatures, checksums, lengths, root layout, duplicate-FADT and PM timer policy checks. No Apple-specific exception or alternate timer.

Split diagnostic stages into memory-read policy, signature, checksum and length checks for RSDP and table parsing. Add a scalar result reason alongside the checkpoint. Pure host tests verify the real predicate accepts type 9 with 0x2600F, rejects unsupported types/partial coverage, and distinguishes signature/checksum failures. Run normal QEMU PM timer, keyboard and FAT32 checks, then build H9 with features=[].

Update only BOOTX64.EFI on the uniquely identified dedicated SanDisk. Preserve GPT/FAT32 and MUSHA.TXT. Back up old EFI, synchronize, verify filesystem, remount read-only, read back and compare exact bytes/SHA-256, then eject. The next Mac photo decides whether root/FADT discovery advances or a different measured condition remains.

## 日本語

H9方針（2026-10-10）：IMG_7449では20バイトのRSDPを読む前に停止。重なる記述子はACPI Reclaim Memory（type 9）、属性0x2600F。bytes()がRP能力ビットを拒否し、checksum検査には未到達。FADTの0表示は未検証であり、機器不在を意味しない。

UEFI GetMemoryMapのAttributeは能力を表し、現在の設定とは限らない。上記仕様を根拠とする。ACPI表はExitBootServices前にfirmwareのmappingで読む。H7の終了後RAMとは異なり、mappingや権限の変更、firmwareデータへの書込みは行わない。configuration-table契約により読み取るデータであり、能力ビットだけでは読取不可を判断できない。ただし現行ハードウェアのページ権限を独立測定したり、壊れたfirmware mappingから回復する変更ではない。

範囲検査からRP能力による拒否だけを除去。非ゼロ・非空・overflow安全な範囲、単一記述子による全被覆、type 2/4/6/9/10を維持。RuntimeServicesDataは終了前の表として許可済みであり、runtimeへ表のpointer/sliceは保持しない。未知・予約・MMIO・部分被覆を拒否。署名、checksum、長さ、root構造、FADT重複、PMタイマー条件を維持。Apple専用例外や代替タイマーは追加しない。

診断をメモリー読取方針・署名・checksum・長さに分離し、段階とは別に固定長の理由を保存。実際の純粋判定関数をホストテストし、type 9/0x2600Fの許可、非対応type・部分被覆の拒否、署名とchecksumの区別を確認。QEMU通常タイマー・キーボード・FAT32を検証し、features=[]のH9を作成する。

専用SanDiskを一意に照合してEFIのみ更新。GPT/FAT32・MUSHA.TXTを維持。旧EFI保存、同期、filesystem検査、読取専用再mount、読戻しbyte/SHA-256一致、安全な取り外しを行う。次のMac写真でroot/FADTへ進んだかを判断する。

## Validation / 検証

16 platform tests passed, including the three new read-range/RSDP tests. QEMU normal boot with two controllers and keyboard on the second passed PM timer, arena, storage/FAT32, key input and Esc cleanup. Normal H9 features=[] build passed. Mac physical validation remains pending.

platformテスト16件（新規範囲・RSDPテスト3件含む）が成功。2コントローラー・第2側キーボードのQEMU通常起動でPMタイマー、arena、storage/FAT32、入力とEsc終了を確認。features=[]の通常H9ビルド成功。Mac実機結果は次回確認する。
