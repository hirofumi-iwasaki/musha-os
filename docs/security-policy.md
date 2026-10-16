# Security policy — threat model, trust boundaries and practices

## English

Status: **owner decision on direction, 2026-10-16.** Items with a version tag are scheduled in that version's scope; the rest are standing rules. Related: [no-shell principle](no-shell-policy.md), [POSIX profile](posix-profile.md), [v0.5.0 scope](release-scope-0.5.0.md), [third-party policy](third-party.md), `SECURITY.md` at the repository root.

### What Musha-OS protects

1. **The user's drive.** Musha-OS writes only inside `SETTINGS/` (v0.4.0 goal 4). No input from the network, a USB device or an application may cause a write anywhere else.
2. **Ring 3 isolation.** A malicious or buggy `AUTORUN.ELF` must not read or write kernel memory, escape its arena, or make the kernel execute code of its choosing. The syscall layer validates every pointer and length against owned user mappings.
3. **Availability.** A crafted network packet, USB descriptor, Bluetooth PDU, file system structure or configuration file must not crash or hang the kernel; it is rejected with a logged reason.
4. **Authenticity of releases.** A user can verify that the EFI and archives they downloaded are the ones the project built (`SHA256SUMS`, reproducible builds; signatures as a later step).

### What Musha-OS does not protect against (by design)

- **Physical access.** Whoever holds the USB drive and the machine controls everything. The EFI is unsigned; Secure Boot support is a separate decision (section "Secure Boot").
- **Firmware.** UEFI firmware, Option ROMs and the Intel ME / AMD PSP are trusted as found.
- **DMA-capable attackers.** No IOMMU is used; a malicious PCIe/Thunderbolt device can read and write memory. USB devices cannot initiate DMA and are covered by the parser rules below.
- **Confidentiality.** There is no encryption and no user authentication. The only secrets the OS holds are Bluetooth link keys in `PAIRING.DAT`, stored in clear on the drive.
- **Being a server.** Musha-OS never offers network services beyond what a release explicitly lists (DHCP client; UDP echo when enabled).

### Trust boundaries

Trusted: the kernel image itself, firmware, and the OS-managed files on the drive (`EFI/`, `README.TXT`, `DOCS/`) as installed.

Untrusted — every byte is validated with explicit bounds before use:
- network input (lwIP, DHCP, UDP, later application sockets);
- USB devices (descriptors, HID reports, BOT responses, Bluetooth HCI events, ATT/SMP PDUs);
- drive contents that the user or an application may change (FAT32/exFAT structures, `AUTORUN.ELF`, `CONFIG.TXT`, `PAIRING.DAT`, data files);
- application syscall arguments.

Adding networking does not change the model: it adds one more untrusted parser.

### Practices

| # | Practice | Scope | Version |
|---|---|---|---|
| 1 | This threat model is published and kept current; every new input source is added to the untrusted list with its validation rules | docs | standing |
| 2 | **Fuzzing** of all host-testable parsers with `cargo-fuzz` (libFuzzer): FAT32/exFAT, ELF loader and NOTE, HID report descriptors, `CONFIG.TXT`, lwIP input path, HCI event / ATT / SMP decoders, `PAIRING.DAT`. Corpus and crash reproducers kept in the repository; fuzz jobs run in CI for a bounded time | tools, CI | v0.5.0 |
| 3 | **`unsafe` inventory**: every `unsafe` block carries a `// SAFETY:` comment stating the invariant; a CI script counts blocks per crate and fails when the count rises without a documented reason | code, CI | v0.5.0 |
| 4 | **CPU defenses verified at boot**: NX, W^X, SMEP, SMAP (with `stac`/`clac` around copy-in/out), stack guards, no user pages mapped executable; the `core:` line at `devices` reports which are active, so hardware records show them | kernel | v0.5.0 |
| 5 | **lwIP hygiene**: pinned version recorded in `third-party.md`, CVE list reviewed per release, unused protocols (TCP, IPv6, DNS unless needed) compiled out, no listening service besides DHCP; UDP echo off by default in published builds (`lan.echo=on` to enable) | net | v0.4.0 decision pending (echo default); rest v0.5.0 |
| 6 | **Supply chain**: `Cargo.lock` pinned (done), `cargo audit` and `cargo deny` in CI, third-party code vendored under the existing policy with recorded versions | CI | v0.5.0 |
| 7 | **Release authenticity**: reproducible builds and `SHA256SUMS` (done); detached signatures (minisign or OpenSSH signatures) on `SHA256SUMS` with the public key published in the repository and README | release | v0.5.0 candidate |
| 8 | **Disclosure**: `SECURITY.md` with a contact, the scope above, and the expectation of a response time; fixes are released as a patch version with a release note entry | repo | standing (file added now) |

### Secure Boot

Two ways exist to boot Musha-OS with Secure Boot enabled. They differ in cost and in who has to act.

