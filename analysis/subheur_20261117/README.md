# サブ MIP の中の解探し (2026-11-17)

背景は analysis/primal_20261117/README.md の 8・9 節。tbfp-network の根の RENS の部分問題を当方のサブ MIP の形
(`ENOMOTO_MIP_DEBUG_AS_SUBMIP`: 入れ子の深さ 1・1 スレッド) で解くと、trivial の解しか出ない。HiGHS はサブ MIP の中のサブ MIP で
72 秒に 29.2、当方の通常の形 (4 スレッド) は Local-MIP で 29.19 を出す。

手元・コミット 23eac6062 の上 (highspy 1.15.1、4 コア)。ログは `logs/`。部分問題の MPS は手元の一時置き場に書き出し、コミットしない。

## 1. サブ MIP の中で止まっている解探し (src/mip/、コードから)

`params.submip` はサブ MIP の中 (どの深さでも) で真、`SUBMIP_DEPTH` は入れ子の深さ (外側の根の RENS の中が 1)。

| 解探し | サブ MIP の中 | 止めている所・条件 |
|---|---|---|
| FJ・Local-MIP (根の別スレッド、`fj_lmip_mix`)・前処理の間の FJ (`PRECHAIN`) | 回さない | solver.rs `run` の FJ の分岐 `!self.params.submip && mip_threads() > 1` |
| FJ (1 スレッドの場合、`feasibility_jump`) | 回さない | 同じ分岐の `else if !self.params.submip` |
| 解発見のスレッド (`find_thread`)・`LMBOOST`・`FJKEEP` | 回さない | `run` の根のカットのループの前、どれも `!self.params.submip` |
| simple rounding・ZI round・randomized rounding | 回す | — |
| **trivial** | 回す (暫定解がなければ) | — (**サブ MIP の根では、ほぼいつもここで質の悪い解が出る**) |
| interior rounding | 回さない | heur_ipm.rs `interior_rounding` の入口 |
| root reduced cost | 回さない (`SUBMIP_NESTED` の `x` で深さ 1 のみ) | heuristics.rs `root_reduced_cost_inner` |
| LP face | 回さない | heur_scip.rs `lp_face` |
| RENS (入れ子) | 暫定解がなく打ち切り値が無限のときだけ (`SUBMIP_HIGHS` の `h`・深さ 1〜2) / 打ち切り値が有限なら深さ 1 で全部固定の形 | heuristics.rs `rens`: `nested_rens_ok` は `self.incumbent.is_none()` が条件 → **trivial の解があると回らない** |
| Feasibility Pump・分数/ベクトル長/係数ダイビング | 暫定解がないときだけ (`noinc_like`) | `run` の根のヒューリスティクス → **trivial の解があると回らない** |
| shift-and-propagate | 暫定解がないときだけ | 同上 |
| locks・clique・vbounds | 回さない (`SUBMIP_NESTED` の `l`・`c`・`v` で深さ 1 のみ) | heur_scip.rs 各入口 |
| local search・repair | 回さない | heur_scip.rs `local_search_heur`・`repair` |
| RINS | 回さない (`SUBMIP_NESTED` の `i` で深さ 1 のみ) | heuristics.rs `rins_inner` |
| RENS (more nodes)・RENS (LP before cuts) | 条件つき (LPBC は深さ 1 未満のみ) | `run` |
| 木の中: rounding・ZI round・shifting・randomized rounding | 回す | `run` の木のループ |
| 木の中: ALNS (RINS・RENS・mutation などの腕) | 呼ぶが、腕のサブ MIP は入口で止まり何もしない | `alns` の腕 → `rins_inner`・`local_branching`・`proximity_search`・`mutation` の `params.submip` |
| crossover・DINS・local branching (2)・proximity (2)・fixmore・LNS の銀行 | 回さない | heur_scip.rs・heur_lns2.rs・fixmore.rs の入口 |

要点: サブ MIP の根では **trivial が最初に解を出す** ので、暫定解なし用の手 (入れ子の RENS、ポンプ、3 つのダイビング、shift-and-propagate)
がどれも呼ばれない。tbfp の RENS の部分問題では trivial の 43.1 (自由な整数列 36362 の部分問題) / 116.4 (8 節の部分問題) のまま木に入る。
暫定解があるときに回る改善の手 (RINS・locks など) は既定ではサブ MIP の中で止めてあるので、trivial の解は木の中の丸めでしか良くならない。

## 2. 試作 `ENOMOTO_MIP_SUBMIP_HEUR` (既定では使わない)

- `t`: サブ MIP の根で trivial を根のヒューリスティクスの最後に回す (先に入れ子の RENS・ポンプ・ダイビング・shift-and-propagate を回す)。
  それぞれの手の予算は今のまま (RENS はノード数、ポンプ・ダイビングは LP の反復数で、どれも時間でなく仕事量で決まる)。
