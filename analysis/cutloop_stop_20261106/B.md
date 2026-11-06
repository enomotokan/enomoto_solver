# 根のカットのループの停滞の判定 (1 ラウンド目以降) とカットの出方: 担当 B (2026-11-06)

- bench に使う環境変数: 既定 (`def`) はなし。試作は `ENOMOTO_MIP_CUTSTALL_GAP=1` (`cs`、この記録で足したもの)。unitcal_7 の 600 秒の `csg` は
  `cs` に既存の `ENOMOTO_MIP_CUTSEL_PROBE=gain` を足したもの。根だけの確かめに `ENOMOTO_MIP_NODE_LIMIT=1` を足した回 (`chk_*`) がある。
  どれも `ENOMOTO_MIP_THREADS=4`・`ENOMOTO_MIP_LOG=1`。試作は既定では使わない。既定の動きは変えていない。
- 機械: 4 CPU、ソルバーは同時に 1 つ (`B/q*.sh` の待ち行列)。HiGHS は highspy 1.15.1。
- 生データ: `B/res.jsonl` (1 問ずつの回: 結果と根の情報)、`B/logs/<問題>_<回>.log.gz` (標準エラー)、`B/res58.jsonl` (58 問)、
  `B/run1.sh`・`q*.sh`・`run58.py` (回し方)、`B/rootinfo.py` (ログから根のラウンド数・木に入った時刻を数える)、`B/summ60.py`・`summ58.py` (集計)、
  `B/mushroom/` (mushroom-best の HiGHS のカットプールの調べ)。
- 担当 A (`A.md`、0 ラウンド目のカットの LP を外して止まる方) とは環境変数を分けた (`ENOMOTO_MIP_CUTSTALL_*`)。

(以下、計測が終わってから書く)