**A. User-enrolled key (free, recommended first).** The project generates its own signing key and certificate, signs `BOOTX64.EFI` with `sbsign` (sbsigntools) or `sbctl`, and ships the certificate (`musha-os-secure-boot.cer`/`.auth`) in `DOCS/`. The user enrolls it once per machine from the firmware setup (most firmwares offer "enroll key from file" in Secure Boot custom mode) or with `KeyTool.efi`/`sbctl` from a Linux host. After that, Secure Boot stays on and Musha-OS boots. Costs nothing, works on NUC5, Ryzen boards and ThinkPads; does not apply to Intel Macs without T2 (no UEFI Secure Boot) nor to T2 Macs (Apple-only chain). Key handling (offline key, where it lives, who signs) must be documented. **Owner decision 2026-10-16: adopted as v0.5.0 goal 19.**

**B. Microsoft-signed shim (paid, slow, uncertain for a hobby OS).** Microsoft does not sign arbitrary third-party OS loaders; the established route is **shim**: a small loader signed by Microsoft after a public review at `rhboot/shim-review`, which then verifies the next stage with the vendor's own certificate embedded in it. Requirements as of 2026: a legal entity with proof of existence; an **EV code-signing certificate** (commercial CAs, on the order of a few hundred USD per year) to sign the submission to the Microsoft Hardware Dev Center; a reproducible shim build from the current release tarball; SBAT entries; two security contacts with PGP keys; an explanation of how the loaded OS enforces "lockdown" under Secure Boot; and a review that reviewers say takes two to three months and expects GRUB2/systemd-boot-like loaders — a custom OS needs extra justification. Since 27 June 2026 Microsoft signs shims only with the **UEFI CA 2023**, so a new shim boots only on firmware whose `db` already contains the 2023 certificate; older machines such as NUC5 may lack it unless a Windows update added it. For Musha-OS this route is out of scope until a legal entity and a reason exist; option A covers the reference machines.

**C. Not available.** Direct Microsoft signing of `BOOTX64.EFI` itself is not offered to projects outside the shim/OEM/IHV programs.

## 日本語

状態：**方向性は所有者決定（2026-10-16）。** 版の付いた項目はその版の範囲に入れ、残りは常時の規則。関連：[シェルを持たない原則](no-shell-policy.md)、[POSIX プロファイル](posix-profile.md)、[v0.5.0 範囲](release-scope-0.5.0.md)、[第三者コード方針](third-party.md)、リポジトリ直下の `SECURITY.md`。

### Musha-OS が守るもの

1. **利用者の USB。** Musha-OS は `SETTINGS/` 内にしか書かない（v0.4.0 目標 4）。ネットワーク・USB 機器・アプリからのどんな入力も、それ以外の場所への書込みを引き起こしてはならない。
2. **Ring 3 の分離。** 悪意ある、あるいは壊れた `AUTORUN.ELF` が、カーネルメモリーの読み書き、arena からの逸脱、カーネルに任意コードを実行させること、をできてはならない。syscall 層は全てのポインターと長さを所有済みのユーザーマッピングに対して検証する。
3. **可用性。** 細工したパケット、USB ディスクリプター、Bluetooth PDU、ファイルシステム構造、設定ファイルでカーネルが落ちたり止まったりしない。理由をログに出して拒否する。
4. **公開物の真正性。** 利用者が、ダウンロードした EFI とアーカイブがプロジェクトの作ったものだと検証できる（`SHA256SUMS`、再現可能ビルド。署名は後の段階）。

### 守らないもの（設計上）

- **物理アクセス。** USB と機体を手にした者が全てを支配する。EFI は未署名で、Secure Boot 対応は別の判断（「Secure Boot」節）。
- **ファームウェア。** UEFI、Option ROM、Intel ME／AMD PSP はそのまま信頼する。
- **DMA 可能な攻撃者。** IOMMU は使わない。悪意ある PCIe／Thunderbolt 機器はメモリーを読み書きできる。USB 機器は DMA を起こせず、下のパーサー規則の対象。
- **機密性。** 暗号化も利用者認証も無い。OS が持つ秘密は `PAIRING.DAT` の Bluetooth リンクキーだけで、USB に平文で置く。
- **サーバーになること。** 公開版が明示するもの（DHCP クライアント、有効時の UDP echo）以外のネットワークサービスを提供しない。

### 信頼境界

信頼する：カーネル画像自身、ファームウェア、インストール時の OS 管理ファイル（`EFI/`、`README.TXT`、`DOCS/`）。

信頼しない——使う前に全バイトを明示的な上限で検証する：
- ネットワーク入力（lwIP、DHCP、UDP、将来のアプリ向けソケット）
- USB 機器（ディスクリプター、HID レポート、BOT 応答、Bluetooth HCI イベント、ATT/SMP の PDU）
- 利用者やアプリが変えうる USB の内容（FAT32/exFAT 構造、`AUTORUN.ELF`、`CONFIG.TXT`、`PAIRING.DAT`、データファイル）
- アプリの syscall 引数

ネットワーク対応はこのモデルを変えない。信頼しないパーサーが 1 つ増えるだけである。

### 実践項目

