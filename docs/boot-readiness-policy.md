# Boot readiness integration — document version 3.0

## English

This active integration supersedes policy 2.0. The complete owner-supplied [policy 3.0, including section 12](reference/boot-api-sdk-v3/docs/boot-readiness-policy.md), [implementation request](reference/boot-api-sdk-v3/handoff/implementation-prompt.md), reviews and SHA256SUMS are preserved unchanged. All supplied checksums were verified. Document version is independent of OS, internal API and external ABI versions. The original remains the normative direction; implementation status below takes precedence over any assumption that a proposed feature already exists.

Adopted direction: one application owns its Ring 3 loop and private arena; OS owns protection and basic I/O. No permanent external step/poll obligation. A/B currently implement a static/cooperative transition profile; [C1](ring3-prototype.md) now probes privilege transitions; the external-app C gate and practical I/O in D remain open.

### Responsibilities and current boundaries

`main::runtime` initializes CPU/memory/display/timer. `diagnostics` updates bounded RAM records without rendering. Explicit `flush` compares physical rows and redraws only changed rows. The normal eight-second page rotation is removed. Fatal CPU display remains independent. Diagnostic rendering stays hidden while the viewer owns the screen. Backend completion does not overwrite viewer output.

`runtime::run` owns Context, application lifecycle, clock, input sources and optional NIC. OS xHCI `run_session` and nested Host frames retain controllers/DMA; an outer storage Host survives inner keyboard-controller callbacks. File Session owns mounted stream state. View changes never reset devices. Existing cancellation/generation checks, key release, DMA stop-before-reuse and retention on unconfirmed stop remain. The single-session contract still stops services after app exit; it does not implement preemption or multiple apps.

| State | Meaning in A |
|---|---|
| CORE_READY | Core CPU/memory established, not application readiness |
| OS_READY FILE_VIEWER | Real internal display/clock/arena and a live mounted file service |
| OS_READY RECOVERY_DISPLAY | Display/clock/arena usable; optional file/input may be absent |
| APP_ENTERING | OS-side moment before static init call |
| APP_RUNNING | Static application reached init; not CPL3 evidence |
| first_frame_complete | First app software render completed, not scanout/vsync |
| APP_EXITED / APP_FAILED / APP_LAUNCH_FAILED | Application outcomes independent of services |
| Service Stopped / StopUnconfirmed | Resources no longer advertised live |

Required clock progress failure never emits OS_READY. Context version/display/arena are exercised before entry. File Session is mounted and its actual metadata API exercised before file-viewer readiness. Missing input/LAN are bounded optional discovery outcomes with existing reason logs. Controller `AppEvent::Ready` is not global readiness; repeated notifications cannot relaunch an initialized app. Normal viewer readiness excludes diagnostic RAM repetition and MUSHA.TXT content checks, retaining memory/resource/ownership/reset and mount validation.

The bounded journal records omitted count, first failed stage and latest major error. Future boot-report API returns versioned bounded value copies and opaque handles, never Panel/BootInfo or physical addresses; reading cannot draw/reinitialize devices. Its wire record is not implemented.

Boot timestamps use one PM epoch starting before runtime memory initialization. EFI entry remains unknown, not zero. Integer milliseconds have timer resolution limits; sampling must occur more often than the PM timer wrap (about 4.69 s for 24 bits). Multi-wrap stalls cannot be reconstructed. Do not subtract the separate existing app Context epoch from boot times or claim full EFI-to-ready timing. First frame is sampled at software completion and not overwritten by diagnostic viewing.

See [API](application-api.md), [ABI ledger](application-abi.md), [SDK](sdk.md), [validation/gates](boot-api-validation.md).

## 日本語

添付3.0を2.0の更新版として採用し、原本第12節・実装指示・照合値を保存しました。記録更新と描画を分離し、変更行のみ明示描画します。通常版の8秒ページ送りとアプリ固有RAM反復／ファイル内容検査をReady条件から外し、安全検査は維持します。

OSランタイムが機器・DMA・入力・NICの寿命を所有します。FILE_VIEWERとRECOVERY_DISPLAY、OS_READYとAPP_ENTERING／APP_RUNNING、描画完了と機器停止を区別します。現段階は静的／協調実行で、Ring 3ではありません。EFI入口時刻は未取得であり、タイマーの複数ラップを含む完全な起動時間保証もありません。[C1](ring3-prototype.md)で固定payloadの特権遷移試験を追加しました。Cの外部アプリ実行とDの実用外部I/Oは後続です。
