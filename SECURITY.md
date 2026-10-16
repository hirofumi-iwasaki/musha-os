# Security Policy / セキュリティポリシー

## English

### Reporting a vulnerability

Please report security issues privately to **hirofumi.iwasaki@mushagaeshi.com** with the subject `Musha-OS security`. Include the release version or commit, the machine or QEMU configuration, and steps or a file that reproduces the problem. Do not open a public issue for an unfixed vulnerability.

You can expect an acknowledgement within 7 days. Confirmed issues are fixed in a patch release with an entry in the release notes; the reporter is credited unless they prefer not to be.

### Scope

Musha-OS is a single-application bare-metal environment. The threat model, trust boundaries and what the project does and does not defend against are in [docs/security-policy.md](docs/security-policy.md). In short, the project treats as vulnerabilities:

- any input from the network, a USB or Bluetooth device, the drive contents or an application that causes a write outside `SETTINGS/`, a kernel crash or hang, or an escape from Ring 3;
- any way for a downloaded release to differ from the project's reproducible build without `SHA256SUMS` detecting it.

Out of scope: attacks requiring physical access to the machine or the drive, firmware and DMA-capable PCIe/Thunderbolt devices, and the absence of encryption or user authentication, which are design decisions.

### Supported versions

Only the latest published release receives security fixes.

## 日本語

### 脆弱性の報告

セキュリティに関する問題は、件名を `Musha-OS security` として **hirofumi.iwasaki@mushagaeshi.com** に非公開で報告してください。公開版の版番号またはコミット、機体または QEMU の構成、再現手順か再現するファイルを添えてください。未修正の脆弱性を公開の issue にしないでください。

7 日以内に受領の連絡をします。確認された問題はパッチ版で修正し、リリースノートに記載します。報告者は、希望しない場合を除き、クレジットします。

### 範囲

Musha-OS は単一アプリのベアメタル実行環境です。脅威モデル、信頼境界、守るもの・守らないものは [docs/security-policy.md](docs/security-policy.md) にあります。要約すると、次を脆弱性として扱います。

- ネットワーク、USB／Bluetooth 機器、USB の内容、アプリからの入力によって、`SETTINGS/` 外への書込み、カーネルのクラッシュや停止、Ring 3 からの脱出が起きること。
- ダウンロードした公開物がプロジェクトの再現可能ビルドと異なるのに `SHA256SUMS` で検出できないこと。

対象外：機体や USB への物理アクセスを要する攻撃、ファームウェアと DMA 可能な PCIe／Thunderbolt 機器、暗号化や利用者認証が無いこと（設計上の判断）。

### 対象の版

セキュリティ修正は最新の公開版にのみ提供します。
