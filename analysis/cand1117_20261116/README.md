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
- I (切り分け): P で cand1117 が cand1114 より全種で遅かった 10teams (3.54)・neos-827175 (1.59) の原因を分ける。cand1114・cand1116・cand1117 と、
  cand1117 から 1 つずつ外した 10 通り (m_*)・cand1116 に 1 つずつ足した 3 通り (c16+*) (cfg.py の ISO) × 2 問 × 種 1〜8、60 秒 (256 回)。
  `python run.py I K` (K = 0〜3、各約 30 分)、集計 `python summI.py > summ_I.txt`。
- W (誤答の切り分け): unitcal_7 種 2 で cand1116 が最適値を切り落とした件。cand1116・cand1117・cand1116 から鍵を 1 つずつ外した 7 通り × 種 1〜8、600 秒 (72 回)、
  18 分担 (各最大約 42 分): `python run.py W K` (K = 0〜17)。デバッグ解 (unitcal_7_pre.sol.gz) を渡し、logs/ に MIP_DEBUG_SOL の行を残す。集計 `python summW.py > summ_W.txt`。
- S (supportcase40 の切り分け): cand1116・cand1117・s_nodfsbb・s_k5・s_k3・c16+dfsbb × 種 1〜4、600 秒 (24 回)、8 分担 (各最大約 32 分):
  `python run.py S K` (K = 0〜7)、集計 `python summ.py S`。
- W2 (誤答の再現、回数を増やす): cand1116・cand1117 × unitcal_7 × 種 1〜16、600 秒 (32 回)、8 分担: `python run.py W2 K` (K = 0〜7)、
  集計 `python summW.py W2 > summ_W2.txt`。最新の版でビルドして回す (デバッグ解の検査の誤検知を直し、落とした所の説明を足した版)。
- G (DFSBB=2 の部品の切り分け): supportcase40・rocI-4-11・mcsched (600 秒) と noswot・timtab1・roll3000 (60 秒) × 種 1〜4 × 8 設定
  (cand1117・s_nodfsbb・g_p10・g_p30・g_k1・g_pq・g_sq・g_sib、cfg.py の HGS)、192 回、16 分担 (見積り各 35〜41 分): `python run.py G K` (K = 0〜15)、
  集計 `python summ.py G`。ENOMOTO_MIP_HGS_OPT を足した版 (このコミット以降) でビルドして回す。
