# T2 three-phase capture completeness — 2026-10-20

## English

The current `log=trace` path is a sequential console. EARLY can scroll away and the viewer can overwrite PRE EBS or POST EBS. A visible `T2_DIAGNOSTICS_COMPLETE` marker proves only that the diagnostic emitted a completion marker; it does not prove all rows were photographed. The old rotating panels are obsolete. This checkpoint adds a **host-only**, read-only validator for a manually transcribed capture; it does not change boot UI, access the T2, persist logs from the device or guarantee that a camera captured a missing line.

`tools/validate_t2_capture.py` accepts one JSON file of at most 64 KiB, with no duplicate keys, exactly the three named phases, and at most 64 BCE records/errors per phase. The schema version is `1`:

```json
{
  "schema_version": 1,
  "run": {
    "model": null,
    "firmware": null,
    "efi_sha256": null,
    "config_sha256": null,
    "topology": null,
    "evidence_ref": null
  },
  "phases": [
    {"name":"EARLY","status":"not_captured","bce_count":null,"omitted":null,"records":[],"errors":[],"evidence_ref":null},
    {"name":"PRE_EBS","status":"not_captured","bce_count":null,"omitted":null,"records":[],"errors":[],"evidence_ref":null},
    {"name":"POST_EBS","status":"not_captured","bce_count":null,"omitted":null,"records":[],"errors":[],"evidence_ref":null}
  ],
  "complete_marker": false
}
```

This is an **explicitly incomplete template**, not a physical run. `null` means **NOT CAPTURED**, never zero. A visible phase uses `status: "observed"`, its raw `T2 BCE COUNT` and `OMIT` numbers, one record per visible BCE BDF, and a frame/log reference. Each record has `bdf`, `pci_id`, `command_status`, `bars_0_5` (an array of six raw BAR strings), `pm_msi` and `evidence_ref`; unknown fields stay `null`. Record all `T2 CAP ERROR` and similar lines in `errors`. `status: "error"` requires an error line and cannot carry measured counts; `not_captured` likewise cannot carry invented values. The validator rejects count mismatches, duplicate phases/BDFs and missing required shape. It reports incomplete for omitted records, any error, absent run identity, any missing record field or phase, a changed PCI ID across phases, or missing marker. Even a schema-complete result is **structural completeness of a transcription only**: raw values are not checked against the video, and it is not verified resources, DMA safety or goal 15 acceptance.

Run `python3 tools/validate_t2_capture.py CAPTURE.json --require-complete`; exit 0 means structurally complete, 1 means valid but incomplete, 2 means invalid input. Without `--require-complete`, valid incomplete data exits 0 for inventory workflows. Reviewers must compare each reference and value against the original video/log and note the EFI/CONFIG attribution basis. Keep the original binary, CONFIG and media/topology evidence separately. The host tests cover missing EARLY despite marker, omission/error lines, missing BAR fields, changed identity, duplicate phase/BDF, count mismatch, duplicate JSON keys and size limit. The validator does not make raw BAR values into validated BAR sizes or map the device.

For a future on-device aid, first review the sequential-console ownership contract. A bounded ring of phase-tagged read-only *copies* could expose counts, explicit overflow and the original boot-log record identifiers to a later viewer without capturing control writes or silently writing files. It must preserve EARLY/PRE/POST ordering, error/OMIT provenance and the existing USB shutdown path. Do not implement that display or persistence until its memory bound, ownership and UI lifecycle are reviewed; this host schema is the current safe bridge.

## 日本語

現行`log=trace`は逐次consoleで、EARLYがscrollし、後続viewerが画面を書き換えます。`T2_DIAGNOSTICS_COMPLETE`だけでは全行の撮影済みを証明しません。旧回転panelは復活させません。今回の追加は手入力した証拠の**host専用・読取り専用**検証器であり、boot UI/T2操作/自動file保存は変更しません。

上記schema v1は64 KiB、各phaseのBCE記録/error最大64件、3 phase固定です。`null`は**未採取**であって測定0ではありません。各行を元動画のframe等へ対応付け、BCE countとOMIT、Command/Status、BAR0–5（生値6件の配列）、PM/MSI、errorを記録します。欠落・省略・errorやmarker欠落、phase間のPCI ID変化は未完と表示し、重複/個数不一致/不正形は拒否します。`--require-complete`の終了codeは**転記構造**の完全0・未完1・入力不正2です。値が原動画と一致するかは人が確認する必要があり、完全判定でも実機資源・DMA・内蔵入力の受入ではありません。

将来の実機補助には、逐次consoleとviewer/USBの所有権・有限ring・overflow表示を先にreviewする必要があります。今回のschemaと試験は撮影漏れを識別できますが、失われた画面を復元しません。[既存採取手順](mac-storage-t2-stages-0.4.0.md)の動画・原本保持条件を維持します。
