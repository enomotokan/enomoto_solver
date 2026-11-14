## 洗い出し A: src/mip/solver.rs 1〜11000 行目

grep の当たり 491 行。うち約 370 行は表示・統計だけ (ログの秒、`t0`/`elapsed` の対、TSDIAG・OVERRUN3・XOLAZY・TRSTAT・PROBE_BASIS の診断、`last_log` (7230)、`tprof`/`t_sync` (10644・10725)、`prog_at` (4262)、pool1/pool3_diag (1307・7705)、GAP4_RSS (2218〜2247)、DEBUG_LMIP/DEBUG_FJ_VS_LMIP)。
先例 2 つは既定化済み: `ENOMOTO_MIP_BG_CUTS_DETERM` 既定 `c` (4131)、`BG_CUTS_LP_WORK` 既定 `1` (separation.rs:1628、秒の上限は 5 倍の安全網)。

| # | 場所 | 何を決めているか | 既定値・環境変数 | 有効 | 測り方 | 種別 | 手間に置き換える案 | 響き |
|---|---|---|---|---|---|---|---|---|
| A1 | solver.rs:9776-9792 (止める先 separation.rs:3468) | ERWAIT `b`+`k`: 早めの再スタートが決まった根で裏のカットを最大 2 秒待ち、まだ回っていれば止めて最後に終わったラウンド (`last_ok`) の行と基底を使う。取り込むラウンド数が秒で決まる | `ENOMOTO_T_MIP_ERWAIT_BGWAIT`=2.0 | cand1114 | 壁時計・絶対秒 | 分かれ道・量 | 裏の LP 反復の累計 `bg.job.prog_it` (単位 `d.unit` = 根の LP の反復) が k×unit か、ラウンド数 `prog.len()` が R に届くまで待つ | 高 (早めの再スタートの問題全部。根の下界と再スタートでの固定の数が変わる) |
| A2 | 9587 → 実体 19597-19603 | ERWAIT `s`: 並列のサブ MIP を 0.5 秒待ってから止める。それまでの解を受け取る | `ENOMOTO_T_MIP_LOSE6_1_GRACE`=0.5 | cand1114 | 壁時計・絶対秒 | 分かれ道 | サブ MIP の進んだノード数で止める (SubmipCtl にノード数の原子変数) | 高〜中 |
| A3 | 7911 (`cap`)・8004 (`FJ_FEAS_FRAC`)・8066 (`fjl_cap`)・8076-8077 (`nosol_cap`)・8135/8146 (FJ と Local-MIP の時間の分け方、実体 heuristics.rs:3607〜)・8925 (`join_fj_thread`) | FJ / Local-MIP のスレッドの量。根の LP・カットと同時に走り、時間上限・主のカットのループの終わり・解のない間の延長のどれかで止まる | cap=min(0.05×TL, 5 秒) (実行可能性問題は 0.5×TL)、`LOCALMIP_TIME_FRAC` 0.1 (10 秒まで)、`FJ_NOSOL_EXTEND`="2" で 0.25×TL−経過、`FJ_LMIP_MIX` | 既定 | 壁時計 | 量 (`inc_only_fj`・`fj_poor` を通して分かれ道にも) | FJ の `effort = 50×nnz` (7912)、Local-MIP の `lr.steps` で予算を手数にし、主はカットの終わりで規定の手数まで待つ | 高 (暫定解が FJ だけか → 解探しのスレッド・裏のカットのスレッド数・ROOTSTOP `b` (8975)・LMBOOST が変わる) |
| A4 | 6277-6281 `tree_sep_budget_ok` (呼ぶ側 12177・12179) | 木の中の分離をするか・2 ラウンド目を続けるか (ノードごと) | `TREE_SEP_TIME_FRAC`=0.1×経過、`TREE_SEP_LP_FRAC`=0.25×`node_lp_secs` | 既定 | 壁時計・割合 | 分かれ道 | `node_cut_iters` (分離後の解き直しの反復) を新たに数え、`node_iters` (11518) と比べる | 高 (木に入る問題全部) |
| A5 | 4733-4734 (使うのは heuristics.rs:1512) | 根・木のサブ MIP の時間上限 = 0.07×残り、最大 6 秒。種類ごとに 2124〜3482 で割合を差し替え | `SUBMIP_TIME_FRAC`=0.07、`SUBMIP_TIME_MAX`=6.0 | 既定 | 壁時計 | 量 (重いサブ MIP で普段から当たる) | ノード上限に加えて LP 反復の上限 = k×外側の `root_iters` を MipParams に渡し、秒は安全網 | 高〜中 |
| A6 | 9010-9012 (`root_lp_secs` は 8696) | BG_CUTS_BASIS_X=lpw: 根の LP に 1 秒以上かかった問題だけ裏のカットに列の値つき基底を渡す | 既定 "lpw"、`T_..._BASIS_X_SECS`=1.0 | 既定 | 壁時計・絶対秒 (根の LP は単体法と内点法の競争) | 分かれ道 | `root_iters` か 反復×(行+非零) | 中 (根の LP 1 秒前後の問題で機械により逆転) |
| A7 | 9833-9835 (`wait_determ` 4436) | 裏のカット (determ) の待ちの安全網 min(0.3×残り, 残り−0.05×TL) | `BG_DETERM_SAFE_FRAC`=0.3、`BG_CUTS_WAIT_MIN_LEFT`=0.05 | 既定 | 壁時計 | 安全網 (ときどき当たる) | 既に手間の上限 (20×根の LP 反復) あり。残す | 低〜中 (ログ why=`cap`) |
| A8 | 8942-8944 | 解探しのスレッドの時間上限 min(0.3×残り, 20 秒)。暫定解が質の悪い FJ の解だけ・3 スレッド以上で起こす | `FIND_THREAD_FRAC`=0.3、`_MAX`=20 | 既定 (条件付き) | 壁時計 | 量 | ノード・反復の上限を渡す (`dive_budget = 2×root_iters+1000` は反復) | 中 |
| A9 | 6858-6861 | 暫定解の整数列を固定した LP の上限 max(0.02×TL, 1 秒) | `INC_LP_TIME_FRAC`=0.02 (FJ_P3 既定) | 既定 | 壁時計 | 量 (重い LP だけ) | `SolveLimits.iteration_limit` = k×`root_iters` | 低〜中 |
| A10 | 10633-10636 | 並列木: 主がノードを従に渡し始めるのは待ち行列 8 個か 1 秒経過 | `TREE_HOLD_SECS`=1.0、`TREE_START_NODES`=8 | 既定 (2 スレッド以上) | 壁時計 | 分かれ道 | ノード数だけ、または `node_iters` の増え ≥ k×根の LP 反復 | 中 (並列の木はもともと非決定的) |
| A11 | 9383 (使うのは 20220-20224) | 根のヒューリスティクスの「利得/秒」を木のヒューリスティクス役の起動判定の初期値に | 既定 (4 スレッド以上) | 既定 | 壁時計 | 量 (初期値) | 分母を LP 反復 (`heur_iters`/`dive_iters` の差分) に | 低〜中 |
| A12 | 6758 (判定 19856-19873) | LMBOOST を起こすか: 経過 ≥ 10 秒、最後の改善 ≥ 0.5×経過。止め方も 0.5×t の停滞と 0.25×TL | `LMBOOST_MIN_SECS`=10、`_RECENT`=0.5、`_STALL`、`_FRAC` | cand1114 | 壁時計 | 分かれ道 | 改善時点の `lp.total_iterations()` か Local-MIP の `steps` | 中 |
| A13 | 10514 (`tr_t0`)、使うのは 21494・21516 | TREEEFF `r` の木の中の再スタートのガード (木の秒・残りの見込み・根の秒) | `TREE_RESTART_GUARD_ROOT`=1.0 ほか | cand1114 | 壁時計 | 分かれ道 | 木に入った時点の `lp.total_iterations()` と `node_iters` | 中〜高 |
| A14 | 9897-9908 | 裏のカットの待ち (determ でない方) | `BG_CUTS_WAIT_*` | `BG_CUTS_DETERM=0` のときだけ | 壁時計 | 分かれ道 | determ に任せる | 低 (通らない) |
| A15 | 6229-6230・6133・6163-6170・6204・6292・6306-6308 | ノードの分離の頻度 (時間の割合) | `NODE_CUT_TIME_FRAC` ほか | 試作のみ | | 分かれ道 | `node_cut_iters` | 低 |
| A16 | 9268-9275・9329-9331 | 根のヒューリスティクス全体・ダイビング 1 回の時間予算 | 既定 0 | 試作のみ | | 量 | `dive_budget` | 低 |
| A17 | 9252-9263・9290・9344・6031・9858・9337-9338・10088-10091・3165・3171・3647-3654・5961-5975・6010-6025・10794・3677-3678 | ROOTCAP・TREE3_*・SUBCAP・SUBMIP_HIGHS_NOSOL の秒の上限 (多くは `root_lp_secs` の倍数) | | 試作のみ | | 量 | `root_lp_secs` → `root_iters` | 低 |
| A18 | 5595-5810・9071・8473-8476・8827-8839・9969-9983・10311-10316・10693-10697・9467・10570-10571・6050・2391 | ROOTLP_BOTH/OPTFACE_*・ROBUST_ROOTIPM・OPTFACE_PARCUTS・ROOT_LNS・DONATE_HALF・FPR r・TREE_RESTART=inc・BISECT_MIN_RUN・MEMCAP | | 試作のみ | | | | 低 |
| AS | 4493-4494 (`deadline`)、6034-6038 (`time_up`)、6057-6061、`lp.solve(deadline)` 各所、8590、9003、9126→separation.rs:2974 (`BG_ROOT_LP_SECS`)、9525、8009/8112/8124 | 全体の時間の上限・取り消し | | 既定 | | 安全網 (残す) | | |

補足 (秒は測らないが時刻の競争で揺れるもの): 根の LP の単体法と内点法の同時実行 (`root_lp_secs` もこれに引きずられる)、ROOTSTOP `b` (8975) は裏のカットを起こす時点の暫定解 (FJ しだい)、裏のカットの `S4_BGZERO` (4445) は主が見た時点のラウンド数。

診断: A1 は `MIP: ERWAIT: waited {w}s ... ({why}, {n} rounds)` (9783) の why=`cap`。A3 は `join_fj_thread` (19964) で止まった理由・手数を 1 行出すとよい。A4 は `tree_sep_budget_ok` が偽の回数を `adaptive node cuts` の行 (22269) に足す。A5 は `sub-MIP: ... status TimeLimit` (heuristics.rs:1762)。A2 は `LOSE6_1_STOP: stopping {n} of {m}` (19612)。A6 は `background cuts: basis ... (root LP {:.2}s)` (9014)。
