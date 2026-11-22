# traininstance6 の not_solved (2026-11-22)

gentop3 の M 分担 11 (ビルド 27d87ea36、def、4 スレッド・種 1・600 秒) で、traininstance6 が 302.8 秒・121410 ノードで
not_solved (目的値 28290 = 下界 28290) で終わった。t3 は optimal 28290 (288.3 秒)。

## ファイル

- `rep.sh SEED TAG`: def・4 スレッド・600 秒・`ENOMOTO_MIP_LOG=1` で 1 回解き、ログを `logs/ti6_sSEED_TAG.log.gz`、結果を `rep.jsonl` に足す。

## 再現 (rep.jsonl、logs/)

| ビルド | 種 | 設定 | 状態 | 秒 | ノード | 際どい実行不能: 一から作った LP で確かめ / 結論なし |
|---|---|---|---|---|---|---|
| 27d87ea36 | 1 | def | optimal 28290 | 271.7 | 130594 | 184 / 16 |
| 27d87ea36 | 2 | def | optimal 28290 | 236.2 | 99773 | 301 / 0 |
| 27d87ea36 | 3 | def | optimal 28290 | 223.5 | 116017 | 193 / 0 |
| 09e6f6da6 + 数え上げ | 1 | def | optimal 28290 | 293.6 | 122094 | 333 / 0 (葉の結論なし 0) |
| 09e6f6da6 + 数え上げ | 1 | LEAF_FRESH | optimal 28290 | 173.6 | 77042 | 168 / 0 (葉の結論なし 0) |

4 スレッドの揺れで、5 回とも再現しなかった。`scan.sh` で neos17 (60 秒、種 1〜10)・traininstance6 (種 4〜8) を走査中 (`scan.jsonl`)。

## 木を進めた後に not_solved になる道筋 (コード)

既定で not_solved を返す所のうち、木の探索を終えた後のもの:

1. `self.unresolved` (solver.rs の木の探索の終わり): 下界 = 暫定解の値で `NotSolved`。報告の「目的値 = 下界 = 28290」と合うのはこれだけ。
   `unresolved` が立つのは (a) `branch_without_lp` で整数列がすべて固定された葉 (LP が解けない・結論なしで分けられない)、
   (b) 木を分け合う従のスレッドの異常終了 (`join` が `Err` = panic)、(c) 従の `pool.unresolved` (従の (a))。
2. 共有の最良値が主の暫定解より良い (木の探索の終わり・最後の守り・PERMRACE): 下界 = 共有の最良値など、暫定解より小さい値になり、警告の行が出る。
3. 止めた従・待ち行列に打ち切り値より下のノードが残った: 下界 < 打ち切り値 (< 暫定解)、警告の行が出る。

元の計測 (run.py) は stderr を捨てていたので (a) と (b) は区別できない。(a) は `lpinf_fresh_check` の「結論なし」(一から作った LP は
実行可能だが元の LP は実行不能のまま、など) が葉で起きる場合で、wrong_widden_20261117 の 10 節の neos17 と同じ型。
Python 側 (`src/types.rs` の状態の文字列、`benchmark_highs` の worker) は MipStatus をそのまま写すだけ。

## 数え上げと試作 (src/mip/solver.rs)

- `LEAF_NOLP` (既定の動作は変えない): 整数列がすべて固定された葉で LP の結果がない回数。`ENOMOTO_MIP_LOG` で、未解決の印を付けるたびに
  `MIP: a leaf with all integer columns fixed has no LP result ...`、最後に `MIP: leaves with all integer columns fixed and no LP result ...` の行。
- 試作 `ENOMOTO_MIP_LEAF_FRESH`: その葉で、定義域の境界と LP の行 (カットを含む) で一から作った LP を解く。実行不能 (証明が際どくない) なら閉じ、
  最適なら値が打ち切り値以上か、解を暫定解にできれば閉じる (整数列はすべて固定なので、この LP の最適値が葉の最良値)。閉じられなければ今までどおり未解決。

## 走査 (scan.sh → scan.jsonl、先頭 09e6f6da6 + 数え上げ、def、4 スレッド)

- neos17 (60 秒、種 1〜10): すべて optimal (3.6〜5.5 秒)、葉の結論なし 0、lpinf の結論なし 0。
- traininstance6 (600 秒、種 4〜8): すべて optimal 28290 (286.5・443.3・327.1・415.1・447.7 秒)、葉の結論なし 0。
  lpinf の結論なしは種 8 で 54 回 (一から作った LP が実行可能 61) 出たが、どれも葉ではない (LP なしの分枝で分けられた)。
- まとめ: 計 20 回で not_solved は再現せず、葉の結論なしも 0。(a) と (b) は区別できないまま。

## 試作 LEAF_FRESH の確かめ (fuzz.sh → fuzz_lf.out、ENOMOTO_MIP_LEAF_FRESH=1)

- fuzz 7 本すべて bad 0 (tiny 400・highs 300・objint 1000・slack 1000・tiny2 400・objint2 1000・slack2 1000、内訳も以前と同じ形)。
- `cargo test --release --lib`: LEAF_FRESH・既定とも 419 通過・0 失敗。
- ただし葉の結論なしが一度も起きていないので、試作の閉じ方の経路そのものはまだ通っていない。方針 (親): 試作 (既定外) のまま。

## 見分けるための手当て

- 計測の台本 (`analysis/gentop3_20261121/run.py`・`analysis/objstepcut_20261122/wide/run.py`): worker の stderr を一時ファイルに受け、
  not_solved・crash の回だけ最後の 60 行を結果の行の `"stderr_tail"` に残す。今後の M の台本はこれを写す。
- solver.rs: (a) 葉で未解決の印を付けるとき (スレッドごとに最初の 1 回) と、(b) 従のスレッドの異常終了のときに、既定でも 1 行出す
  (`MIP: a leaf with all integer columns fixed has no LP result ...`・`MIP: a tree search worker thread ended abnormally (panic) ...`)。
  (b) は panic の通知 (Rust の既定の panic の出力) も stderr に出る。これで次に not_solved が出たら stderr_tail で道筋が分かる。

## 残り

- 次に not_solved が出たら stderr_tail を見る。(a) なら LEAF_FRESH を既定にするかを (fuzz・広い計測で) 判断する。

## 過去の結果

- analysis/*/res_M*.jsonl (18000 行) の not_solved はこの 1 行だけ。M 以外の res_*.jsonl では 18 行: neos17 (P、11-17 の旧方式の pcd・parorder・gate2 など) 8 行、
  robust2_20261114 の prace (試作) 10 行。neos17 は analysis/wrong_widden_20261117 の 10 節の型 (a)。
