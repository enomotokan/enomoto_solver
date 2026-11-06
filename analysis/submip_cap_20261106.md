# 1 ノードが重いサブ MIP が根を長くふさぐのを防ぐ (試作 `ENOMOTO_MIP_SUBCAP`、2026-11-06)

bench に使う環境変数: `ENOMOTO_MIP_SUBCAP=<文字>` (`n`・`w`・`j`・`t` の組み合わせ。計測したのは `nw`・`nwjt`)、
`ENOMOTO_T_MIP_SUBCAP_NODES` (200)、`ENOMOTO_T_MIP_SUBCAP_LEAVES` (500)、`ENOMOTO_T_MIP_SUBCAP_W` (1)、`ENOMOTO_T_MIP_SUBCAP_W_NODES` (10)、
`ENOMOTO_T_MIP_SUBCAP_W_SECS` (1)、`ENOMOTO_T_MIP_SUBCAP_J` (1)、`ENOMOTO_T_MIP_SUBCAP_J_MIN` (1)、`ENOMOTO_T_MIP_SUBCAP_T` (2)、`ENOMOTO_T_MIP_SUBCAP_T_MIN` (2)。
どれも既定では使わない (既定の動きは変えていない)。

- 機械: 4 CPU。ソルバーは同時に 1 つ (`submip_cap_20261106/queue.sh` の待ち行列)。ENOMOTO は 4 スレッド、`ENOMOTO_MIP_LOG=1`、種 `ENOMOTO_T_MIP_SEED`。
- ビルド: `submip_cap_20261106/build.txt` のコミット (上流 fc706a5f + この試作) の wheel。21:10 に `w` の基準の直し (2 節) を入れた wheel に替えた。
  supportcase42 の `nwjt0` (直す前) と `def` の種 1 (wheel を作る間に走った) はその前のもの。
- 台本: `run1.py` (1 回)、`queue.sh` (待ち行列)、`bench58.py`・`cfgs.txt` (58 問)、`summ.py` (集計)。結果は `res.jsonl`・`bench58.jsonl`、ログは `logs/` (gzip)。

## 1. サブ MIP の止め方の一覧 (事実、ソースから)

外側の根のサブ MIP は、どれも `solve_submip_problem` (`heuristics.rs`) を通る。根のヒューリスティクスの並び (`solver.rs` の 7817〜8330 行) では、
`root reduced cost`・`LP face`・`RENS`・`RENS (LP before cuts)`・crossover・DINS・vbounds (loose/tight) などの**サブ MIP は並列** (別スレッド、
同時に最大 `スレッド数 - 1` 本、`submip_par_limit`)。並列の区間の終わり (`vbounds` の後) で `join_submips(0)` で**全部を待つ**。その後の
`RENS (more nodes)`・`RINS`・`objective support` などは**主のスレッドで逐次**に解く。木の中の大近傍探索 (ALNS、`tree ...`) も逐次。

| 種類 | ノード上限 | 時間の上限 | 成果連動 (`submip_outcome_stop`) | 停滞 12 (`SUBMIP_STALL`) | 待ち |
|---|---|---|---|---|---|
| 根の並列のサブ MIP (被約費用・LP face・RENS・RENS (LP before cuts)・vbounds など) | 500 (多くの呼び出し。local branching 1000) | **残り時間** (`SUBMIP_HIGHS` の `t`)。出すときに残り時間で頭を押さえる | 外側の上限より良い解を見つけた後だけ (k から max(k, 50) ノード) | サブ MIP に暫定解があるときだけ | `join_submips(0)` で全部の終わりを待つ (上限なし。試作 `TREE3_JOINCAP`・`LOSE6_1_STOP`・`LOSE6_3_JOINSTOP` だけ) |
| 根の主のスレッドのサブ MIP (RENS (more nodes)・RINS・objective support・暫定解のない RENS のダイブ (`dive_neighbourhood`) など) | 500 (RENS (more nodes) は 1 回目の 4 倍で続きから、2000) | **残り時間** | 同上 | 同上 | 主が待つ (根はその間止まる) |
| 木の大近傍探索 (RINS・local branching・proximity・mutation・crossover・DINS) | 500 (local branching 1000) | min(残り × 0.07, 6 秒) と大近傍探索の予算 (2 × 残りの予算、下限 0.3 秒) | ない (`root_submip` は根だけ) | 同上 | 主が待つ (時間の上限が短い) |
| `heur_scip.rs` の min relaxation・ほか | 1000 / 500 | min(残り × 0.1, 10 秒) / min(残り × 0.07, 6 秒) | ない | 同上 | - |
| サブ MIP の中のサブ MIP (入れ子の RENS) | 50 | 残りの 15% (最大 3 秒) | ない | 同上 | - |

