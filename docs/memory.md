# ページテーブル・予約領域・RAM arena

2026-10-08実装。BSP、ring 0、割込み無効、4段pagingを対象とする。

## 予約と所有権

Boot Services終了前に通常stack 64KiB、map 128KiB、BootInfo 4KiB、
緊急stack 32KiB、ページテーブルpool 1MiBをLoaderDataとして確保する。
Loaded Image ProtocolからEFIイメージの基点・サイズを値として保存する。
Boot Services領域、Runtime Services領域、ACPI領域は回収しない。

最終メモリマップをdescriptor strideで読み、型・物理アドレス・ページ数・属性を解析する。
descriptor version 1、strideが40byte以上かつ8の倍数、全領域のページ境界、
算術overflow、領域間の重複を検査する。イメージ・stack・BootInfo・map・
緊急stack・table pool・GOPを明示予約し、重複していれば初期化を拒否する。

arenaの候補はEfiConventionalMemoryでWB対応属性を持ち、Runtime / RP / ROの
属性がない領域だけ。予約範囲とページ0を除外し、最大の連続領域から
16〜64MiBを選択する。16MiBに満たない場合はMEMORY LOWとして停止する。
選択後の重複も再検査する。DMA用の領域はまだ確保・公開しない。

メモリ初期化は一度だけ呼ぶ。arenaは唯一の `&mut [u8]` として診断アプリへ渡し、
アプリ内部で自由に配置できる。現在は各ページの両端へpatternを書き、
volatileで照合する診断のみ。汎用アロケータ、安定した外部アプリABI、
プロセス分離はまだ提供しない。BootInfoと予約領域の寿命は停止まで。

## 自前ページテーブル

4KiBページの恒等マッピングを使う。EFIイメージ、専用stack、BootInfo、
メモリマップ、CPUテーブル、緊急stack、table pool、GOP、arenaだけをマップする。
未使用RAM、UEFIサービス、機器MMIO全体を一括でマップしない。
通常stackの先頭4KiBは未マップのガードページとする。

イメージ内PE32+のsection情報を検査し、ヘッダーをRO/NX、コードをRO/execute、
書込可能sectionをRW/NXとする。write+execute、ページ単位のsection重複、
範囲外section、4096以外のsection alignmentは拒否する。
データ・stack・arena・GOPは実行禁止。全ページをsupervisor専用とする。
CR0.WPとEFER.NXEを有効にし、旧global translationをCR4.PGEの無効化でflushして
CR3を自前rootへ切り替える。CR3を読戻しで検査する。

CPUのNX・PAT・物理アドレス幅を確認する。ファームウェアのPAT / MTRRを変更せず、
既存PAT中のWBとUCを選択し、RAMへWB、GOPへUCを指定する。
MTRRとの組合せによる実効属性はCPUの規則に従い、WC最適化は行わない。

## 検証と未対応条件

ホストで不正map、overflow、予約の穴と重複、ページ境界、容量不足、
PEのコード・データ属性と不正sectionを検証する。
QEMUで通常起動、64MiB arena、null書込、コード書込、NX違反、stack guard、
自前root上の#UD / #GP / #DFを確認する。全ての例外は診断後停止する。

現時点でLA57 / PCIDが有効な構成、NX / PATなし、低位canonical範囲外のアドレス、
未対応map / PE形式、table pool不足は診断して拒否する。
ページテーブルの動的拡張、table pool回収、緊急stack guard、DMA / IOMMU、
ACPIのマッピング、実機でのPAT / MTRR適合性、arenaの全byte試験は後続作業。

参照: [Intel SDM](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)、
[UEFIメモリマップ](https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html)
