# NUC5 segment recovery diagnosis / NUC5 segment復帰診断

Current trace follow-up: detailed control/stage/timing and BAR evidence requires a newly built `log=trace` EFI; F8/8 selects trace. The earlier development EFI hashes and observations below predate trace and keep their original settings. Concise device facts remain at devices; no old artifact is relabeled or regenerated here. See [boot log](boot-log.md).

現行Trace仕上げ：詳細制御／stage／計測／BAR証拠は新sourceからbuildした `log=trace` EFIで採取し、F8／8もtraceへ強制します。下記の旧開発EFI hash／観測はTrace追加前なので元の設定と来歴を保持します。devicesは機器事実の要約です。本追記で旧artifactを付け替えたり再生成したりしません。[boot log](boot-log.md)参照。


Status: current (2026-10-16). Local correction verified; physical retest pending. Uncommitted development work.

## English

### Recorded failure and exact mapping

Owner video `IMG_7654.MOV` SHA256 `1af681b389513672bc3c7cedca324368ae7332d2849631b5fdecfa6c0e1581a7` shows VECTOR **D (#GP)**, ERROR **18h**, RIP **140056D89h**, not vector0/#DE. The raw frame is `out/nuc5-7654/frame-006.png`; network state is in `dense-0052.png`. EFI association comes from prior deployment/readback, not a hash visible in the video. The exact deployed EFI is SHA256 `268b9a49705dde18fb249378c0ae9ccf0f2de6d6a925c786404b083da4953220`.

A local rebuild of the unchanged pre-fix source with only linker `/map` added produced that **identical EFI hash**. `out/nuc5-fix-map/image.map` places the fault within `ring3::lifecycle::once` at140056B90h, before `launch` at140056F20h. `attempts` is inlined there. RIP minus PE base140000000h gives RVA56D89h; bytes `66 41 8E E4` decode to `movw %r12w,%fs`, the restoration statement at pre-fix `boot/src/ring3/lifecycle.rs:81`. This is an exact binary/function mapping plus matching source statement, not a PDB line-number claim.

The OS GDT uses selector18h for the TSS. Previously GDT takeover reloaded DS/ES/SS but retained firmware FS/GS selectors and their cached descriptors. Reloading the saved FS selector during app recovery consults the new GDT and can interpret its index as a TSS, causing #GP. ERROR18h identifies the GDT slot; the full prior selector including RPL was not captured. The correction reloads FS/GS with the valid kernel data selector10h during GDT takeover and verifies both. It does not suppress #GP or skip recovery checks. A QEMU-only fixture creates cached data selectors18h, restores the TSS descriptor, then follows the production takeover and recovery path.

### Independent I218 finding and timer audit

At5.2seconds the clip shows PCI8086:15A3 REV03 BDF00C8 (00:19.0), `I218 SOFTWARE NVM CONFIG REQUIRED`, USB/FAT32 reads, then stopped DMA. The reset backend rejects software-required NVM configuration at register0x28 bit27 before the hardware-autoload path. This is separate from the later CPU segment fault. No PHY/reset/link/DHCP/UDP success or five-second budget timeout is established. The safeguard is retained: an unreviewed LCD/OEM implementation is not replaced by a bypass.

No zero-denominator path was found: PM milliseconds use constant frequency3579545; short-delay conversion uses constant1000000; the I218 budget uses subtraction/multiplication. Zero and excessive driver delays are rejected. Audit did find a range mismatch: reset/link requests5000–50000us while `ShortDelay` accepts at most1000us. `Time::delay_us` now splits requests up to one second into at most1000 valid short chunks, samples the clock throughout, and still fails on a stalled clock. The existing I218 aggregate5-second deadline remains shared; no deadline or DMA-stop gate was weakened. This timer defect was latent behind the NVM refusal and is not claimed as the observed fault's cause.

### Exception frame and local results

No-error-code stubs already push a synthetic zero; hardware-error stubs retain the CPU value. Compile-time frame-offset assertions and host vector classification protect the contract. UI now displays `ERROR N/A` when the CPU supplies no error code; debug's normalized numeric zero remains compatible. #UD vector6 displays N/A; #GP vectorD retains its real error38h in the QEMU fault probe.

Evidence is `out/nuc5-segment-fix/verification.json`. Related platform/net host159 cases pass, including new error-code classification and bounded long-delay/zero/oversize cases; existing I218 software-NVM rejection tests remain. Final focused QEMU checks cover cached18h takeover plus actual PM50ms/zero rejection and82574 DHCP/UDP with three app returns, injected NVM refusal with three returns, #UD, #GP, and strict normal viewer pixels. Earlier two intermediate return checks are retained as overlapping executions, not additional features. This is not a full QEMU regression or physical acceptance.

### Corrected development EFI

`out/i218-segment-fix-20261016/files/EFI/BOOT/BOOTX64.EFI`, SHA256 **2275d088fe7bebedab61556cced9ff705cff309c4c6dae1cd800ad553f7fcf53**. Locked release UEFI build, `i218-network` only (transitive `i218-phy-probe`), no injection feature. Source HEADd44cdab plus recorded uncommitted changes; CONFIG is `log=devices\n`. This is a development artifact, not a formal candidate. No USB deployment, commit or push was performed. The next owner test must separately record app/viewer recovery and the still-blocked physical I218 path. Do not run UDP echo until an actual DHCP lease exists.

References: [Intel SDM volume2B MOV](https://cdrdv2-public.intel.com/835752/253667-sdm-vol-2b.pdf) and [system programming manual](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-system-programming-manual-325384.pdf); [I218 quickcheck](i218-hardware-quickcheck.md), [acceptance worksheet](hardware-acceptance-0.4.0.md).

## 日本語

### 記録した失敗と正確な対応

所有者動画`IMG_7654.MOV`（SHA256 `1af681b389513672bc3c7cedca324368ae7332d2849631b5fdecfa6c0e1581a7`）はVECTOR **D/#GP**、ERROR **18h**、RIP **140056D89h**で、vector0/#DEではありません。原frameは`out/nuc5-7654/frame-006.png`、LAN表示は`dense-0052.png`。EFI hashは動画から測ったものではなく前段deploy/readbackとの対応です。起動したEFIは`268b9a49705dde18fb249378c0ae9ccf0f2de6d6a925c786404b083da4953220`。

修正前sourceへlinker `/map`だけ追加した再buildが**同じEFI hash**になりました。`out/nuc5-fix-map/image.map`によりfaultは140056B90h開始の`ring3::lifecycle::once`内、次の`launch`は140056F20h。`attempts`はinline化されています。PE base140000000hとの差はRVA56D89h、bytes `66 41 8E E4`は`movw %r12w,%fs`で、修正前`boot/src/ring3/lifecycle.rs:81`の復元文と一致。正確なbinary/関数対応とsource文の照合で、PDB行番号の主張ではありません。

OS GDTのselector18hはTSSです。従来takeoverはDS/ES/SSだけ再設定し、firmwareのFS/GS selectorとhidden descriptorを残していました。app復帰時のFS再loadは新GDTを参照するため、同じindexがTSSになり#GPとなり得ます。ERROR18hはGDT slotを示し、RPLを含む元selector全値は未採取。修正はtakeover時にFS/GSを有効なkernel data10hへ正規化して読戻し。#GPや回復検査は抑制しません。QEMU専用fixtureでcached data18hを作りTSS descriptorを戻してから、production正規化・復帰経路を検証します。

### 別件のI218とtimer監査

動画5.2秒にはPCI8086:15A3 REV03 BDF00C8（00:19.0）、`I218 SOFTWARE NVM CONFIG REQUIRED`、USB/FAT32読取りとDMA停止が見えます。reset backendはregister0x28 bit27のsoftware-required NVM構成をhardware-autoload前に拒否します。後続CPU segment faultとは別件で、PHY/reset/link/DHCP/UDP成功や5秒budget超過は確認できません。LCD/OEM実装の未reviewを安全拒否の無効化で置き換えません。

zero分母は見つからず、PM msは固定3579545、短delay換算は固定1000000、I218 budgetは減算・乗算です。0/過大delayは拒否。一方reset/linkが5000～50000usを要求するのに`ShortDelay`上限1000usという実際の範囲不一致を発見しました。`Time::delay_us`は最大1秒の要求を最大1000個の有効な短待ちへ分割しclockを継続sample、clock停止時は失敗します。I218全体5秒期限・DMA停止gateは維持。この潜在timer不具合はNVM拒否より先へ進んだ場合の問題で、映像の直接原因とは主張しません。

### 例外frameとローカル結果

error code無しstubは元からsynthetic0をpushし、有りstubはCPU値を維持していました。frame offsetのcompile時assertとhost分類試験を追加。CPUがcodeを出さない場合だけUIを`ERROR N/A`へ変更し、debugの正規化数値0は互換維持。QEMUの#UD vector6はN/A、#GP vectorDは実code38hです。

証拠`out/nuc5-segment-fix/verification.json`。関連platform/net host159件合格（新規分類・長待ち分割・0/過大負例を含む）、既存NVM拒否試験も維持。最終限定QEMUはcached18h＋実PM50ms/zero拒否＋82574 DHCP/UDPで3復帰、NVM拒否注入で3復帰、#UD、#GP、通常viewer厳密pixelを確認。途中2回の復帰検査は重複実行として保持。全QEMU回帰・実機受入ではありません。

### 修正開発EFI

`out/i218-segment-fix-20261016/files/EFI/BOOT/BOOTX64.EFI`、SHA256 **2275d088fe7bebedab61556cced9ff705cff309c4c6dae1cd800ad553f7fcf53**。locked/release/UEFI、`i218-network`のみ（派生`i218-phy-probe`）、注入featureなし。HEADd44cdab＋記録した未commit差分、CONFIGは`log=devices\n`。正式候補ではなく開発artifact。USB配備・commit・pushなし。次の所有者実機確認ではapp/viewer復帰と未解決I218を分け、実DHCP leaseが表示されるまでUDP echoを実施しません。

一次資料は英語節Intel SDMリンク、[I218手順](i218-hardware-quickcheck.md)・[受入表](hardware-acceptance-0.4.0.md)参照。
