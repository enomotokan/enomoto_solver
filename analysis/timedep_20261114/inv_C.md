## 洗い出し C: src/mip/separation.rs・src/mip/mod.rs

- `cap_clock()` (mod.rs:267-289) はこのスレッドの CPU 時間 (`ENOMOTO_MIP_WALL_CAPS` で壁時計)。
- 既定で有効 (env_def!): `MPSEP=oqmgs` (7603)、`CUTLPCAP=ac` (1819)、`CUT_LP_CAP_ROBUST=p`、`CUT_LP_CAP_STOP=flat`、`CUT_CAP_FROM_LOOP=1`、`CUTSEL_PROBE=rate`、`PRESOLVE_WORK=1`、`FEAS_FJ_EARLY=fl`、`GR_AUTO`。
- cand1114 でこの範囲の時間の判断を新たに有効にするものは無い (`GAP4=rocn` に `p` は無い、`TREEEFF=s` はラウンド数)。

| # | 場所 | 何を決めているか | 既定値・環境変数 | 有効 | 測り方 | 種別 | 手間に置き換える案 | 響き |
|---|---|---|---|---|---|---|---|---|
| C1 | separation.rs:1438-1440 (`cut_frac`/`time_cap`)・1518・1526-1527・1549 (`cap_base`)・2258-2259・2296 (判定)、比べの分の差し引き 2207・2216 | 外側の根のカットのループを打ち切るか: `now − cap_base > time_cap && round ≥ min_rounds` | `ENOMOTO_T_MIP_CUT_TIME_FRAC`=0.1、`CUT_CAP_FROM_LOOP`=1、`T_MIP_CUT_MIN_ROUNDS`=0 | 既定 | CPU 時間。TL×0.1 | 分かれ道 | 試作 `ENOMOTO_T_MIP_CUT_WORK_CAP` (1745・1768-1775・2299-2307) がそのまま使える (手間 = `loop_iters/root_iters + 0.05×work_rounds`、時間は 2 倍の安全網。既定 0) | 高 (1 ラウンドが重い問題: neos-960392・physiciansched6-2 など。根の頂点・カット・RENS の出発点・木が全部変わる) |
| C2 | separation.rs:1487・1904-1928・3720-3730・2296 | 第 2 段階 (HiGHS のスコア) の打ち切り: t1 + max(3·t1, 0.2) | `T_MIP_CUT_PHASE2_TIME_MULT`=3、`CUT_PHASE2_ROUNDS`=60 | 既定 (強いカットの問題だけ。裏の determ では外す) | CPU 時間 (第 1 段階との比) | 分かれ道 | 第 2 段階の LP の反復 ≤ 3×第 1 段階の反復 | 中 (timtab1 型・hcm_auto 後) |
| C3 | separation.rs:2961-2963・2998-3003・3059・3335-3346・3394・3410-3416、GR_AUTO の解き直し 3252-3261 | 根のカットの LP 1 回が時間で止まったらそのラウンドのカットを外す (ループを止めることも) | `T_MIP_CUT_LP_TIME_FRAC`=0.02、`CUT_LP_WORK_TIME_MULT`=5 (本来の上限は反復 `CUT_LP_WORK`=0.1 ほか) | 既定 | 壁時計 | 主に安全網 (重い LP では当たる) | 時間は残す。GR_AUTO の解き直しは反復の倍数に | 低〜中 |
| C4 | separation.rs:1438-1440 (`SUBMIP_CUT_AGG_TIME_FRAC`=0.5)・1549・1777・2961 | サブ MIP の根のカットのループの打ち切りと LP 1 回の上限 | `SUBMIP_HIGHS` 既定 `-s` に `c` | 既定 | CPU 時間 / 壁時計 (サブ MIP の上限との比) | 上限が短いサブ MIP では分かれ道 | サブ MIP でも `CUT_LP_WORK`・`CUT_WORK_CAP` を使う | 低〜中 (ここの `time_up()` (solver.rs:6034-6040) は `heur_stop`・`nosol_deadline`・`subcap_heavy` を含む) |
| C5 | separation.rs:2182-2187 (`rate`)、時間の元 1159・1223 | CUTSEL_PROBE で既定の選び方と POOL のどちらを選ぶか (時間あたりの伸び) | `CUTSEL_PROBE`=rate (試作 `TIE_FEWER` は 2172-2175、別に計測中) | 既定 (外側の最初の根) | 壁時計 (分離 + LP) | 分かれ道 (既知: parint2) | 各側の LP の反復 (`total_iterations()` の差、`keep_iters` 1169 の隣) + 足したカットの非零 (`PROBE_NADD_HIST`) | 高 (毎回通る。同点でなくても伸びの比が近いと入れ替わる) |
| C6 | separation.rs:2099-2108 (`decisive`: 比 > 3)・2121-2122・2143-2145 | 比べを 1 ラウンドで決めるか 6 ラウンド回すか、POOL 側をどこで止めるか | `T_MIP_CUTSEL_PROBE_RATIO`=3、`_ROUNDS`=6 | 既定 | 壁時計 | 分かれ道 | C5 と同じ | 中〜高 |
| C7 | separation.rs:2093-2097・2123・1181 | 比べの合計の予算 | `T_MIP_CUTSEL_PROBE_TIME_FRAC`=0.05 | 既定 | 壁時計 | 分かれ道 | c×root_iters の LP 反復 | 中 (puriri 型) |
| C8 | separation.rs:1170-1171・1216-1218・1219-1221 | 比べの中のカットの LP 1 回の上限 (止まるとその側の比べが終わる) | `cut_lp_time_frac`=0.02 (最低 1 秒)、`CUT_LP_WORK` がかからない | 既定 | 壁時計 | 分かれ道 | 本体のループと同じ反復の上限、時間は 5 倍の安全網 | 中 (physiciansched6-2 型) |
| C9 | separation.rs:5302 (`MPSEP_MOVE_SECS = clamp(0.25×root_lp_secs, 0.01, 0.1)`)、期限 solver.rs:5663-5669、止まる所 simplex/mip_lp.rs:3220 | 多点分離 (MPSEP `o`) で最適面の上をどこまで移るか | `MPSEP`=oqmgs、`T_MIP_MPSEP_PIVOTS`=1000 | 既定 | 壁時計 (根の LP の秒×0.25) | 分かれ道 | 入れ替えの回数 (`st.pivots+st.flips` ≤ 1000) と ftran の数 | 中 (最適面が広い問題の停滞ラウンドで毎回) |
| C10 | separation.rs:5516-5536・5541-5548 | 面の頂点での分離 (int2) の面の LP の上限 | `cut_lp_time_frac`=0.02 (最低 1 秒) | 既定 (列の多い問題) | 壁時計 | 分かれ道 (まれ) | 反復の上限だけに | 低 |
| C11 | separation.rs の `time_up()`・`sep_deadline`・ZH_DEADLINE・`self.deadline` を渡す LP | 全体の期限 | | 既定 | | 安全網 | | |
| C12 | mod.rs:1141-1145 (`secs = min(30, 0.5T)`)・4447/4460・4502・4551/4581 | LP による固定 (前処理) の打ち切り | `PRESOLVE_WORK`=1 (反復が主)、`T_MIP_PRESOLVE_WORK_LP_SECS`=30 | 既定 | 壁時計 | 主に安全網 (cod105 で 30 秒) | `budget_left` から時間を外す | 低〜中 |
| C13 | mod.rs:1150-1155・2041-2170・4090-4100 | 実行可能性問題で前処理の途中に早めの FJ と Local-MIP (1 スレッドでは 0.2 秒ずつ交互) | `FEAS_FJ_EARLY`=fl、`_FRAC`=0.1、`T_MIP_FEAS_EARLY_LMIP_SLICE`=0.2 | 既定 | 壁時計 | 分かれ道 | FJ の手数・Local-MIP の steps | 中 (実行可能性問題だけ) |
| C14 | mod.rs:5710-5739・5838・6186-6233・2534-2552 | 前処理の各手順 (プロービング・equiv2・対称性) の上限 | `PRESOLVE_WORK_SAFE`=10 (手間の上限が主) | 既定 | CPU/壁時計 | 安全網 (置き換え済み) | | 低 |
| C15 | mod.rs:399-401・507・511-513・1137・3017・3022・1139・1714・2272-2278 | 全体の期限と残り時間 | | 既定 | | 安全網 | | |
| CP | separation.rs・mod.rs の試作 (CUT_EXTEND・CUTRND・GAP2_ZEROCAP・GAP4 p・LOSE6_2_TIE・CUT_LP_CAP_MULT ほか・BG_CUTS_LP_CAP・SEP_COUNT_CPU・FACE_ADAPT・MPSEP h/c/a・ZH_TRIAL・CLQFB_CAP・CLIQUE_CUTS=probe・ROOTCAP・SNFMODK/MODKVB/REDSPLIT・OPTFACE_PARCUTS・SEPLEARN=t・TIGHTEN_VUBS ほか) | | | 試作のみ | | | | — |

表示・統計だけ: separation.rs 約 70 か所 (sepprof・ログ・合計の記録)、mod.rs 約 40 か所。

診断: C1 は `ENOMOTO_MIP_LOG=1` で毎ラウンド `root cut time cap check: clock X of cap Y [-> stop]` (2288-2294。外側の主の根だけ)。C5 は `cut selection probe (k rounds): ... -> X` と `ENOMOTO_MIP_CUTSEL_PROBE_TIE_LOG`。C6-C8 は `compared at m rounds (early)` の `early`。C9 は `MPSEP=oqmgsv` で `optface mpsep: ..., stopped by time`。C2 は `cut loop phase 2` で入ったことだけ。
