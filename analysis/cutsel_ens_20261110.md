# PAPER5: 文脈を見るカット選択 (SCIP の cutsel_ensemble) の試作 (2026-11-10)

bench に使う環境変数: `ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=s` (s = 1, 2) に、設定ごとに
def = なし (既定 = cand1108zd)、ens = `ENOMOTO_MIP_CUTSEL_ENS=1`、ensw = `ENOMOTO_MIP_CUTSEL_ENS=w`。
計測のスクリプトは `cutsel_ens_20261110/bench.py`、根の表と矛盾の検査は `summ.py`、生データは `cutsel_ens_20261110/*.jsonl`・`logs/`。

(途中。計測が終わり次第まとめる)