| # | 項目 | 対象 | 版 |
|---|---|---|---|
| 1 | この脅威モデルを公開し維持する。新しい入力源は検証規則とともに「信頼しない」一覧に追加する | 文書 | 常時 |
| 2 | **ファジング**：ホストで試験できる全パーサーを `cargo-fuzz`（libFuzzer）で回す：FAT32/exFAT、ELF ローダーと NOTE、HID レポートディスクリプター、`CONFIG.TXT`、lwIP の入力経路、HCI イベント／ATT／SMP のデコーダー、`PAIRING.DAT`。コーパスとクラッシュ再現入力はリポジトリに保持し、CI で時間を区切って実行 | tools、CI | v0.5.0 |
| 3 | **`unsafe` の棚卸し**：全ての `unsafe` ブロックに不変条件を述べる `// SAFETY:` コメントを付け、CI のスクリプトがクレートごとの件数を数えて、理由無く増えたら失敗させる | コード、CI | v0.5.0 |
| 4 | **CPU 防御の起動時確認**：NX、W^X、SMEP、SMAP（copy-in/out の前後に `stac`/`clac`）、スタックガード、ユーザーページを実行可能で map しないこと。`devices` の `core:` 行に有効なものを出し、実機記録に残す | カーネル | v0.5.0 |
| 5 | **lwIP の衛生**：固定した版を `third-party.md` に記録、公開ごとに CVE 一覧を確認、使わないプロトコル（TCP、IPv6、必要になるまで DNS）をビルドから外す、DHCP 以外に待ち受けを作らない。公開版では UDP echo を既定で無効（`lan.echo=on` で有効） | net | echo の既定は v0.4.0 の所有者判断待ち。残りは v0.5.0 |
| 6 | **サプライチェーン**：`Cargo.lock` の固定（済）、`cargo audit` と `cargo deny` を CI に、第三者コードは既存方針で版を記録して取込み | CI | v0.5.0 |
| 7 | **公開物の真正性**：再現可能ビルドと `SHA256SUMS`（済）。`SHA256SUMS` への分離署名（minisign または OpenSSH 署名）と、公開鍵のリポジトリ・README への掲載 | 公開 | v0.5.0 候補 |
| 8 | **報告窓口**：`SECURITY.md` に連絡先、上記の範囲、応答の目安を書く。修正はパッチ版として公開しリリースノートに記載 | リポジトリ | 常時（ファイルは今回追加） |

### Secure Boot

Secure Boot を有効にしたまま Musha-OS を起動する方法は 2 つあり、費用と「誰が動くか」が違う。

**A. 利用者が鍵を登録する（無償。まずこれを推奨）。** プロジェクトが自前の署名鍵と証明書を作り、`BOOTX64.EFI` を `sbsign`（sbsigntools）か `sbctl` で署名し、証明書（`musha-os-secure-boot.cer`／`.auth`）を `DOCS/` に同梱する。利用者は機体ごとに 1 回、ファームウェア設定（多くのファームウェアは Secure Boot のカスタムモードに「ファイルから鍵を登録」がある）か、Linux ホストの `KeyTool.efi`／`sbctl` で登録する。以後は Secure Boot を有効にしたまま Musha-OS が起動する。費用は無く、NUC5・Ryzen のマザーボード・ThinkPad で使える。T2 の無い Intel Mac（UEFI Secure Boot 自体が無い）と T2 Mac（Apple 専用の連鎖）には適用できない。鍵の扱い（オフライン保管、所在、誰が署名するか）を文書化する。**所有者決定 2026-10-16：v0.5.0 の目標 19 として採用。**

**B. Microsoft 署名の shim（有償・時間がかかり・趣味 OS には不確実）。** Microsoft は任意の第三者 OS ローダーに署名しない。確立した経路は **shim**——`rhboot/shim-review` での公開レビュー後に Microsoft が署名する小さなローダーで、内蔵したベンダー証明書で次段を検証する。2026 年時点の要件：法的実在を証明できる組織、Microsoft Hardware Dev Center への提出物に署名する **EV コードサイニング証明書**（商用 CA。年額で数百米ドル規模）、現行リリース tarball からの再現可能な shim ビルド、SBAT エントリー、PGP 鍵を持つ 2 名のセキュリティ連絡先、ロードする OS が Secure Boot 下で「lockdown」をどう強制するかの説明、そしてレビュー担当が 2〜3 か月かかると言い、GRUB2／systemd-boot 系のローダーを前提にしているレビュー——独自 OS は追加の正当化が要る。2026 年 6 月 27 日以降、Microsoft は shim を **UEFI CA 2023** でのみ署名するため、新しい shim はファームウェアの `db` に 2023 証明書が既にある機体でしか起動しない。NUC5 のような古い機体は、Windows Update が追加していなければ持っていない可能性がある。Musha-OS では、法人と理由が揃うまで対象外とし、A で基準機を賄う。

**C. 無いもの。** `BOOTX64.EFI` そのものへの Microsoft の直接署名は、shim／OEM／IHV のプログラム外のプロジェクトには提供されていない。
