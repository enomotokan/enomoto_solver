# メモリの試作 (memall) と修復の下界の直し (FZ3_REPAIR_BOUND) の既定化 (2026-11-04)

既定にしたもの (コミット ae1497cc。それぞれ `=0` で以前の形に戻る。まとめて戻すのは run.py の `prememall`):

| 環境変数 | 既定 | 中身 | 記録 |
|---|---|---|---|
| `ENOMOTO_MIP_MEM1_SIBLU` | `1` | 潜りの兄弟の一覧で親の LP の LU を持つのを新しい 1 個だけにする (`ENOMOTO_MIP_MEM1_SIBLU_ROWS`、既定 1000: LP の行がこれ以上のときだけ) | `uccase12_mem1_20261103.md` |
| `ENOMOTO_MIP_MEM2` | `p` | ノードに保存する DSE 重みを番号なしの f32 の並びにする (ほかの文字 `s`・`b`・`d`・`l` は試作のまま) | `uccase12_mem2_20261103.md` |
| `ENOMOTO_MIP_MEM3_TINY_MARGIN` | 有効 | 幅の小さい列の固定で行を緩める余裕を `1e-9 (1 + min(\|b\|, 1)) + 4ε\|b\|` にする (右辺の大きい行を 1e-9\|b\| 緩めない。再スタートの後の暫定解が元の問題で実行不能になるのを防ぐ) | `uccase12_mem3_20261103.md` |
| `ENOMOTO_MIP_FZ3_REPAIR_BOUND` | 有効 | 後処理の修復で下界のずれを直す前の解で測り、下界を直した解の値で抑える。時間の上限なし (inf) で修復が panic するのを防ぐ | `fuzz_tiny_classify_20261103.md` |

## 根拠

- bench22 (60 秒、`halve_submip_20261030.md`): memall は 58 問 2 回 SGM −0.5%・解けた数 87 -> 85、Mittelmann −0.3%・49 -> 49。
  ほかの 3 つの単独 (siblu1r・mem2p・margin) も ±2% の中。解けた数の −2 が揺れかは bench23 (境目の 7 問を 6 回ずつ) で確かめる。
- 600 秒 (`mitt600_20261103.md` の「memall の 600 秒」): 既定でメモリ不足で落ちた 6 問のうち 5 問 (app1-2・chromaticindex1024-7・
  neos-4763324-toguru・radiationm40-10-02・uccase12) が落ちなくなり、最大 RSS は 9 問中 8 問で 12.2〜13.6 GB -> 1.6〜9.0 GB。supportcase18 は落ちる。
- FZ3_REPAIR_BOUND: `fuzz_mixtiny.py` で時間の上限なしの 3 seed が panic していた。fuzz_tiny 400・`cargo test --release --lib` は変わらない。

## 確かめ

- ビルド (ae1497cc) で neos-1456979 (20 秒、`ENOMOTO_MIP_LOG=1`): 既定は `MIP: mem1 sibling LU (all threads): held at most 1, pushed 985, dropped 967, adopted 5`
  (兄弟の LU の間引きが働く)、`prememall` ではこの行が出ない (以前の形)。
- 残り: 開いたノードの数に比例して増える持ち物に上限がない (radiationm40-10-02・uccase12・CMS750_4 は 600 秒でも伸びる) ので、
  並列のセッション (memcap A・B・C) で上限の試作と supportcase18 の持ち主を調べている。

## 追記: 丸めの余裕はサブ MIP では以前のまま (2026-11-04)

- bench23・bench24 (`halve_submip_20261030.md`) で qiu が系統的に遅くなった (中央値 22 → 40 秒) のは `MEM3_TINY_MARGIN` を戻すと消える (仕組みは未確認、
  サブ MIP の前処理で効いていると推測。余裕は固定する列の数は変えず、固定の後に行を緩める量だけを変える)。既定を `m` (外側の求解と再スタートだけ小さい余裕) に直した。qiu は 6/6・中央値 28 秒に戻る。
