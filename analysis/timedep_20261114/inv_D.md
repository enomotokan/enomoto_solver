## 洗い出し D: src/mip/ の solver.rs・separation.rs・mod.rs 以外

cand1114 のうち担当範囲に時間の判定があるのは SYMPRE=probe と FOLD=rinq だけ (ROOTSTOP・RCSTRONG・PLSB・LMIP_SWAP は時間を表示に使うだけ)。

| # | 場所 | 何を決めているか | 既定値・環境変数 | 有効 | 測り方 | 種別 | 手間に置き換える案 | 響き |
|---|---|---|---|---|---|---|---|---|
| D1 | heuristics.rs:3607-3678 `fj_lmip_mix` (3610 t_end、3612 slice、3623-3624 ext、3628-3629、3636 use_lm、3641、3642/3649 t_lm/t_fj、3646) | FJ のスレッドの中で FJ2 (種を変えて何回) と Local-MIP (何区切り) に時間を分ける。使った秒の割合 (先導 0.75) で次を選び、全体を cap か ext で終える | `ENOMOTO_T_MIP_FJ_MIX_SLICE`=0.2 秒、`_FJ_MIX_LEAD`=0.75、cap = max(min(0.1×TL,10), min(0.05×TL,5)) (solver.rs:8065)、ext は `FJ_NOSOL_EXTEND`=2 で 0.25×TL−経過 (solver.rs:8076-8077) | 既定 (`NO_FJ_LMIP_MIX` で止まる)。並列で目的が 0 だけでない問題 | 壁時計 | 分かれ道・量 | Local-MIP は `res.steps`、FJ は `fj2_search` に `s.effort` を返させ、比べる量を手間の和に。cap/ext は nnz に比例する手間の予算に | 高 (並列のほぼ全部の問題。どの種・区切りで解が出るか、受け渡しの時刻、根の暫定解・RENS/RINS の出発点・打ち切り値が変わる。主のカットのループの終わりでも止まるので、その時刻の違いは別に残る) |
| D2 | localmip.rs:666 (16 手ごとに `Instant::now() >= t_end`)、984、1118 | Local-MIP が何手回るか (手数の上限なし) | 呼び出し元 (D1 の 0.2 秒、FJ_LOCALMIP、solver.rs:13363/13402/13444/19905 の木の役・LMBOOST) | D1 を通して既定 | 壁時計 | 分かれ道・量 (D1 の土台) | `run` に手数の上限 (`step`) を渡す | 高 (D1 と同じ。LMBOOST などにも効く) |
| D3 | heuristics.rs:1498-1538 `submip_time_limit` (1512 t_plain = frac×remaining)、使う所 1622・1833 (RENS の継続) | 木の中のサブ MIP (ALNS の RINS・LB・proximity・mutation・crossover・DINS、RRC) と入れ子の RENS の時間の上限。サブ MIP の中の TL 比例の量も全部動く | `ENOMOTO_T_MIP_SUBMIP_TIME_FRAC`=0.07、`_MAX`=6 秒 (solver.rs:4733)、RRC 0.07 (3482)、入れ子 RENS `SUBMIP_RENS_OLD_FRAC` 0.3 (2125)、暫定解なしの入れ子 0.15・3 秒 (2182-2183)。外側の根の RENS は `SUBMIP_HIGHS` t で remaining×1 (安全網) | 既定 | 壁時計 | 分かれ道・量 | ノード上限 (既定 500) に加えて LP 反復の上限 (外側の根の LP 反復・`heur_iters` の倍数) | 高 (コメントに physiciansched で 0.6〜2.5 秒・0 ノードで時間切れ、n2seq36q で 5 秒で止まり続けた記録。ALNS の報酬・`lns_grow`・RENS の継続も変わる) |
| D4 | heur_ipm.rs:27-36 (budget = min(0.05×TL, 0.5×残り))、50、57/82、187 | 内点法の丸めで、内点法が予算内に終わるか (終わらなければ丸めない) と何点試すか | `ENOMOTO_T_MIP_IPM_TIME_FRAC`=0.05 | 既定 (外側の根だけ) | 壁時計 | 分かれ道・量 | 内点法の反復の上限、または KKT の分解の非零からの演算量 | 中〜高 (内点法が 3 秒前後かかる大きな問題。中心からの解が根の暫定解になる) |
| D5 | heuristics.rs:1004-1080 `local_search` (1013 t_end、1079)、heur_scip.rs:447-453 `ls_time_cap`・456-480 | 根の暫定解がないときの局所探索の長さ | `ENOMOTO_T_MIP_LS_TIME_FRAC`=0.05×TL (5 秒まで)、手数 200×nnz を [2e5, 1e8] | 既定 (noinc_like のとき。solver.rs:9620 と解発見スレッド 9215) | 壁時計 | 分かれ道・量 | 既にある手数の上限 `steps_total`/`max_steps` だけにする | 中 (暫定解がない問題だけ) |
| D6 | heur_scip.rs:550-555 | Repair の補助 MIP の時間上限 min(0.07×残り, 6 秒)、ノード 500 | 固定 | 既定 (noinc_like) | 壁時計 | 分かれ道・量 | ノード上限に任せ LP 反復の上限を足す | 中 |
| D7 | heuristics.rs:1247-1312 `fj_search_fb` (1257-1261・1293-1296・1300-1306) | FJ2 を種を変えて何回回すか、旧来の局所探索の分 | 実行可能性問題 cap = `ENOMOTO_T_MIP_FJ_FEAS_FRAC` 0.5×TL (solver.rs:8004) | 既定で当たるのは目的が全部 0 の問題だけ | 壁時計 | 分かれ道・量 | 種の回数の上限か effort の和 | 中 (実行可能性問題では解が出たかがそのまま結果) |
| D8 | hnf.rs:502-503、330 | HNF の前処理で成分ごとに諦めるか | `ENOMOTO_T_MIP_HNFPRE_SECS`=1 秒 (手間の上限 `HNFPRE_WORK`=3e8 もある) | 既定 | 壁時計・絶対秒 | 安全網兼分かれ道 | 手間 `work` だけに | 低〜中 |
| D9 | symmetry.rs:597・605-608・617/720/806/867、sympre.rs:412/421・223 | 対称性の検出の打ち切り | time_cap = `SYM_TIME` 0.3 × `PRESOLVE_WORK_SAFE` 10 = 3 秒 (再スタート後 50 秒)、手間の上限 2.5e7・1.3e8 (mod.rs:2545-2549)、探りは残りの 0.5 倍 | 既定 (`probe` は cand1114) | 壁時計 | 安全網 (普段は手間が先) | そのまま | 低 |
| D10 | cmirdelta.rs:172-176・271-275 | δ の候補を足すのをやめる | `ENOMOTO_T_CMIRDELTA_SECS`=20 秒 (全スレッドの合計) | 既定 | 壁時計の累計 | 実際は安全網 | `CALLS` の累計 | 低 |
| D11 | fold.rs:593-594・652〜821 | 畳んだ問題・前処理した問題に渡す残り時間 | TL − 畳みの秒 | cand1114 (FOLD=rinq) | 壁時計 | 安全網 (内側の TL 比例の量がわずかに動く) | 内側には元の TL を渡し期限だけ共有 | 低 |
| D12 | fj.rs:334・464・474 | FJ2 の 1 回を打ち切る時刻 | 呼び出し元の left | 既定 | 壁時計 | 普段は手間が先 (D1・D7 の最後の 1 回だけ時刻) | D1・D7 を手間にすれば不要 | 低 |
| D13 | zerohalf.rs:136-138・275・352、lp.rs:102、heuristics.rs:971・3183、fpr_paper.rs:466、heur_scip.rs:345-350 | 期限・止めの合図 | 全体の期限 | | 壁時計 | 安全網 | | 低 |
| D14 | heur_lns2.rs (LB2・PROX2)、treefix.rs:123-124・177-178、prechain.rs:124-130・357-404、heuristics.rs:2078-2079・1515・1528-1529・1534・2742-2764・3130・3151・2732・1260-1287、fpr.rs、hnflat.rs、modkvb.rs、snfmodk.rs、redsplit.rs、seplearn.rs:411、cutconf.rs:78-90 | 各試作の打ち切り・配分 | | 試作のみ | 壁時計 | | | — |

表示・統計だけ: 約 90 か所・約 30 ファイル (heuristics.rs のサブ MIP の秒、FOLD_PROF、各ログ、pretime.rs 全体 (`ENOMOTO_MIP_DEBUG_PRETIME`)、lp_api.rs は期限の受け渡しだけ)。

診断: D1 は `ENOMOTO_MIP_FJ_DEBUG` で 3664 行 (FJ の回数・秒、Local-MIP の区切り数・秒、先導、最初の解の時刻と側)。D3 は 1762 行の `sub-MIP: ... status` (主の分だけ。入れ子は `ENOMOTO_MIP_SUBMIP_VERBOSE`)。D4 は 70 行・167 行 (期限で止まったかは出ない)。D5 は止まった理由が出ない。
