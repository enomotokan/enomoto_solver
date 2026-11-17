# クリーク型の行の数え上げ (2026-11-17、treeclose 9.78 節)

診断 `ENOMOTO_MIP_DIAG_CLQROWS` (既定では使わない。値 `stop` で数えたら止める、`nolp` で LP を解き直す比べをしない) で、前処理・クリークの拡張の後の
根の LP (カットの前) を数える。1 スレッド。

- `python run.py > run.out` → `res.jsonl` (58 問 + 重い 7 問)。集計 `python summ.py > summ.txt`。
- `python run.py drop > run_drop.out` → `res_drop.jsonl` (余裕のあるクリーク型の行がある 21 問で、LP を基底なしで作り直して、全部の行 / 余裕のある行を除いた行で解き比べる)。
