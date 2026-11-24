# ti6: traininstance6 で打ち切り値を与えたときの報告値 (2026-11-24、親の依頼、調べただけで直していない)

材料: analysis/treegap_20261123 (res_tg.jsonl の 13・14 行目)。打ち切り値 `ENOMOTO_MIP_CUTOFF=28290.02829` (最適値 28290) を与えた回で、
(1) 終わりの下界が 3636 → 118 に下がる、(2) 解が見つかっていない (`first_solution_*` が null) のに結果の `objective` が 41160 になる。

再現: `ENOMOTO_MIP_CUTOFF=28290.02829 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_LOG=1 python3 scripts/run_miplib_benchmark.py
--worker-ours .miplib_cache/traininstance6.mps --time-limit 20 --threads 1` で、20 秒でも `objective` 41160・`first_solution_objective` null
(ログ `ti6_cut_20s.log.gz`)。60 秒も同じ。

## 1. (2) objective の 41160 の出どころ (事実)

- 前処理の前に VB01 (`VB01 (loose, best): sub-MIP solution 4.1160000000e4`) が元の問題で 41160 の解を見つけ、前処理後の問題では実行不能に
  なる (`the start solution is not feasible in the presolved problem; dropped`) ので、`ENOMOTO_MIP_TPRE` の道に入る:
  `MIP: TPRE: keeping the solution 4.1160000000e4 of the original problem as the cutoff and the fallback answer` (`src/mip/mod.rs` の
  `tpre_orig`)。
- 木の打ち切り値は `params.cutoff.min(u)` (u = 41160) で、利用者の打ち切り値 28290.03 のほうが小さいのでそれが使われる。木は 28290.03 より
  良い解を見つけないので、暫定解は最後までない (`first_solution_*` が null なのはこのため)。
- 終わりに `tpre_patch` が、木の解がない (`r.x.is_none()`) ので x_orig (41160) を結果に入れる。Python の結果 (`src/model.rs`) は `x` が
  あれば `objective` を入れるので 41160 になる。打ち切り値そのものや何かの残りではなく、**利用者の打ち切り値より悪い予備の解**。

### 誤りかどうか

- 返す点そのものは元の問題で実行可能 (`p.is_feasible`) なので、値が嘘というわけではない。ただし「打ち切り値より良い解だけを求める」という
  打ち切り値の意味からは外れ、`first_solution_*` とも食い違う (SCIP は解なしを返す)。
- **正しさに関わる場合がある** (コードからの推論、再現はしていない): `tpre_patch` は木の結果が `Infeasible`・`Optimal`・
  `InfeasibleOrUnbounded` なら状態を `Optimal`・下界を u にする。打ち切り値が u のときは「u より良い解はない」なので正しいが、利用者の
  打ち切り値 c が u より小さいときは、木が示したのは「c より良い解はない」だけで、c と u の間 (今回なら 28290〜41160) は調べていない。
  そこで x_orig を「最適」と返すのは誤り (例: 最適値より少し下の打ち切り値を与えて木が実行不能で終われば、41160 を最適と返す)。

### 直し方の案

- `tpre_orig` を作るとき、u が利用者の打ち切り値 c (TPRE の前の `params.cutoff`) 以上なら、x_orig を予備の答えにしない
  (`tpre_orig = None`。打ち切り値も c のまま)。打ち切り値より悪い解は返さず、木の結果 (時間切れなら解なし、実行不能なら「c より良い解はない」)
  をそのまま返す。u < c のときは今までどおり (木の打ち切り値は u で、x_orig を予備の答え・「u より良い解はない」なら最適)。
- 確かめ: traininstance6 の打ち切り値つき 20 秒で `objective` が null になること、打ち切り値なしでは今までどおり 41160 を予備の答えに持つこと、
  `ENOMOTO_MIP_TPRE_FORCE_ORIG` の fuzz (TPRE の道を通す) と MIP の fuzz 7 本、cargo test。

## 2. (1) 下界が下がる理由 (コードからの推論)

- 木の中の「打ち切り値に使える値があるか」の判定 `Solver::has_cutoff_value` (`src/mip/solver.rs`) は、自分の暫定解 (木を分け合うときは
  ほかのスレッドの共有の解) があるときだけ真で、**利用者の打ち切り値 (`params.cutoff`) は入らない**。
- 打ち切り値なしの回は 6 秒目に暫定解 (36xxx、`first_solution_time` 6.19 秒) が見つかり、以後この判定は真。打ち切り値つきの回は 28290.03 より
  良い解が 600 秒見つからず、判定はずっと偽。偽のあいだは:
  - ノードの被約費用による固定 (`local_redcost_fixing`、`rc_fixed = self.has_cutoff_value() && ...`) をしない、
  - 下界最小のノードを取る頻度が 10 回に 1 回 (`ENOMOTO_T_MIP_BB_EVERY`) で、暫定解があるときの 4 回に 1 回 (`..._BB_EVERY_INC`) より少ない、
  など。打ち切り値 (28290) は暫定解の値 (36xxx) より厳しいのに、それを使う処理が止まり、深さ寄りの探索になって下界が上がらない
  (3636 → 118) とみる。下界 118 自体は最適値以下で正しい (**正しさの問題ではなく効率の問題**)。
- 一方、根の目的のプロービングなど (`21454` 行) は `self.has_cutoff_value() || self.params.cutoff.is_finite()` で打ち切り値も数えていて、
  扱いがそろっていない。

### 直し方の案

- `has_cutoff_value` に `self.params.cutoff.is_finite()` を加える (打ち切り値は `prune_limit` にすでに入っているので、被約費用の固定・
  ノードの選び方もそれを使える)。ただし TPRE の打ち切り値 (u) や、再スタートで引き継ぐ打ち切り値でも真になるので、打ち切り値なしの普段の
  回の挙動も変わる (traininstance6 の打ち切り値なしの回も最初の 6 秒が変わる)。既定を変える前に P・M の計測が要る。
  試作にするなら環境変数で切り替える形 (既定オフ、新しい閾値なし)。
