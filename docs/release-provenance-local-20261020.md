# Local candidate provenance checkpoint / 候補のローカル来歴確認

On 2026-10-20, two isolated clean checkouts of source commit `5dc18fb4b1b3f3284fcae483ff7a013b426f0a07` each ran the normal `tools/prepare-release.py` command without feature overrides. `tools/compare-candidates.py --mode same-source` passed for the four archives, `build-manifest.json`, `SHA256SUMS`, and every runtime member including the EFI, QEMU image and three SDK sample ELFs. The machine-readable comparison is `/private/tmp/musha-five-offline-20261020/goal5/same-source.json`.

This is local reproducibility evidence for a development candidate. It does not establish physical USB acceptance, remote CI, a release tag, or publication approval.

2026-10-20、source commit `5dc18fb4b1b3f3284fcae483ff7a013b426f0a07`の独立した二つのclean checkoutで、featureを追加せず通常の`tools/prepare-release.py`を実行した。`compare-candidates.py --mode same-source`は四つのarchive、`build-manifest.json`、`SHA256SUMS`、EFI・QEMU像・SDK sample ELF三つを含む全runtime要素の一致を確認した。機械可読の比較結果は上記JSONに保存した。

これは開発候補のローカル再現性の証拠であり、実USB受入、remote CI、release tag、公開承認を示すものではない。
