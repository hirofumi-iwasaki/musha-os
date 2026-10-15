# Normal-build LAN enablement proposal / 通常版LAN有効化案

Status: current

## English

**Proposal only; owner approval required before changing defaults.** Keep the normal82574 path and shared DHCP/UDP implementation. Recommend enabling I218-V in normal builds only after the NUC5 optional-feature trial demonstrates reset/link/lease/echo and safe restart/stop. Its current software LCD/OEM refusal stays explicit; an unsupported firmware configuration must not be bypassed to obtain a ready label. The release's physical LAN gate remains open.

Keep Realtek runtime writes/DMA closed. Expected PCI10EC:8168 and the ASUS L8200A label do not identify a safe RTL8111H initialization profile. The independent host prototype exercises reset, descriptor ownership/FCS bounds and conservative stop, but is not a PHY/OCP driver or a working hardware transport. It deliberately cannot authorize hardware writes, and BME-off alone never supplies a DMA-drain/reuse proof.

For admission review the owner supplies: exact board/NIC identification; `devices` PCI vendor:device:revision and BDF; subsystem IDs; BAR type/base/length and PCI command; raw read-only TxConfig(0x40) from a reviewed driver-assisted register capture, with tool/version and source of the capture. Do not guess MMIO addresses or use arbitrary register-write tools. PCI revision and TxConfig must be considered together. Current `devices` records PCI identity, not this Realtek register snapshot.

Next review must select the identified MAC/PHY variant, establish firmware/OCP requirements and a source-independent implementation, and specify reset/link/DMA-stop/drain ordering. Then connect the Realtek frame adapter to existing lwIP DHCP/UDP and stage5 reinitialization, run host/negative tests and existing82574 regression, and trial the opt-in feature on Ryzen. Only after that evidence should a separate approved change enable Realtek normally. Neither QEMU's RTL8139 nor82574 proves RTL8111H operation. No new application network API is proposed. NUC11 I225-LM remains a separate future track, not an acceptance machine.


Startup impact is unmeasured. Existing limits are link polling3s and asynchronous no-address/DHCP30s, managed ULP300ms, plus bounded PHY/reset operations. Flash can wait10s idle plus10s per read cycle; up to two signature and64 checksum reads mean a conservative flash-only1320s sum, not a practical normal-boot guarantee. Therefore default enablement should additionally require an approved aggregate initialization budget (proposed5s, new implementation required), with failure disabling LAN rather than blocking the viewer. Do not advertise a five-second bound before implementing it. Record PM-epoch LAN start/finish, first viewer/ready and DHCP lease time on NUC5/Ryzen, comparing absent/link-down/no-server/cold/restart runs against the same default-disabled EFI.

Display proposal: `phases` reports LAN unsupported/initialization failed/lease pending with the concrete reason; `devices` adds measured PCI/revision, selected backend, link/capacity/address and PM-epoch durations. Ready never means DHCP succeeded before a lease; a missing or unmatched PCI device is skipped without touching NIC registers, and LAN absence preserves USB/viewer/app operation. Existing required-failure visibility remains; no new fatal LAN requirement is introduced by this proposal.

## 日本語

**提案のみです。既定変更は所有者承認後に実装します。** 通常82574と共有DHCP/UDPを維持し、NUC5で任意feature版のreset/link/lease/echoと安全な再起動/停止を確認してからI218-Vを通常版へ有効化する案を推奨します。software LCD/OEM拒否は維持し、ready表示のために未対応firmware構成を迂回しません。公開の実機LAN gateは未完です。

Realtekのruntime書込み/DMA gateは閉じたままにします。見込みPCI10EC:8168やASUS L8200A表記では安全なRTL8111H初期化profileを識別できません。独自host prototypeはreset・descriptor所有権/FCS範囲・保守的停止を検査しますが、PHY/OCP driverや実機通信経路ではありません。hardware書込みを承認する経路は無く、BME-offだけでDMA drain/再利用安全を成功扱いしません。

gate reviewには所有者からboard/NICの正確な識別、`devices`のPCI vendor:device:revision/BDF、subsystem ID、BAR種別/base/長さ・PCI command、review済みdriver経由の読取り専用TxConfig(0x40)採取値とtool/版/採取元が必要です。MMIOアドレスを推測したり任意register書込みtoolを使いません。PCI revisionとTxConfigを合わせて照合します。現devicesはPCI識別を表示しますがRealtek register snapshotは表示しません。

次に特定MAC/PHY variant、firmware/OCP要件、コードコピーを行わない実装とreset/link/DMA停止/drain順をreviewします。その後Realtek frame adapterを既存lwIP DHCP/UDPと工程5再初期化へ接続し、host/負例・82574回帰、Ryzen任意feature実機試験を実施。証拠が揃ってから別の承認済み変更で通常有効化します。QEMUのRTL8139/82574はRTL8111Hの証明ではありません。アプリ通信API追加は提案しません。NUC11 I225-LMは別の将来trackで、受入機種へ追加しません。

起動時間の増分は未測定です。既存上限はlink poll3秒、非同期DHCP/アドレス無し30秒、管理ULP300msと各PHY/resetの有界処理です。flashは1読取りでidle10秒＋cycle10秒、署名最大2回＋checksum64回ならflashだけの保守的加算1320秒で、実用的な通常起動保証にはなりません。従って既定有効化には別途、初期化全体budget（提案5秒、要新実装/承認）を設け、失敗時はLAN無効化でviewerを進める案です。実装前に5秒保証を表示しません。同じ既定無効EFIと比較し、NUC5/RyzenのLAN開始/終了PM epoch、最初のviewer/ready、lease時刻を、NIC不在/link断/server不在/cold/restartで記録します。

表示案：phasesはLAN未対応/初期化失敗/lease待ちと理由、devicesは実測PCI/revision・backend・link/容量/address・PM epoch所要を表示。lease前にDHCP成功をreadyとしません。PCI不一致/不在はNIC register未接触でskipし、USB/viewer/appを継続します。既存の必須失敗表示を保持し、この提案でLAN不在を新たなfatal条件にはしません。
