# SDK packaging local checkpoint / SDK配布ローカル到達点

Status: current (2026-10-15). Uncommitted tool development evidence; no release acceptance.

## English

Normal candidate CLI builds pinned C Hello and Rust Hello/start-record by default, or accepts unique explicit --sdk-sample LANGUAGE:NAME=PATH roles. Explicit mode requires C Hello plus both Rust roles; unknown/duplicate/incomplete roles, symlink/nonregular/oversize inputs fail. stage and SDK creation still check ELF structure and carry actual sample bytes/hashes through SDK manifest, build sdk_status and runtime_payload. Workflow passes samples built in its earlier SDK step; remote CI was not run. The leftover Rust source-scaffold flag is true only when both validated packaged Rust roles exist. This is a development implementation flag, not guest execution or hardware/formal release approval. Tag/clean/release/SDK-sample gates remain intact. --docs-complete stays false and no docs.zip is qualified.

Existing clean commit fdcd036 was independently built twice using the unmodified candidate CLI in an isolated checkout: actual compare-candidates --mode same-source passed for all four archives plus manifest/checksums. Its older SDKs had no embedded samples; that pass is condition1 evidence for that existing commit, not current dirty runtime or complete SDK release qualification.

A second test overlaid only the four modified tool/workflow files on that isolated fdcd036 base and truthfully used --allow-dirty. The preview contains C Hello and Rust Hello/start-record (three ELFs); archive bytes, sample manifests, sdk_status, runtime_payload and checksums match. Both SDKs were independently extracted and rebuilt; all three ELFs are byte-identical. Comparing this dirty preview with the strict comparator is deliberately rejected as Incomplete/unclean candidate. Condition2 classification/failure behavior has fixture coverage; no equivalence to a hardware-accepted candidate is claimed.

Local evidence: `out/sdk-packaging-round3/verification.json` plus copied clean comparison, preview source/dirty-list and manifest/checksums. Package6, distribution23 and comparison5 tests pass. No physical USB, hardware, new guest execution, full QEMU matrix, remote CI, tag/push/release or final-source acceptance was performed. Current runtime changes are outside the frozen fdcd036/tool-overlay test scope.

## 日本語

候補CLIは既定で固定toolchainのC Hello/Rust Hello・start-recordを生成、又は重複しない--sdk-sample LANGUAGE:NAME=PATHを受け付けます。明示指定はC Hello＋Rust2roleが必要で、未知/重複/不足role・symlink/通常file以外/過大入力を拒否。stage/SDKはELF構造を検査し実bytes/hashをSDK manifest・sdk_status・runtime_payloadへ伝播。workflowは先行SDK stepのsampleを渡し、remoteCIは未実行。Rust旧scaffold flagは検証済2roleをpackageした時だけ実装済みになります。開発実装flagでguest実行/実機/正式公開承認ではなくtag/clean/release/sample gateは維持。docs-complete=false、docs.zip未認定。

隔離clean既存fdcd036を旧CLIで独立2buildし実compare same-sourceが4archive＋manifest/checksums全一致で合格。ただし旧SDKはsample無しで、その既存commitの条件1証拠であり現dirty runtime/SDK正式受入ではありません。

別検証で同baseへ変更tool/workflow4fileだけoverlayし--allow-dirtyを正直に記録。previewのC1/Rust2 ELFはarchive/manifest/sdk_status/runtime_payload/checksum一致。SDKを別directoryへ展開再buildして全3ELF byte一致。dirty previewを正式comparatorで比較すると意図どおりIncomplete/unclean拒否。条件2分類/異常はfixture検証で、実機受入候補との一致は主張しません。

証拠は`out/sdk-packaging-round3/verification.json`等。package6/distribution23/comparison5合格。実USB/実機/新guest実行/全QEMU/remoteCI/tag/push/release/finalsource受入は未実施。並行中runtime変更は固定fdcd036＋tool-overlayの試験範囲外です。
