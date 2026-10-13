# External API direction

## English

The [C1 transition probe](ring3-prototype.md) has a bounded fixture-only query/log/exit dispatcher. [C2](ring3-elf.md) adds SDK C query/log/exit with bounded owned-mapping copies and the candidate log permission. The following external application service contracts remain pending.

[Internal Rust API 5](app-api.md) remains operational and is not the external binary interface. External declarations are experimental/compile-only; the normal EFI has no external application dispatcher. Future single apps own their loop, without mandatory step/poll. No kernel callbacks are exposed.

| Operation | Candidate semantics | External status |
|---|---|---|
| query | zero args, feature mask, no mutation | wrapper only |
| log | UTF-8 bytes + u64 length, limit 4096; accepted-byte result, partial/AGAIN explicit | wrapper only |
| exit | signed int32 result, never returns; cancel app-owned requests | wrapper only |
| boot report | bounded versioned copy, unknown times/availability; read cannot draw/reset | type/number undecided |
| time/wait | monotonic deadline and wake reason, no busy-loop requirement | D |
| input/display | bounded events/overflow and copied commands, no physical framebuffer | D |
| files | opaque IDs, read-only first; partial progress; EOF=success/zero bytes | internal stream only |
| network | permission/buffer lifetime design required | D |

Signed int32 errors use [candidate values](application-abi.md); raw ABI sign-extends them. Unsupported operations must fail. Output capacity differs from input length. User buffers remain application-owned, checked/copy-pinned until asynchronous completion. Cancellation needs confirmed completion before reuse; unconfirmed DMA stop retains memory. Exit invalidates handles; device stop is a separate OS action. Current single cooperative session still ends both in sequence.

No libc/POSIX, allocator, dynamic loader, write API, application UDP or thread API is supplied. SDK 0.0.1, ABI 0.1/r1, internal API5 and OS VERSION are independent.

## 日本語

内部API5を維持し外部ABIと区別します。SDKは照会・ログ・終了wrapperまでで、汎用の外部アプリ向けOS側syscallは未実装です。C1には固定試験専用の照会・ログ・終了dispatchがあります。外部I/Oは上限・部分完了・EOF・期限・取消し・所有者・世代検査をDで定めます。DMA停止確認前の再利用は禁止し、内部で動く機能を外部対応済みとは表記しません。
