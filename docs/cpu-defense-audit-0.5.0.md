# CPU defense audit / CPU 防御調査 — v0.5.0 preparation

Status: static research (2026-10-16), source HEAD `2c3598175510b6c740e761dbc562574fbd038863`; no code, CPU state or hardware test changed. Owner direction: [security policy](security-policy.md). Source locations below identify this snapshot, not a future moving-line guarantee.

## English

| Defense | Current source evidence and boundary |
|---|---|
| NX / XD | [CPU support check](../boot/src/memory.rs#L224) rejects missing NX; [EFER.NXE write](../boot/src/memory.rs#L598) sets bit11 before OS CR3. [Kernel leaf builder](../boot/src/memory.rs#L60) sets NX unless explicitly executable; GOP/DMA RW/NX leaves are read back. NX enablement has no dedicated general boot read-back report. Pre-EBS EFI mappings remain firmware-controlled. |
| W^X + write protection | [PE image validation](../memory/src/lib.rs#L245) rejects writable-executable/overlapping sections; kernel sections receive their own permissions. [CR0.WP](../boot/src/memory.rs#L603) is set. [ELF parser](../memory/src/elf.rs#L94) accepts only R/RX/RW flags; [user leaf gate](../boot/src/memory.rs#L657) rejects writable+executable. This is per virtual mapping: user text still has a supervisor RW/NX backing alias in the arena, so global physical W^X/immutable user code is not established. |
| User code versus data | [ELF mapping](../boot/src/ring3/elf.rs#L83) deliberately makes file code RX; arena/stack/start-record are NX. Therefore literal “no user pages executable” would prevent supported Ring3 ELF code: the policy needs executable user code distinguished from non-executable user data, without claiming that clarification is already adopted. |
| SMEP | No OS-owned CPUID admission/set/read-back of CR4 bit20 was found. [CR4 updates](../boot/src/memory.rs#L238) preserve inherited bits except specified LA57/PCID rejection and PGE clearing. Thus **not enabled/verified by this OS**, rather than a guarantee that every firmware leaves it disabled. Protection against kernel instruction fetch from U/S=1 mappings is not established here. |
| SMAP / copies | Likewise no explicit CR4 bit21 enablement/verification and no `stac`/`clac` in first-party boot Rust/assembly. [SYS_LOG copy-in](../boot/src/ring3/elf.rs#L180) translates bounded owned VA ranges to permanent supervisor aliases; [SYS_INPUT copy-out](../boot/src/ring3/elf.rs#L241) checks all writable destinations before dequeuing and writes through aliases. [C1 log](../boot/src/ring3.rs#L259) uses the same alias principle. These are not raw user-VA accesses, so inserting STAC mechanically would not prove safety; direct-user access, AC cleanup, exception/interrupt entry and aliases require one reviewed design. |
| Stack guards | [Runtime kernel stack](../boot/src/memory.rs#L575) omits its first4KiB; [Ring3 supervisor stack](../boot/src/ring3.rs#L120) removes a guard leaf; [ELF user stack](../boot/src/ring3/elf.rs#L54) checks vacant stack+guard but maps only stack. [TSS/IST](../boot/src/cpu.rs#L94) uses two emergency stack regions; reserved emergency RAM is mapped together, so separate IST guards are not established. No EFI-firmware-stack guard, canary or stack-clash defense is established by this audit. |

Intel defines SMEP as preventing supervisor instruction fetches from user pages, and SMAP as restricting supervisor data access to user pages; NX/page permissions are separate controls. See the primary [Intel SDM Vol.3A, §§4.6, 4.9, 11.12](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-3a-part-1-manual.pdf). Firmware state, CPU capability and effective enabled state must be distinguished.

No new execution test ran for this document. Existing NX/RO/guard/QEMU fixtures are verification routes, not newly measured results. v0.5 work remains: explicit capability/effective-bit read-back with honest `core:` reporting; SMEP/SMAP admission and recovery design; alias/AC audit; targeted negative probes; and coverage of all stacks. Current PAT diagnostics are not a CPU-defense status report. Preserve unsupported/untested outcomes instead of claiming all machines protected.

## 日本語

NXはCPU対応を検査し、EBS後のOS CR3切替前にEFER.NXEを設定します。通常のkernel data/GOP/DMAはNX、PE sectionとuser ELFには属性を設定し、RWXを拒否、CR0.WPも設定します。ただしEFI時点のmappingはfirmware管理で、一般的なNX有効bitの専用boot reportは未確認です。W^Xは仮想mapping単位で、user RX codeの物理backingにはsupervisor RW/NX aliasが残るため、物理全体のW^Xやuser code不変性まで証明しません。

SMEP/SMAPはOSが対応判定して有効化・読み戻し確認する処理が見当たりません。CR4は当該bitを継承するため「全機で無効」とも断定しません。boot Rust/assemblyにstac/clacはなく、現copy-in/outは範囲を検証してsupervisor alias経由で読書きします。従って単純なSTAC追加では安全性を証明できず、直接userアクセス・AC復帰・例外/割込み・aliasをまとめて設計する必要があります。

通常kernel stack、Ring3のsupervisor stack、ELF user stackにguardはありますが、2つのIST emergency stackは領域全体をmappingしており個別guardを確認できません。EFI stack guard、canary、stack-clash防御も本調査では成立を確認していません。user codeはRXで実行するため、方針の「user pageを実行可能にしない」はdataとの区別を決める必要があります。これは提案で、所有者方針を勝手に変更していません。

具体位置は英語表のリンク、CPU機構の根拠はIntel一次資料です。今回は静的調査だけで新規実行試験はしていません。v0.5では能力と実効bitの読み戻し・core表示、SMEP/SMAPの導入と復帰、alias/AC、異常系probe、全stackの確認が残ります。PAT表示を防御状態の証拠とせず、未対応・未確認をそのまま記録します。
