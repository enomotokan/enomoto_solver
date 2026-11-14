## 洗い出し B: src/mip/solver.rs 11000〜22637 行目

手間の数えとして範囲内で使えるもの: `self.lp.total_iterations()`、`node_iters`/`node_lps`、`sb_iters`/`sb_lps`、`heur_iters` (サブ MIP の `r.lp_iterations` 込み。heuristics.rs:1785・1867、solver.rs:19657)、`dive_iters`、`root_lp_iters`、`BG_ROOT_ITERS`、`self.nodes`/`global_node_counts()`。非零の演算数の物差しは無い。

| # | 場所 | 何を決めているか | 既定値・環境変数 | 有効 | 測り方 | 種別 | 手間に置き換える案 | 響き |
|---|---|---|---|---|---|---|---|---|
| B1 | solver.rs:11735-11754・11783・11804-11809・11831 | 木の大近傍探索 (ALNS/RINS など) を呼べるか (`lns_ok = lns_avail > 0`) と 1 回のサブ MIP の上限 `submip_time_max = min(6 s, 2×lns_avail, 下限 0.3 s)` | `T_MIP_LNS_TIME_MIN` 0.02 + `T_MIP_LNS_TIME_FRAC` 0.08×成功率、`T_MIP_LNS_CALL_MULT` 2、`T_MIP_LNS_CALL_MIN` 0.3、`T_MIP_LNS_FREQ` 50 (`MIP_LNS_ITERS` で以前の反復の予算に戻る) | 既定 | 壁時計・経過秒の割合、上限は絶対秒 | 分かれ道・量 | `heur_iters` と木の LP の反復 (`node_iters + sb_iters`)×frac の比べ。サブ MIP の上限はノード数か反復数 | 高 (暫定解があり 50 ノードを超える木ならほぼ全部。上限は最低 0.3 s と短い) |
| B2 | 12172-12181 (`ts_due`)、判定 6243-6282 | 木の分離をするか・続けるか (= A4) | `T_MIP_TREE_SEP_TIME_FRAC` 0.1、`T_MIP_TREE_SEP_LP_FRAC` 0.25 | 既定 (外側の主の求解のみ) | 壁時計 | 分かれ道 | `node_cut_secs` → 解き直しの反復 + 分離した行の非零、`node_lp_secs` → `node_iters` | 中〜高 |
| B3 | 12037-12075 | HiGHS 流のカット管理でノードの分離を続けるか | `T_MIP_HCM_NODE_SEP_FRAC` 0.1 (`MIP_HIGHS_CUTMGMT` か `hcm_auto`、separation.rs:1916、timtab1 型) | 既定 (自動で一部) | 壁時計 | 分かれ道 | B2 と同じ | 低〜中 |
| B4 | 11307-11329・11452-11465 | ノードの LP 1 回の上限 `max(50×平均の秒, 0.5 s)`。超えると全論理変数基底から解き直し、それも超えると LP なしで分枝 | 定数 50・0.5 s | 既定 | 壁時計 | 分かれ道 | 反復の上限 `iter_limit = max(10×avg_node_iters, 20000)` が既にある。秒は安全網として 5〜10 倍に | 中 (根の LP が重い問題で各スレッドの最初のノードが 0.5 s で切られる) |
| B5 | 13105-13137 (`heur_role_mine`)・13182-13238 (`heur_trigger_eval`)・13270-13306 (`mu_worth`)・11821-11835/13527-13549 (`mu_observe`) | ヒューリスティクス役のスレッドが役に就くか・木に戻るか (0.25 s おきに評価、木の完了の速さ (1/秒) から完了予測を出し残り秒と比べる。warm は木の秒 ≥ 1 s かつ全体の 5%) | `T_MIP_HEUR_TRIG_WARM_FRAC` 0.05、`T_MIP_HEUR_MU_STAGNANT` 4、`_KAPPA` 1、`T_MIP_HEUR_TRIG_ALPHA` 0.3、`_NODES` 100 | 既定 (4 スレッド以上) | 壁時計 | 分かれ道 | 速さの分母を木の LP の反復の合計に、`mu_observe` の dt は `heur_iters` の差に | 高 (複数スレッド) |
| B6 | 13496-13498・13525・13556-13562・13568-13577 | 役のサブ MIP 1 回の上限 5 s、木に戻った後の冷却 5 s (戻るたびに倍) | `T_MIP_HEUR_ROLE_CALL_MAX` 5、`_COOLDOWN` 5、`_PATIENCE` 20 | 既定 (4 スレッド以上) | 壁時計・絶対秒 | 量・分かれ道 | サブ MIP はノード数・反復数、冷却は木のノード数 | 中〜高 |
| B7 | 21486-21580 (`tree_restart_guard_ok` の `R`) | 木の中の再スタートを見送るか: 木の残りの見込み `est_r=(1−q)·Δ木の秒/Δq` が根のやり直しの見込み `tr_t0` (木に入るまでの秒) を下回れば見送る (= A13) | `T_MIP_TREE_RESTART_GUARD_COST` 1、`_ROOT` 1。cand1114 の `TREEEFF=r` でガード "KR" | cand1114 | 壁時計の秒どうし | 分かれ道 | Δ木の秒 → q を測った間の木の LP の反復、`tr_t0` → 根の LP + カットのループの反復 | 高 (cand1114。再スタートするかで結果が大きく変わる) |
| B8 | 21459-21461 | 木の中の再スタートは残りが上限の 0.1 倍以上のときだけ | `T_MIP_TREE_RESTART_MIN_LEFT` 0.1 | cand1114 | 割合 | 安全網 | 残す | 低 |
| B9 | 16631-16638・16689・16756 (`gap1_lp_probe`/`lp_probe_cutoff`) | 根で分数の 0-1 列を LP で試して固定する。tmax 秒で打ち切る | `T_MIP_GAP1_PROBE_TIME_FRAC` 0.1×TL | cand1114 (`GAP1=u`) | 壁時計・TL の割合 | 量 | `total_iterations() − it_all` ≤ k×`root_lp_iters` (`it_all` は既にあり未使用) | 中〜高 (大きな問題で当たる。固定の数→再スタート・根の下界) |
| B10 | 16773-16776・17187・17256・17269 (`root_obj_probe`/`obj_probe_core`) | 打ち切り値つきの根のプロービング (伝播だけ) の合計時間 | `T_MIP_ROOT_OBJ_PROBE_TIME_FRAC` 0.2×TL | 既定 | 壁時計 | 量 (普段は安全網寄り) | 試した列の数か伝播で触った非零の数 | 低〜中 |
| B11 | 16464-16481 | HiGHS 式の衝突解析を休むか (使った秒 > 経過×0.1 + 0.05) | `T_MIP_CONFLICT_TIME_FRAC` 0.1 | 既定 | 壁時計 | 分かれ道 | 段数・説明に使った行の非零の累計とノード LP の反復の比 | 低〜中 |
| B12 | 19857-19960 (`lmip_boost`)、判定 19866-19871、止め 19927-19950 | LMBOOST (= A12) | `T_MIP_LMBOOST_*` | cand1114 | 壁時計 | 分かれ道・量 | 根のカットのラウンド数・反復数、Local-MIP の歩数 | 中〜高 (当たる問題は少ない、comp07 型) |
| B13 | 19596-19613 (`abort_submips_after_grace`)、呼ぶのは 9587 | 早めの再スタートの後、並列サブ MIP を 0.5 s 待って止める (= A2) | `T_MIP_LOSE6_1_GRACE` 0.5 | cand1114 (ERWAIT `s`) | 壁時計 | 分かれ道 | すぐ止める (解は受け取る) か、ノード数で区切る | 中 |
| B14 | 19458-19581 (`join_submips`: SUBCAP J/B・TREE3_JOINCAP・LOSE6_3) | 根で並列サブ MIP を待つ上限 | | 試作のみ | | 分かれ道 | | — |
| B15 | 19388-19393 (`SubmipCtl::with_nosol`/`with_subcap`) | 並列サブ MIP の解のないうちの期限 `nosol_deadline` | 呼び出し側 | 呼び出し側しだい (SUBCAP は試作) | 壁時計 | 量 | ノード数上限 | — |
| B16 | 11738-11751・11839-11848・11853-11986・12161-12168・12918-12985・12995-13062・13342-13450・14190-14195・14818-14846・16788-17046・16512-16521・18900-18911・18963-18975・19774-19810・21124-21240・11334-11345/11531-11538 | ROBUST_LNSFREQ・OBJ_BISECT・NOINC_HEUR・FC_NODE_SEP2・ROBUST_TSTALL・PROOF_SWITCH・ROLE_LMIP・PRECHAIN・ROOT_LMIP・SUBMIP_SB_TIME_FRAC・SUBMIP_NOSOL_STOP・OBJPROBE・CUTCONF・OVERRUN3・RESTART_FIRST_LPW・POOL2_TAKE・POOL3_BEST・NODELP2 x | | 試作のみ | | | | — |
| B17 | 21033-21066 (`race_step`) | racing ramp-up の勝者を秒で決める | 5 s | 実際は無効 (racing を true にする所がない) | | | | 無し |
| B18 | 19337-19355 | 並列サブ MIP の上限を残り時間で頭打ち | | 既定 | | 安全網 | | 低 |
| B19 | 19161-19166 | 根の再スタート後の求解の `time_limit` = その時点の残り秒 | | 既定 | 壁時計 | 安全網。ただし量に伝わる | 割合の基準の元の TL を別の欄で渡す | 中 (再スタート後は「× time_limit」の量がすべて機械で変わり、`self.start` も作り直されて経過秒の割合も振り出しに戻る) |
| B20 | 20250 | 従のスレッドの上限 = 残り + 2 s | | 既定 | | 安全網 | | 低 |
| B21 | 21940-21946 | 根の緩和が非有界のとき切片の問題に 60 s まで | `T_MIP_UNB_FEAS_SECS` 60 | 既定 | | 安全網寄り | | 低 |
| B22 | 14893-14900 (`resume_pause`) | RENS の継続の期限を `ResumeCmd.secs` で付け直す | 呼び出し側 | 既定 | | 安全網 | | 低〜中 |

補足 (時間を測らないが壁時計の速さで結果が変わる所): `join_submips` (既定は先に終わったものから受け取る、`is_finished()` を 1 ms ごと)、木の中の `poll_submips` (19615〜)、FJ/Local-MIP のスレッドは `join_fj_thread` (19962) で主のカットのループの終わりに止まる。

診断: B1 は今は見えない (`lns_skip_budget`・cap で下げた回数・TimeLimit の回数を 22267 の node diag に足す)。B7 は TRSTAT 行 (21629-21640) で `root`・`tree`・`est_r`・`guard stop R/pass` が見える。B5/B6 は `ENOMOTO_MIP_HEUR_ROLE_VERBOSE`・`ENOMOTO_MIP_HEUR_TRIG_DEBUG`。B2 は 22269 の `adaptive node cuts` に予算で断った数を足す。B9 は "GAP1 u: LP probing … ({:.2}s)" (16761) に timed_out を足す。
