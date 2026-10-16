# User-enrolled Secure Boot research / 利用者登録鍵によるSecure Boot調査

Status: research (2026-10-16). v0.5.0 mandatory goal19, option A in [owner security policy](security-policy.md); Microsoft-signed shim is outside scope. No key generation, signing, enrollment, firmware/USB change or Secure Boot boot test was performed.

## English

### Signing and provenance — proposed offline procedure

An offline signer can create a private key and self-signed certificate with OpenSSL `req -x509 -newkey rsa:2048 -sha256`, then export the public certificate with `openssl x509 -outform DER` as `DOCS/musha-os-secure-boot.cer`. Keep the private key encrypted/offline; key custody, certificate extensions/expiry, rotation and recovery still need a release procedure. These are research commands, not executed steps. [OpenSSL req](https://docs.openssl.org/3.2/man1/openssl-req/), [x509](https://docs.openssl.org/master/man1/openssl-x509/).

Build reproducible **unsigned** EFI first; retain its SHA256/source/toolchain/features and normal reproducibility comparison. Sign a separate copy with `sbsign --key db.key --cert db.crt --output BOOTX64.signed.EFI BOOTX64.unsigned.EFI`; verify with `sbverify --cert db.crt BOOTX64.signed.EFI`. Record both file hashes, certificate fingerprint and signing-tool version. Signature insertion changes whole-file bytes; do not compare signed/unsigned SHA256 for equality or assume repeat signing is deterministic. Verification against a certificate does not establish firmware enrollment or dbx acceptance. [sbsigntools upstream](https://kernel.googlesource.com/pub/scm/linux/kernel/git/jejb/sbsigntools/), [UEFI image signing §32.2](https://uefi.org/specs/UEFI/2.10/32_Secure_Boot_and_Driver_Signing.html?highlight=authenticated+variable).

`.cer` is an X.509 certificate; `.esl` is a signature list; `.auth` is a timestamped authenticated variable update. Proposed db append: `cert-to-efi-sig-list -g <owner-guid> db.crt db.esl`, then `sign-efi-sig-list -a -c KEK.crt -k KEK.key db db.esl db.auth`. User mode requires an already authorized KEK or PK; the project image key cannot authorize its own admission into an OEM database. Therefore a universal project `.auth` is not automatically installable: document physical-presence/Setup Mode enrollment or the machine owner's KEK-authorized update, preserving other trusted boot chains. PK removal enters Setup Mode, which is **not** enabled signature enforcement. [efitools manual](https://github.com/mjg59/efitools/blob/master/doc/sign-efi-sig-list.1.in), [UEFI §32.5.3](https://uefi.org/specs/UEFI/2.10/32_Secure_Boot_and_Driver_Signing.html?highlight=authenticated+variable).

### Firmware evidence — not tested compatibility

| Family | Primary documentation and remaining check |
|---|---|
| NUC5 Visual BIOS | [Intel Visual BIOS glossary](https://www.intel.com/content/dam/support/us/en/documents/intel-nuc/NUC-Visual-BIOS-Glossary.pdf) documents Custom/Standard, PK generation and database clearing. It does **not** establish a file-enrollment menu on this NUC5. Capture exact model/BIOS revision, db import formats and a recovery/export route before choosing firmware UI versus a trusted KeyTool/Linux enrollment path. |
| ASUS TUF GAMING X570-PLUS | [X570 series BIOS manual, Key Management](https://dlcdnets.asus.com/pub/ASUS/mb/SocketAM4/PRIME_X570-P/E15829_PRIME_PRO_TUF_GAMING_X570_Series_BIOS_EM_WEB.pdf) lists save/set/append db and time-authenticated variable files. Exact installed BIOS, accepted file format and append behavior remain unverified; a DER certificate is not automatically the required `.auth` file. |
| ThinkPad | [Lenovo X250 security white paper, Secure Boot](https://download.lenovo.com/pccbbs/mobiles_pdf/lenovo_thinkpad_x250_bios_security_white_paper.pdf) distinguishes User/Setup and Standard/Custom modes and factory restoration. It is an example, not proof for the unselected reference ThinkPad; model/BIOS and custom-enrollment access must be checked. |

### OVMF verification proposal

Use disposable NVRAM, an X64 OVMF build with `SECURE_BOOT_ENABLE` and protected variables, e.g. the documented Q35/SMM+pflash route (`smm=on`, secure pflash, readonly CODE, writable **copy** of matching VARS). Enroll test PK/KEK/db; confirm `SecureBoot=1` and `SetupMode=0`, then boot signed EFI and reject the otherwise identical unsigned EFI from the same medium. Capture firmware refusal plus absence of kernel entry, not just a black screen; retain firmware/QEMU versions, NVRAM hashes and certificate fingerprint. [OVMF requirements](https://raw.githubusercontent.com/tianocore/edk2/master/OvmfPkg/README), [QEMU variable protection](https://www.qemu.org/docs/master/devel/uefi-vars.html).

Local read-only inventory found secure CODE/VARS files and Q35 `smm` support, but no `sbsign`, `sbverify` or efitools in PATH; filename alone does not prove matching secure firmware or enrolled keys. This test needs a pinned signing/efitools environment and verified firmware pairing, then separate physical signed-pass/unsigned-reject evidence on all three reference machines. All remain unverified. Secure Boot authenticates the EFI chain; it does not make untrusted `AUTORUN.ELF` trusted.

## 日本語

### 署名と来歴 — オフライン手順案

OpenSSLの`req -x509 -newkey rsa:2048 -sha256`で秘密鍵と自己署名証明書を作り、`openssl x509 -outform DER`で公開証明書を`DOCS/musha-os-secure-boot.cer`へ出す案です。秘密鍵は暗号化してオフライン保管し、保管責任・証明書拡張/期限・更新・復旧を公開手順に定めます。今回はコマンドを実行していません。[req](https://docs.openssl.org/3.2/man1/openssl-req/)、[x509](https://docs.openssl.org/master/man1/openssl-x509/)。

再現可能な**未署名EFI**を先に作り、SHA256/source/toolchain/featuresと通常の再現性比較を保持。別copyを`sbsign --key db.key --cert db.crt --output BOOTX64.signed.EFI BOOTX64.unsigned.EFI`で署名し、`sbverify --cert db.crt BOOTX64.signed.EFI`で確認します。両file hash・証明書fingerprint・署名tool版を記録。署名でfile bytesは変わるため、署名済/未署名hashの一致や署名工程の決定性を仮定しません。証明書指定の検証はfirmware登録/dbx受理の証明ではありません。[上流tool](https://kernel.googlesource.com/pub/scm/linux/kernel/git/jejb/sbsigntools/)、[UEFI §32.2](https://uefi.org/specs/UEFI/2.10/32_Secure_Boot_and_Driver_Signing.html?highlight=authenticated+variable)。

`.cer`は証明書、`.esl`は署名一覧、`.auth`は時刻付き認証変数更新です。db追記案は`cert-to-efi-sig-list -g <owner-guid> db.crt db.esl`→`sign-efi-sig-list -a -c KEK.crt -k KEK.key db db.esl db.auth`。User Modeでは既存の認可KEK又はPKが必要で、projectのimage鍵だけではOEM dbへ自己登録できません。共通`.auth`が全PCで使えるとはせず、物理操作/Setup Mode登録又は所有者KEKによる更新を機種別に定め、他OSの信頼経路を保持します。PKを消したSetup Modeは署名強制が有効な状態ではありません。[efitools](https://github.com/mjg59/efitools/blob/master/doc/sign-efi-sig-list.1.in)、[UEFI §32.5.3](https://uefi.org/specs/UEFI/2.10/32_Secure_Boot_and_Driver_Signing.html?highlight=authenticated+variable)。

### firmware一次資料 — 実機対応の確認ではない

| 系統 | 資料と未確認点 |
|---|---|
| NUC5 Visual BIOS | [Intel glossary](https://www.intel.com/content/dam/support/us/en/documents/intel-nuc/NUC-Visual-BIOS-Glossary.pdf)はCustom/Standard・PK生成・db消去を説明しますが、このNUC5のfile登録画面は証明しません。正確なmodel/BIOS版、db取込形式、export/復旧経路を採り、UI又は信頼済KeyTool/Linux経路を選びます。 |
| ASUS TUF GAMING X570-PLUS | [X570 BIOS manualのKey Management](https://dlcdnets.asus.com/pub/ASUS/mb/SocketAM4/PRIME_X570-P/E15829_PRIME_PRO_TUF_GAMING_X570_Series_BIOS_EM_WEB.pdf)はdb保存/設定/追記と時刻付き認証変数fileを記載。搭載BIOS・受理形式・追記動作は未検証で、DER証明書を要求された.authと同一視しません。 |
| ThinkPad | [X250白書のSecure Boot](https://download.lenovo.com/pccbbs/mobiles_pdf/lenovo_thinkpad_x250_bios_security_white_paper.pdf)はUser/Setup・Standard/Custom・factory復元を説明。未選定の基準ThinkPadへ一般化せず、型番/BIOSとcustom登録権限を確認します。 |

### OVMF検証案

使い捨てNVRAMとSECURE_BOOT_ENABLE有効X64 OVMF、保護された変数領域を使います。例は公式Q35/SMM+pflash構成（smm=on、secure pflash、CODE読取専用、対応VARSの**copy**を書込可能）。試験PK/KEK/db登録後、SecureBoot=1・SetupMode=0を確認し、同媒体の署名済EFI成功/同じ未署名EFI拒否を採取。黒画面だけでなくfirmware拒否とkernel未到達を確認し、firmware/QEMU版・NVRAM hash・証明書fingerprintを保持します。[OVMF](https://raw.githubusercontent.com/tianocore/edk2/master/OvmfPkg/README)、[QEMU変数保護](https://www.qemu.org/docs/master/devel/uefi-vars.html)。

local読取調査ではsecure CODE/VARS fileとQ35 smm項目を確認しましたが、PATHにsbsign/sbverify/efitoolsはありません。filenameだけで対応firmware組合せや鍵登録を証明しません。固定版署名/efitools環境とfirmware整合確認が必要で、続いて基準3台の実署名成功/未署名拒否を別に採取します。いずれも未検証です。Secure BootのEFI認証でAUTORUN.ELFまで信頼済みにはなりません。
