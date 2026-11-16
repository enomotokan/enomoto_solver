# 調べ B: 最適解は早く持つのに証明に HiGHS の 5〜10 倍の節点が要る 6 問 (treeclose 9.52 節、親の依頼)

対象 (cand1118 の M): neos-3627168-kasai・csched008・unitcal_7・sct2・net12・supportcase33。最適値は probsB.py。
1 分担 = 1 問 × {ours, ours_cut, highs, highs_cut}、600 秒・4 スレッド・種 1。*_cut は最適値を打ち切り値として与える
(ours: `ENOMOTO_MIP_CUTOFF`、HiGHS: `objective_bound`、整数の目的値は最適値 − 0.5) ので、解を探す差を除いて証明だけを比べられる。
ログ (ours は `ENOMOTO_MIP_LOG=1`、HiGHS は自分のログ) を logs/ に gzip で全部残す。

- `python runB.py K` (K = 0〜5、6 分担、各最大約 42 分)。既定 (9ab8b789b 以降 = cand1118 が既定) でビルドして回す。途中から再開できる。
- 集計 `python summB.py > summ.txt` (状態・秒・節点数・根の下界、根のギャップが 1/2・1/4・1/10・1/100 になった節点数と秒)。
