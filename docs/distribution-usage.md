# Distribution usage / 配布物の利用

## English

Status: current v0.4.0 development contract; physical/formal candidate acceptance remains pending. Published v0.3.0 downloads and evidence are unchanged. Bounded OS-owned SETTINGS records, startup configuration, C/Rust execution, r2 and viewer return are locally implemented. Bluetooth software paths exist, but target-adapter/controller compatibility is unverified. Only SETTINGS may be written by the OS; data/viewer/exFAT remain read-only.

### Drive and assets

The drive root is EFI/, DOCS/, empty SETTINGS/ and README.TXT; AUTORUN.ELF belongs to the user. DOCS contains LICENSE, NOTICE, third_party and sdk-guide.md, external-app.md, file-viewer.md, distribution-usage.md. README is ASCII/CRLF, under4 KiB and at most72 columns; its exact length/FNV hash are generated at build time. exFAT remains read-only data media; boot CONFIG support is FAT32-only; see [startup configuration](https://github.com/hirofumi-iwasaki/musha-os/blob/release/0.4.0/docs/startup-configuration.md) for the stage2 development implementation.

Future release names (no v prefix): musha-os-<ver>-x64.zip, musha-os-<ver>-x64-qemu.img.gz, musha-os-<ver>-c-sdk.zip and musha-os-<ver>-rust-sdk.zip, plus SHA256SUMS/build-manifest.json. docs.zip is allowed only after documentation stage2/goal17. QEMU image is64 MiB GPT/FAT32, for QEMU only. No deployment ZIP or candidate tar is published. C and Rust execution/wrappers/samples are implemented locally; each generated archive records its actual executable/sample status. Draft attachment remains blocked unless both SDK implementation/sample gates pass.

The simple USB extraction procedure is awaiting the [formatted-media gate](formatted-media-boot-verification.md); do not infer formatter/hardware acceptance from tool or QEMU tests. The README front page still describes published v0.3.0. Published files are never rebuilt or replaced by this work.

### Optional differential updater

Trust the release source and verify the downloaded ZIP against its published SHA256SUMS before use. Package metadata is not a signature. Schema3 metadata lives in the ZIP comment (maximum65,535 bytes), so ordinary extraction creates no manifest/state file. Only an explicit empty SETTINGS directory is serialized; metadata never manages, hashes or overwrites SETTINGS contents. Removing or truncating the comment makes the optional updater reject the package. build-manifest.json remains a separate release asset.

Generation accepts repeatable --previous-package paths to checksum-verified trusted prior drive/files ZIPs. Current and previous content hashes are carried in metadata; it does not trust an on-drive manifest. A managed/obsolete file with unknown or modified bytes is refused before backup/write. Missing files can be added, identical files skipped; known obsolete DIAGNOSE/MUSHA markers, both install JSON names, manifest and root license files are backed up outside the volume and removed. Unknown user files are retained; an unknown obsolete file requires owner review, not forced deletion. No force-overwrite switch exists.

Dry-run is default. The macOS physical adapter requires a writable external physical USB FAT32 volume,512-byte sectors and matching UUID for apply; directory mode refuses mounted volumes. Backup must be outside the target and on the host for physical media. After apply: verified-files/verified-absent in host backup/update-result.json, no USB state file. Existing SETTINGS contents remain untouched; the host updater may create its empty directory. It rechecks symlinks/case aliases, device identity and observed hashes and synchronizes file writes. Multi-file FAT32 updates are not power-failure atomic; on failure inspect host report and recover individually from verified backups. Do not retry by writing a whole image.

Directory simulation (not physical media):

```sh
python3 tools/update-usb.py development/musha-os-0.4.0-x64.zip --volume out/test-volume --backup out/test-backup --test-directory
```

The test directory must already contain EFI/BOOT/BOOTX64.EFI with current or known prior bytes; --apply modifies only this regular-directory simulation. No physical command is executed here. Hardware erase/update operations remain owner-run.

### Provenance and limits

Same-commit clean builds require every asset byte identical. Docs-only successors must preserve EFI, full QEMU image and each C/Rust sample ELF; VERSION/README generator/template edits are runtime edits. QEMU images currently embed DOCS: editing those pages changes image bytes, so this case fails docs-only equivalence and is not silently accepted. Owner decision2026-10-15: image-embedded documents/licenses are outside condition2 and require renewed acceptance; image-external docs qualify only with all runtime members identical. Local SDK execution does not by itself demonstrate complete accepted-candidate payload equivalence or pass a release gate.

Historical deployment instructions are preserved in [usage through v0.3.0](distribution-usage-through-0.3.0.md). Their old file names/state/layout belong to those releases.

## 日本語

状態：現行v0.4.0開発契約、実機/正式候補受入は未完。公開v0.3.0/証拠は保持。OS所有SETTINGS限定record、起動設定、C/Rust実行、r2、viewer復帰はローカル実装済み。BT software経路はあっても実adapter/controller互換は未検証。OS書込みはSETTINGS内だけでdata/viewer/exFATは読取り専用です。

### 媒体と配布物

rootはEFI/・DOCS/・空SETTINGS/・README.TXT、AUTORUN.ELFは利用者管理。DOCSはLICENSE/NOTICE/third_partyとsdk-guide/external-app/file-viewer/distribution-usage各md。READMEはASCII/CRLF、4 KiB未満/72桁以下で長さ/FNVをビルド生成。exFATは読出しデータ媒体、CONFIG起動経路はFAT32限定。工程2開発実装は[起動設定](https://github.com/hirofumi-iwasaki/musha-os/blob/release/0.4.0/docs/startup-configuration.md)を参照。

将来配布名はv無しのmusha-os-<ver>-x64.zip、-x64-qemu.img.gz、-c-sdk.zip、-rust-sdk.zipとSHA256SUMS/build-manifest.json。docs.zipは文書第2段/目標17後のみ。QEMU像は64 MiB GPT/FAT32の仮想用途のみ。deployment ZIP/candidate tarは公開しません。C/Rust実行・wrapper・sampleはローカル実装済みで、生成archiveは実際の実行/sample状態を記録。両SDK実装/サンプル条件を満たすまでdraft添付を拒否します。

簡易USB展開手順は[フォーマット媒体の節目](formatted-media-boot-verification.md)待ち。host/QEMUでformatter/実機合格を推論しません。README表紙は公開v0.3.0のまま。公開物を再ビルド・置換しません。

### 任意の差分更新

配布元を信頼確認しZIPを公開SHA256SUMSで照合。metadataは署名ではありません。schema3はZIP comment内（上限65,535byte）なので通常展開でmanifest/stateを作りません。空SETTINGSだけ明示し、内容の管理/hash/上書きは禁止。commentの削除/切捨てはupdater拒否。build-manifest.jsonは別assetです。

生成時--previous-packageを繰返し指定し、hash照合済みの信頼できる前版drive/files ZIPから現/前hashを収録。USB内manifestは信用しません。未知・利用者変更済みの管理/廃止fileは書込み・backup前に拒否。欠落は追加、同一はskip、既知の旧marker/両install JSON/manifest/root licenseはUSB外backup後削除。未知userfileは保持し未知旧fileは所有者レビューへ。強制上書きはありません。

dry-run既定。macOS物理adapterは外付け物理USB/FAT32/512byteとapply UUID照合が必要、directory modeはmountを拒否。backupは対象外、物理時host上。結果verified-files/verified-absentはhost backup/update-result.json、USB状態file無し。SETTINGS内容は不変、host updaterは空directory作成可。symlink/case alias/device/hash再検査とfile同期。複数file更新は停電原子的ではないため失敗reportと検証済backupで個別復旧し、全image書込みへ切替えません。

通常directoryの模擬試験（物理媒体ではない）：

```sh
python3 tools/update-usb.py development/musha-os-0.4.0-x64.zip --volume out/test-volume --backup out/test-backup --test-directory
```

既存EFI/BOOT/BOOTX64.EFIのbytesが現行または既知前版であることが必要。--applyはこの通常directoryだけに反映。ここで物理操作は実行しません。実媒体消去/更新は所有者実施。

### 来歴と制約

同一commit clean2buildは全asset一致。文書のみ後続はEFI/QEMU像全体/C・Rust各ELF一致。VERSION/README生成器/template変更はruntime変更。現QEMU像にDOCSを同梱するため、その文書変更は像bytesを変え、docs-only一致は不合格。所有者追加判断2026-10-15で像内文書/licenseは条件2外・再受入必要。像外文書も全runtime一致時のみ対象。SDKローカル実行だけで候補の全payload一致やrelease合格を証明しません。

過去手順は[v0.3.0までの利用手順](distribution-usage-through-0.3.0.md)へ内容を保持。旧名/state/layoutは当該版のものです。

## Host platform boundaries / host別境界

English: macOS is the recorded physical host-update route; see [macOS guide](usb-install-macos.md) for identification/dry-run and limitations. Linux/Windows physical-device identification/update/capacity-adjustment adapters and their physical host tests are not provided. Ordinary-file/directory simulation is available but does not establish real-device support. Do not raw-write a64MiB image to a large USB or infer a first-install writer from the FAT32 EFI placement contract. Windows Explorer FAT32 extraction is a separate [formatter gate](formatted-media-boot-verification.md), not updater support. The former platform notes remain in [Linux history](history/usb-install-linux.md) and [Windows history](history/usb-install-windows.md).

日本語：実host更新の記録経路はmacOSで[案内](usb-install-macos.md)に従い識別/dry-run/制限を確認。Linux/Windowsの実device識別・更新・容量調整adapterと実host試験は未提供。通常file/directory simulationは実USB対応の証拠ではありません。64MiB像を大容量USBへraw書込みせずFAT32上EFI配置から初回writer提供を推定しません。Explorer FAT32展開は[formatter gate](formatted-media-boot-verification.md)でupdater対応とは別。旧platform文書は[Linux履歴](history/usb-install-linux.md)・[Windows履歴](history/usb-install-windows.md)に保持。

`--docs-complete` is an explicit owner assertion after documentation acceptance, not an automatic index-presence verdict. The packager also requires the index, but cannot judge translation quality, README hardware prerequisites or evidence completeness. Keep it false until those gates are accepted. / `--docs-complete`は文書受入後の所有者明示判断で索引の存在だけによる自動完了判定ではありません。packagerは索引存在も要求しますが翻訳品質/README実機前提/証拠完備は判定できず、受入までfalseを維持します。