1 ノードが重いときに長く回る経路 (事実は過去の記録、経路の特定はソースから):

1. **根で、解を見つけない (または外側の上限より悪い解しか見つけない) サブ MIP**: 成果連動の打ち切りは「外側の上限より良い解」を見つけるまで効かず、
   停滞もサブ MIP に暫定解がないと数えない。残るのは 500 ノードと残り時間だけなので、1 ノード t 秒なら 500t 秒 (残り時間まで) 回る。
   neos-873061 の根の被約費用 (109 ノード・184〜187 秒、並列)、thor50dday の暫定解のない RENS のダイブ (286 秒、主のスレッド)、
   cod105 の RINS (76〜79 秒、主のスレッド。今日 `S4_RINSRATE` で飛ばすようにした) がこれ。
2. **根で並列のサブ MIP を全部待つ所**: 主のスレッドの根のヒューリスティクスが先に終わっても、`join_submips(0)` で残り時間の上限のサブ MIP を待つ
   (thor50dday・trento1・h80x6320d・tbfp-network。`TREE3_JOINCAP` で上限を試作済み)。
3. **根で良い解を見つけ続けるサブ MIP**: 成果連動の打ち切りは「見つけるまでと同じだけ進めて良くならなければ止める」なので、1 ノードが重く解を出し続けると
   長い (supportcase42 の RINS: 201 ノード・54〜58 秒、成果連動で止まる)。
4. 木の大近傍探索は 6 秒と予算で止まるので、根のような長いふさぎにはならない (この記録では触らない)。

## 2. 試作 (`ENOMOTO_MIP_SUBCAP`、どれも外側の根のサブ MIP だけ)

| 文字 | 中身 | 場所 |
|---|---|---|
| `n` | (a) ノード上限を min(今の上限, 200 + 外側のノード数 / 20) にし (HiGHS の `solveSubMip`。根では 200)、葉 (`objlim_leaves + infeasible_leaves`、木に入ってからの差) が 500 で止める。`RENS (more nodes)` の続きは 200 の 4 倍 (800) まで | `heuristics.rs` の `solve_submip_problem` (`node_limit`)、`solver.rs` のノードの繰り返しの頭 |
| `w` | (b) サブ MIP の木に入ってからの秒が max(1 秒, k × 外側の根の LP の秒 × max(進んだノード数, 10)) を超えたら止める (k = 1)。1 つのノードで止まっている間も `time_up` で見る。根の LP の秒は、前処理の LP による固定の最初の LP の秒の方が長ければそちら (supportcase42 は根の LP が温め直しで 0.05 秒、最初の LP は 28.7 秒) | `solver.rs` の `subcap_heavy`・`SubmipCtl::with_subcap`・`SUBCAP_PROBE_SECS` |
| `j` | (c) 根で並列のサブ MIP を全部待つとき、待ち始めてから max(1 秒, 1 × 根の時間) 過ぎても終わらないものに止めの合図 (その時点の解を返す)。根の時間 = 求解の始めから最初の根のヒューリスティクスまで (前処理・根の LP・カット)。`TREE3_JOINCAP` は「0.25 × 根の始めから待ち始めまで」 | `solver.rs` の `join_submips` |
| `t` | (d) 時間の上限を、残り時間ではなく min(今の上限, max(2 秒, 2 × 根の時間)) にする (`RENS (more nodes)` の続きも) | `heuristics.rs` の `submip_time_limit` |

止めたサブ MIP は `TimeLimit` で返る (葉・重さ・待ち。`RENS (more nodes)` を呼ばない)。ログ: `MIP: SUBCAP n: ...`・`MIP: SUBCAP w: ...`・
`MIP: ENOMOTO_MIP_TREE3_JOINCAP: stopping ...` (`j` も同じ行)。(終わりの集計行も書いたつもりだったが `mod.rs` への差し込みが失敗していて、計測の wheel には入っていない。`SUBCAP_STOPS` は数えるだけ)

## 3. 確かめ (10 問、4 スレッド・300 秒)

(計測中)

## 4. 58 問 (4 スレッド・60 秒、2 回)

(計測中)
