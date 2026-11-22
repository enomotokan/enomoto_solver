# traininstance6 の not_solved (2026-11-22)

gentop3 の M 分担 11 (ビルド 27d87ea36、def、4 スレッド・種 1・600 秒) で、traininstance6 が 302.8 秒・121410 ノードで
not_solved (目的値 28290 = 下界 28290) で終わった。t3 は optimal 28290 (288.3 秒)。

## ファイル

- `rep.sh SEED TAG`: def・4 スレッド・600 秒・`ENOMOTO_MIP_LOG=1` で 1 回解き、ログを `logs/ti6_sSEED_TAG.log.gz`、結果を `rep.jsonl` に足す。

## 調べ中

- 有力な道筋 (コードから): ノードの LP が際どく実行不能 → 一から作った LP は実行可能だが元の LP は実行不能のまま (`lpinf_fresh_check` が `Error`
  = 結論なし) → LP 値なしの分枝 (`branch_without_lp`) で、整数列がすべて固定された葉なので分けられず `unresolved` → 木の探索の終わりで
  `NotSolved` (下界 = 暫定解の値)。報告の「目的値 = 下界」と合う。ログの `marginal LP infeasibility` の行で確かめる。
- 過去の M の結果 (analysis/*/res_M*.jsonl、18000 行) の not_solved はこの 1 行だけ。M 以外では neos17 (種 2 など) が同じ型
  (analysis/wrong_widden_20261117 の 10 節)。
