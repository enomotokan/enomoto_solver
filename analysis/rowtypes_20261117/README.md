# 元の問題の行の種類ごとの数え上げ (2026-11-17、treeclose 9.83 節)

診断 `ENOMOTO_MIP_DIAG_ROWTYPES` (既定では使わない) で、木に入るとき (根のカットのループの後の LP 解) に元の行を種類 (clique・vb・knap・int・mixed・cont・eq・range) に分け、
行・非零・スラックが基底の割合・相対の余裕の分布と、大域の境界の下で冗長な行を、木に入るとき・60 秒 (`ENOMOTO_T_MIP_DIAG_ROWTYPES_SECS`)・終わりに数える。
4 スレッド・種 1、58 問 90 秒 + 重い 10 問 150 秒。`python run.py > run.out` → `res.jsonl`、集計 `python summ.py > summ.txt`。
