# 0.1.9 の既定と SCIP の比べ (2026-11-17、利用者の依頼「SCIP との比較も頼む」)

HiGHS との比べ (analysis/vshighs_20261117/) と同じ問題・同じ形で、当方 0.1.9 の既定と SCIP を、同じ機械・同じ種で組にして比べる。
台本は vshighs_20261117 の写し。ソルバーは同時に 1 つ。途中から再開できる。

## 設定 (cfg.py)

- def: 当方の既定 (環境変数なし)、4 スレッド (ENOMOTO_MIP_THREADS=4)。
- def_t1: 当方の既定、1 スレッド (ENOMOTO_MIP_THREADS=1。run.py の TH は analysis/clqlazy_20261117/run.py の `_t1` と同じ扱い)。
- scip: pyscipopt 6.3.0 の wheel に同梱の **SCIP 10.0.2** (LP は SoPlex 8.0.2)。`scripts/run_miplib_benchmark.py --worker-scip`。
  - **SCIP は既定で逐次 (1 スレッド)**。並列版 (FiberSCIP・concurrent) は使わない。なので公平な比べは def_t1 / scip、def / scip は「当方の 4 スレッド対 SCIP の 1 スレッド」。
  - 時間の上限 limits/time = 60 秒 (P) / 600 秒 (M)。時間は `optimize()` の経過時間 (当方・HiGHS と同じく読み込みを除く)。
  - 相対ギャップ limits/gap = 0 (HiGHS の mip_rel_gap = 0・当方の既定にそろえる。SCIP の既定も 0 なので既定のまま)。絶対ギャップ limits/absgap も既定 (0)。
  - メモリの上限は付けない (limits/memory は既定の無限大)。出力は hideOutput で抑える (ENOMOTO_BENCH_SCIP_LOG=1 で出す)。
  - 乱数の種: randomization/randomseedshift = 種 (ENOMOTO_BENCH_SCIP_SEED。当方は ENOMOTO_T_MIP_SEED)。
  - 状態は当方の名前にそろえる: optimal / infeasible / time_limit / unbounded / infeasible_or_unbounded など。行には scip_version・pyscipopt_version も残す。
- 同じ (問題, 種) の 3 設定は同じ分担で続けて解き、順は (問題, 種) ごとに回して入れ替える。

## 問題と分担 (plan.py)

- M: Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) × 種 1 × {def, def_t1, scip}、600 秒、**50 分担** (見積り各 1.51〜1.57 時間、計 77.7 時間): `python run.py M K` (K = 0〜49)
- P: MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {def, def_t1, scip}、60 秒、**4 分担** (見積り各 1.3 時間): `python run.py P K` (K = 0〜3)
- 見積り: vshighs_20261117 の def の時間 (res_M_s*・res_A_s*・res_P_s* の種の平均)。def_t1 と scip は不明なので min(2 × def, 上限) と置き、
  def で解けなかった問題は上限。1 回ごとに M は +30 秒、P は +8 秒。作り直しは `python plan.py 50 4`。
  SCIP が 600 秒で解けない問題が多いと見積りより長くなりうる (上限は 3 × 630 秒 = 1 組 0.53 時間なので、最悪でも 1 分担は 1.5 時間前後 × 数割)。

## 準備

```
pip install maturin highspy==1.15.1 pyscipopt && pip install -e .
```

(この台本のコミット以降でビルド。pyscipopt は 6.3.0 = SCIP 10.0.2 が入ることを確かめた。MPS は当方が HiGHS で読むので highspy も要る。)

## 集計 (summ.py)

`python summ.py M > summ_M.txt`、`python summ.py P > summ_P.txt`。
- 3 者そろった (問題, 種) で、解けた数、状態の内訳、食い違い (最適値の不一致・最適と実行不能の衝突。参考の HiGHS・vshighs の def も含めて見る)、最大 RSS。
- def / scip・def_t1 / scip (と def / def_t1) の SGM (shift 1 秒、時間切れは上限で数える) と問題単位のブートストラップの 95% CI、片方だけ解けた数。負なら当方が速い。
- 参考の表: vshighs_20261117 の HiGHS 1.15.1 (4 スレッド) と当方 def を同じ (問題, 種) で並べる。**機械・時期が違うので参考**。

## 手元の確認 (L)

shards_L.json (blend2・dcmulti・gt2 × 種 1・2 × 3 設定、60 秒) を `python run.py L 0` で回し、`python summ.py L` の出力を local_L.txt に残した。
3 設定とも 6 組すべて optimal、最適値は 3 者・参考の HiGHS・vshighs の def と一致 (食い違い 0)、crash 0。
gt2 は 0.1 秒で終わるので RSS の見張り (0.5 秒ごと) に掛からず 0 MB と出る (短い問題の RSS は当てにならない)。
