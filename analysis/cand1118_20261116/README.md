# cand1118 の確かめ (2026-11-16)

束 cand1118 = cand1117 + `ENOMOTO_T_MIP_CUT_MIN_ROUNDS=8` + `ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0=8` (DFSBB=2 は k10・p100 のまま)。
設定は cfg.py (cand1114 / cand1117 / cand1118 / highs)。分担表は plan.py (`python plan.py 44 9`、見積りは cand1117 の回の cand1117・HiGHS の時間)。
台本は cand1117_20261116 の写し。

- P: 58 問 × 種 1〜4 × {cand1114, cand1117, cand1118}、60 秒、9 分担 (見積り各約 38 分): `python run.py P K` (K = 0〜8)
- M: Mittelmann 48 問 × 種 1・2 × {cand1117, cand1118, highs}、600 秒、44 分担 (見積り各 35〜41 分): `python run.py M K` (K = 0〜43)

同じ (問題, 種) の 3 設定は同じ分担で続けて解く (順は種で回す)。4 スレッド、ソルバーは同時に 1 つ。途中から再開できる。
集計は `python summ.py P > summ_P.txt` / `python summ.py M > summ_M.txt` (SGM shift 1・問題単位のブートストラップの 95% CI、解けた数、
食い違い、最大 RSS、比 >1.15 / <0.87 の問題と「全種で遅/速」)。M で全種で遅い問題が出たら `python planX.py` で種 3・4 の追加 X を作る
(`python run.py X K`、集計 `python summ.py MX`)。
準備: pip install maturin highspy==1.15.1 && pip install -e . 、python analysis/cbcimpl_20261029/probs.py (問題の取得)。
