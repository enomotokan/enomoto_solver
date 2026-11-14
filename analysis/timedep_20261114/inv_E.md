## 洗い出し E: LP・前処理 (src/simplex*、src/interior_point*、src/presolve*、src/solver.rs ほか)

- 二段解法の本体の LU の作り直しは時間で決めていない (`basis_kernel.rs:510-540` は合成クロック `lu.synth_tick()`/`lu.build_tick()`)。時間で決めるのはクロスオーバーの Megiddo 式の主の押し出しの中だけ (E4)。
- 前処理 (`presolve.rs`, `presolve/`)、`graph.rs`、`model.rs`、`types.rs`、`solver.rs` に時間で決める所は無い。KKT の並べ方 (AMD/METIS) は演算量の見積もり (`kkt.rs:150-191`)。

| # | 場所 | 何を決めているか | 既定値・環境変数 | 有効 | 測り方 | 種別 | 手間に置き換える案 | 響き |
|---|---|---|---|---|---|---|---|---|
| E1 | simplex/mip_lp.rs:1550-1579 (`solve` の振り分け)・2053-2182 (`solve_race_ext`)、勝ちは 2090 (内点法側の `cancel.swap`)・2128-2129 (単体法側) | 基底のない LP を二段解法 1 本と内点法+クロスオーバー threads−1 本で同時に解き、先に終わった方の頂点・基底・双対を採る | `COLD_RACE_THREADS` (mip/mod.rs:462-463) ≥ 2。試作 `ENOMOTO_MIP_MC11FIX1_RACE=s/i` (1560-1572) と診断 `ENOMOTO_MIP_ROOT_LP_IPM` (2112) は勝ち手を固定 | 既定 (主スレッドの基底のない求解。サブ MIP・従は 1 本: mip/solver.rs:3901・20203) | 壁時計 (先着) | 分かれ道 | 仮想の時刻で勝ちを決める (単体法は反復、内点法は反復×`chol_flops`、押し出しは pivots)。手軽には「単体法が c·(m+n) 反復以内に最適なら単体法、超えたら内点法を待つ」(`MC11FIX1_RACE=i` と同じ形) | 高 (根の頂点→カット・下界・木の道。mc11・beasleyC3 で記録あり。`root_lp_secs` を通して MIP 側の予算にも波及) |
| E2 | simplex/crossover.rs:905-907、interior_point/boxed.rs:810-828、slope_intercept_dual.rs:5566-5569 | 同時実行中、内点法がどの反復でクロスオーバーに渡すかを、並走する二段解法がそれまでに書いた下界で決める | `ENOMOTO_T_XO_BOUND_GAP=1e-6`、`ENOMOTO_T_RACE_BOUND_EVERY=100` | 既定 (E1 の中) | スレッド間の進み具合 | 分かれ道 | 共有の下界を使わない (`bound_gap=0`)、または内点法の反復に応じた単体法の反復の時点の下界だけ使う | 中〜高 |
| E3 | simplex/mip_lp.rs:1612-1630 (`solve_once` の終わり)、633-637、188・1619 (`LPSCALE_ARMED`、全スレッド共有の static) | 尺度なしの 1 回の求解が 1 秒以上なら尺度つきの双子を作り、以後その LP と以後に作るすべての LP (サブ MIP・再スタート・写し) が双子で解く | `ENOMOTO_T_MIP_LPSCALE_HEAVY_SECS=1.0`、`ENOMOTO_MIP_LPSCALE` 既定 `h`、係数の幅 ≥ `LPSCALE_MIN_RANGE=1e4` | 既定 | 壁時計 | 分かれ道 | 反復/行数 (`it >= c·m`) か合成クロックの tick の累計 | 高 (1 秒前後の LP を持つ問題で以後すべての LP のピボットの道が変わる。static なので裏のスレッドの求解でも切り替わる) |
| E4 | simplex/crossover.rs:3473-3476・3644-3645・3684-3686・3737-3738 (`megiddo_push`) | Megiddo 式の主の押し出しの LU の作り直しの時期 (FT 300〜1200 の間は求解の時間の累計 ≥ 前回の分解の時間で作り直す) | `ENOMOTO_T_XO_MEGIDDO_FT_ADAPT=4.0`、`_FT_MAX=300` | 既定 (根の同時実行の内点法側、内点法の丸めの頂点) | 壁時計 | 分かれ道 | 本体と同じ合成クロック (`synth_clock_should_refactor_density`、basis_kernel.rs:950) | 中 |
| E5 | simplex/crossover.rs:2541-2554・1093・3891・4089 | クロスオーバーの双対の押し出しを締切 (1.0×内点法の時間 + 1 秒) で打ち切り、残りを仕上げに回す | `ENOMOTO_T_XO_MEGIDDO_TIME_FACTOR=1.0`、`XO_DUAL_MEGIDDO=1` | 既定 (同時実行の中でも上限あり) | 壁時計 | 量 | 内点法の手間 (反復×`chol_flops` か nnz(L)) から予算、押し出しの手間 (`nchecks`・`pivots`) で数える | 中 |
| E6 | simplex/mip_lp.rs:1241-1262 (見張りのスレッドが期限でトークンを立てる)、予算は mip/heur_ipm.rs:27-30・mip/separation.rs:5329-5333 | 内点法の丸めの内点法 (+頂点) を期限で打ち切り `None` に (= D4) | `ENOMOTO_T_MIP_IPM_TIME_FRAC=0.05` | 既定 | 壁時計 (5 ms ごと) | 分かれ道・量 | `IPM_MAX_ITERS=200` に分解の演算量の累計の予算を足す | 中〜高 |
| E7 | simplex/crossover.rs:2208-2216・3558・2246-2253 | 主の押し出しの締切 (2×内点法の時間 + 1 秒) | `XO_MEGIDDO_TIME_FACTOR=2.0`、同時実行中は `_RACE=0` で上限なし | MIP では E6 の頂点だけ | 壁時計 | 量 | E5 と同じ | 低〜中 |
| E8 | simplex/lp_fold.rs:374-379・535 (`FoldOpts::default` の `max_secs: 30.0`) | 色の細分化 (畳み) が 30 秒を超えたら畳まない | 環境変数なし | 既定 (mip/fold.rs:91・106・358) | 壁時計 | 分かれ道 | 隣の `work` に上限 `c·(nnz+n+m)` | 普段は低、起きれば高 |
| E9〜E15 | mip_lp.rs:251-309・2272-2390 (ROOTLP_BOTH)、mip_lp/race3.rs (ROOTLP3)、mip_lp.rs:3173-3223 (OPTFACE)、mip_lp/pdlrc.rs (PDLRC)、boxed.rs (GONDZIO_AUTO)、crossover.rs:1741-1758 (PROJ_PUSH)・2304-2334 (XO_MIP_REPAIR / LP 単体) | | | 試作のみか LP 単体のみ | | | | 低 |
| E16 | simplex/race.rs・simplex.rs:1994-2003 | LP の Auto の先着 | `auto_race` | LP 単体のみ | | | | MIP には響かない |
| E17 | slope_intercept_dual.rs:1610-1648・4955-4970 | 厳密な DSE 重みの打ち切り | `BG_CUTS_DSE=rw/q` | 試作のみ | | | | 低 |
| ES | cancel.rs:44-70、slope_intercept_dual.rs:1213-1218 (32 反復ごとに期限)、mip_lp.rs:2700-2706・2160-2167・2404-2408、simplex.rs:1892-1918 | 全体の期限・取り消し | | 既定 | | 安全網 | | 低 (MIP 側が `SolveLimits.deadline` を LP 1 回の締切に使う所は分かれ道) |

表示・統計だけ: 約 500 行 (lu.rs の prof、crossover.rs の prof・ログ、slope_intercept_dual.rs の XPROF・LUCLOCK、mip_lp.rs の lpscale/xolazy/RACE_DIAG、interior_point の prof、presolve の PROF_PRESOLVE など)。
参考 (時間ではないが機械に依る所): 同時実行の内点法側のプールが threads−1 本、WOODBURY の束の大きさ (kkt.rs:1588)、外積の並列の束 (kkt.rs:1224) はスレッド数に依り、並列の和の順が変わる。

診断: E1 は `RACE_DIAG`/`take_race_diag` に勝った側と反復を足し、終わりに「同時実行 N 回: 単体法の勝ち a / 内点法の勝ち b」。E3 は `LPSCALE_STATS[14]` (切り替えの数) が既にあり `lpscale_report()` に閾値の近くの数と反復/行数を足す。E2 は `RACE_DIAG` の `gap_shared` の横。E5/E4 は `phase_timing` の `xo_dual_megiddo_skipped`。
