# 前処理後の問題の大きさの比較 (HiGHS 1.15.1 と ENOMOTO、MIP 55 問)

報告は `../presolve_size_20261025.md`。MPS は `.miplib_cache/` (`origin/inv/mps-cache-20261021` から展開) に置く。

- `one.sh <mps> <作業場所>`: HiGHS の前処理後と元の問題の形・LP 緩和の値を 1 行の JSON で出す (`sizes.jsonl`)。
- `eone.sh <mps> <作業場所> <ラベル>`: ENOMOTO の前処理後 (クリーク行の拡張の後) を書き出したところで止め、形と LP の値を出す。
  環境変数はそのまま渡る。`batch_e.sh <ラベル> <出力>` で 55 問 (`e_def`・`e_dd`・`e_all.jsonl`)。
  `sizes_final.jsonl` = `sizes.jsonl` + `e_def.jsonl` (ENOMOTO 側は `e2`、時間は `et`)。
- `hpre.py`・`hlog.py`・`hoff.py`: HiGHS の前処理 (書き出し・規則ごとの記録・規則を止めた大きさと LP の値)。
- `pstats.py`: MPS の形と LP 緩和の値。`cmpcols.py`: 2 つの前処理後の問題を列ごとに比べる。
- `table.py`・`summary.py`・`cmp_cfg.py`: 表 (`table.md`・`cmp_cfg.md`) と集計 (入力は `sizes_final.jsonl`)。
  `run_eno.py`: 求解して解を元の問題で確かめる (`solve_check.txt`)。
