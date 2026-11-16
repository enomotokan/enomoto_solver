# cand1117 の確かめ (2026-11-16)

束 cand1117 = cand1116 + `ENOMOTO_MIP_TICKS=ntwumcflshk` + `ENOMOTO_MIP_DETHASH=1` + `ENOMOTO_MIP_DFSBB=2` (HiGHS 型の木の進め方、
analysis/treeclose_20261114.md 9.44・9.45 節)。設定は cfg.py (cand1114 / cand1116 / cand1117 / highs)。**c19fc41d9 以降の版で回す。**
分担表は plan.py (`python plan.py 48 9`、見積りは cand1116 の回の cand1116 の時間と cand1113 の HiGHS の時間)。

- P: 58 問 × 種 1〜4 × {cand1114, cand1116, cand1117}、60 秒、9 分担 (見積り各約 40 分): `python run.py P K` (K = 0〜8)
- M: Mittelmann 48 問 × 種 1・2 × {cand1116, cand1117, highs}、600 秒、48 分担 (見積り各 33〜43 分): `python run.py M K` (K = 0〜47)

同じ (問題, 種) の 3 設定は同じ分担で続けて解く (順は種で回す)。いずれも 4 スレッド、ソルバーは同時に 1 つ。途中から再開できる。
集計は `python summ.py P > summ_P.txt` / `python summ.py M > summ_M.txt` (SGM shift 1・問題単位のブートストラップの 95% CI、解けた数、
食い違い、最大 RSS、比 >1.15 / <0.87 の問題と「全種で遅/速」(全種で比 >1.25 / <0.8))。
準備: pip install maturin highspy==1.15.1 && pip install -e . 、python analysis/cbcimpl_20261029/probs.py (問題の取得)。
- X (追加): M で cand1117 / cand1116 が全種で > 1.25 だった問題に種 3・4 を足す (cand1116 の shards_X と同じ形、{cand1116, cand1117, highs}、600 秒)。
  M がそろってから `python planX.py` (問題を手で与えるなら `python planX.py 問題 ...`) で shards_X.json を作る。1 分担 = 1 (問題, 種) で最大約 32 分。
  `python run.py X K`、集計は `python summ.py MX` (M と X を合わせる)。