- `l`: 深さ 1 のサブ MIP の根の終わりに Local-MIP (連鎖の手あり) を回す。予算は `ENOMOTO_T_MIP_SUBHEUR_LM_WORK` (既定 200) × 非零の数を、
  Local-MIP の 1 手の仕事量の見積もりで割った手数 (時間でなく手数で止める、0.05 秒の切れごとに手数を見る)。`s` を足すと暫定解から始める。

### 書き出した部分問題で (サブ MIP の形・1 スレッド・120 秒、打ち切り値なし)

部分問題: tbfp-network の根の部分問題 (`ENOMOTO_MIP_PRIMAL=hs`・種 1 の回の 0 番、2436 行・自由な整数列 36362、手元の一時置き場)。

| 形 | 最初の解 | 120 秒の暫定解 | 下界 |
|---|---|---|---|
| 既定 | trivial 43.13 (11 秒) | 43.13 | 23.26 |
| `t` | ベクトル長ダイビング 28.92 (47 秒。入れ子の RENS 3 回 9 秒・ポンプ 15 秒・分数ダイビング 8.5 秒は解なし) | **28.92** | 23.46 |
| `l` (予算 200 × nnz = 7409 手、1.3 秒) | trivial 43.13 | 43.13 | 23.26 |
| `l` (3000 × nnz = 11 万手、22 秒) | trivial 43.13 | 43.13 | |
| `l` (20000 × nnz、106 秒で 45 万手) | trivial 43.13 → Local-MIP 43.12 (27 秒) … 40.49 (106 秒) | 40.49 | |
| `tl` | `t` と同じ 28.92 (Local-MIP 7552 手は改善なし) | 28.92 | 23.41 |

(参考: 8 節では HiGHS が同種の部分問題で 72 秒に 29.2、当方の通常の形 (4 スレッド) が Local-MIP で 29.19。)

- **`t` が効く**: trivial を後回しにすると、暫定解なし用のベクトル長ダイビングが 28.92 を出す (HiGHS の 29.2 より良い)。
- **`l` は効きが弱い**: サブ MIP の形の Local-MIP (元の部分問題、出発点は下限) は、予算を 100 倍にしても 40.49 まで。通常の形の 29.19 は、
  FJ_P3 の前処理をかけた問題で FJ と混ぜて回したもの。サブ MIP の中で同じことをするのは重いので、`l` はここで止める。
- sat の部分問題 (`satellites2-60-fs` の同じ回の 1 番、自由な整数列 10676) は既定のサブ MIP の形でも 1.3 秒で最適 19 を出し、比べにならなかった。
  全体の satellites2-60-fs をサブ MIP の形で解くと既定は trivial の 80 のまま (120 秒)。

## 3. 本体の 4 問 (4 スレッド・600 秒・種 1、`run4.py`、結果は `res4.jsonl`)

| 問題 | 形 | 最初の解 (秒・値・手) | 最後の改善 (秒・値・手) | 終わり | 時間 |
|---|---|---|---|---|---|
| tbfp-network | 既定 | 17.7・74.29・Local-MIP | 423.5・24.163・木 | 最適 | 438.8 |
| tbfp-network | `t` | 18.3・74.29・Local-MIP | 501.9・24.163・木 | 最適 | 509.8 |
| satellites2-60-fs | 既定 | 38.0・38・Local-MIP | 95.3・19・locks | 時間切れ (下界 -29) | 600 |
| satellites2-60-fs | `t` | 35.0・38・Local-MIP | 92.7・19・locks | 時間切れ (下界 -29) | 600 |
| neos-3216931-puriri | 既定 | 138.2・81380・locks | 同じ | 時間切れ | 600 |
| neos-3216931-puriri | `t` | 138.1・81380・locks | 同じ | 時間切れ | 600 |
| neos-873061 | 既定 | 21.6・125.93・木 | 224.6・117.77・root reduced cost | 時間切れ | 600 |
| neos-873061 | `t` | 21.9・125.93・木 | 225.4・117.77・root reduced cost | 時間切れ | 600 |

- **本体では `t` は効かない** (解の値・時刻は揺れの範囲で同じ)。tbfp の 71 秒の差は、根の終わりが 92 秒 → 159 秒にずれたことによる
  (サブ MIP の手とは関係しない、4 スレッドの揺れ)。
- 理由: 本体の根の RENS (並列のサブ MIP、自由な列 14539 / 7493) は、tbfp では 21 秒・0 ノードで止められ (`TimeLimit`)、
  サブ MIP の根のヒューリスティクスまで届かない。sat では部分問題がすぐ実行不能で終わる。どちらも `t` の効く所 (サブ MIP の根の
  ヒューリスティクスの順) に来ない。書き出した部分問題で `t` が効いたのは、サブ MIP を 1 スレッド・120 秒で最後まで回したから。
- そのため P・M の対の台本・fuzz の台本は作らない (効けば作る、という依頼の条件に当たらない)。手を入れるなら、サブ MIP の打ち切り
  (並列 RENS の 21 秒) や、サブ MIP の根に届くまでの前処理・LP の重さが先。

cargo test --release --lib: 416 通過。
