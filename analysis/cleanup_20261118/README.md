# 不採用の試作の片付け 第 1 弾 (2026-11-18)

依頼は `BRIEF.md`。ブランチ ccr-1beed81b-ex0uhq。1 件ごとに fetch+merge → 削除 → 確かめ → コミット → push。

## 確かめの方法

- `cargo build --release` の警告の行の数 (消す前 77)・`cargo test --release --lib` (消す前 422 通過・0 失敗・19 無視)。
  どちらも fuzz.sh と同じ設定 (`CARGO_TARGET_DIR=target/test-py`・python 3.13 を結ぶ `RUSTFLAGS`)。
- 既定の振る舞い: `ident.py` (1 スレッド・既定の種・60 秒、ENOMOTO_ の環境変数をすべて外す) で 12 問の (状態・目的値・ノード数) が
  消す前のビルド (`base.json`、ecb1ded51) と完全に一致すること。依頼書の例の pk1・neos5・10teams・gen-ip002 は 1 スレッドでは
  60 秒で終わらずノード数が時間で揺れるので、60 秒以内に解ける misc07・fiber・qnet1・dcmulti・blend2・exp-1-500-5-5・neos-860300・
  khb05250 に替えた。消す前のビルドで 2 回流して完全に一致することを確かめた。
- 4 件ごとと最後に、analysis/release019_20261117/fuzz.sh と同じ fuzz 7 本で bad 0。

## 1 件ごとの結果

| # | 試作 | コミット | 減った行 (src) | 警告 | cargo test | ident | 備考 |
|---|---|---|---|---|---|---|---|
| 1 | `PROBE_EQUIV2_RESTART` | — | — | — | — | — | **飛ばした**: 根拠「誤った最適値が出る」は記録と合わない。analysis/eqrst_20261113.md で誤答の原因 (係数の強化) は既定で直っていると確かめられ、`=1` は「正しさの手当てが要らなくなった制限を外す」束の候補として残されている (58 問の計測待ち)。分岐 1 行で、消すより束で決める方がよい |
| 2 | `QIU_MARGIN_DEPTH` | (このコミット) | 1 | 77 | 422/0/19 | 一致 | 根拠: analysis/qiu_margin_20261104.md (uccase12 で HiGHS の最適値より下の値・列 166 本の下限を破る)。調査用の `QIU_MARGIN_SITES` は依頼の外なので残した |
| 3 | `FPR` 一式 (`T_MIP_FPR_*`・`MIP_DEBUG_FPR`) | (このコミット) | 1526 (fpr.rs・fpr_paper.rs を削除、solver.rs の呼び出し・腕 9 番) | 76 | 420/0/19 (fpr の単体テスト 2 本が消えた) | 一致 | 根拠: analysis/fpr_20261110.md (58 問 SGM +1.6%、解き終わる時間は変わらない)。暫定解なしの腕は 10 → 9 本 (FPR の腕は既定では選ばれない最後の番号) |
| 4 | `MODKVB` 一式 (`_CAPDROP`・`_LOG`・`T_MIP_MODKVB_*` …) | (このコミット) | 1198 (modkvb.rs を削除、separation.rs の分離・CAPDROP、hnfcut.rs の写し `map_cutvars`・`MappedVars`・`with_cmir` とその試験の (3)) | 75 | 417/0/19 (modkvb の単体テスト 3 本が消えた) | 一致 | 根拠: analysis/modkvb_20261107.md (58 問 +3.7%・解けた回 84 → 79)、作り直しも見送り (analysis/gate2_20261117/modkvb_s/README.md)。**共有の部品の扱い**: snfmodk.rs (`snf_group` など) は試作 SNFMODK が使うので残し、注記だけ直した (文字 `v`・`k` は今は何もしない)。`SRC_MODK` は MUSHAGG が使うので残した。hnfcut の写しは MODKVB だけが使っていたので消した |

### fuzz (4 件目の後、094a43cb9)

`FUZZ_TAG=after4 bash analysis/cleanup_20261118/fuzz.sh` (release019 と同じ 7 本、既定): fuzz_tiny 400 0 bad 0 (strict 13)・
mip_fuzz_vs_highs 300 bad 0・fuzz_objint 1000 0 bad 0 (optimal 967・infeasible 33)・fuzz_slack 1000 0 mismatches 0・
fuzz_tiny 400 1000 bad 0 (strict 14)・fuzz_objint 1000 5000 bad 0 (optimal 971・infeasible 29)・fuzz_slack 1000 5000 mismatches 0。
cargo test 417 通過・0 失敗。7 本とも rc=0、数は release019 の記録と同じ。

| 5 | `SEPLEARN` (学習の部分) | (このコミット) | 133 (seplearn.rs の飛ばす・上限の判定、`LEARN_SRCS`、separation.rs の `off` の判定と候補の間引き、調整値 `T_MIP_SEPLEARN_WARM`・`_ROOT_SKIP`・`_SKIP`・`_WORKF`・`_MAXSKIP`・`_CAPMULT`・`_CAPMIN`・`_NOCAP`・`_NODE_WARM`) | 75 | 417/0/19 | 一致 | 根拠: analysis/seplearn_20261108.md (58 問 +4.5%・解けた数 85 → 81)。**共有の部品の扱い**: 同じ構造体の計測 `ENOMOTO_MIP_SEPSTAT` は「計測はできた」と記録にあり、docs/backlog.md の案が使う予定なので残した (表の得点・根から木への引き継ぎも計測の一部として残した)。計測の調整値 `T_MIP_SEPLEARN_ALPHA`・`_SURV_K`・`_LPW` と表の頭 `SEPLEARN` は記録の台本と合わせるため名前を変えていない。消した後に misc07 を `SEPSTAT=1` で流し、表が出ることを確かめた |
| 6 | `REDSPLIT` 一式 (`_LOG`・`T_MIP_REDSPLIT_*`) | (このコミット) | 523 (redsplit.rs を削除、separation.rs の k-cuts・縮約・採られた数の診断、cuts.rs の `cmir_with_deltas`) | 75 | 415/0/19 (redsplit の単体テスト 2 本が消えた) | 一致 | 根拠: analysis/redsplit_20261107.md (揺れの大きい 12 問を除くと +5.2%、qnet1 が 2 回とも遅い。k-cuts は効かない) |
| 7 | `LB2`・`PROX2`・`HSUB_STALL` 一式 (`T_MIP_HSUB_*`・`T_MIP_LB2_*`・`T_MIP_PROX2_*`) | (このコミット) | 300 (heur_lns2.rs を削除、solver.rs の根と木の呼び出し・状態、heuristics.rs のサブ MIP の一時的なギャップ `submip_gap`) | 75 | 415/0/19 | 一致 | 根拠: docs/improvement_history.md (「既定にしない: … `LB2`・`PROX2`・`HSUB_STALL`」)、analysis/heur_shift_lns2_20261026.md (56 問 2 回で揃った差なし、15〜45 秒で解ける問題で解き切りが遅れる)。`submip_gap` はこの試作だけが立てていた |
| 8 | `CUTSEL_ENS` 一式 | (このコミット) | 239 (cutsel_ens.rs を削除、separation.rs の 2 か所の選択と `select_cuts_ens`) | 75 | 413/0/19 (cutsel_ens の単体テスト 2 本が消えた) | 一致 | 根拠: analysis/cutsel_ens_20261110.md (SCIP の重みそのままで 58 問 +9.2%・区間 [+2.4%, +17.2%]、本数をそろえた形は中立) |

### fuzz (8 件目の後、4f060298b)

`FUZZ_TAG=after8`: fuzz_tiny 400 0 bad 0 (strict 13)・mip_fuzz_vs_highs 300 bad 0・fuzz_objint 1000 0 bad 0 (optimal 967・infeasible 33)・
fuzz_slack 1000 0 mismatches 0・fuzz_tiny 400 1000 bad 0 (strict 14)・fuzz_objint 1000 5000 bad 0 (optimal 971・infeasible 29)・
fuzz_slack 1000 5000 mismatches 0。cargo test 413 通過・0 失敗。7 本とも rc=0。

| 9 | `GCDFORCE` 一式 (診断 `MIP_DEBUG_GCDCOUNT` を含む) | (このコミット) | 265 (gcdforce.rs を削除、mod.rs の前処理の判定・かけ直しの印・数える診断 2 か所) | 75 | 412/0/19 (gcdforce の単体テスト 1 本が消えた) | 一致 | 根拠: analysis/gcdforce_20261107.md (203 問で当たる行が 0 本、理屈でも起きない)。数える診断 `DEBUG_GCDCOUNT` は同じ調べの道具で `gcdforce::count` を使うので一緒に消した |
| 10 | `OBJ_SUPPORT` 一式 (`_NO_LS`・`_DIAG_NOFIX`・`T_MIP_OBJ_SUPPORT_*`) | (このコミット) | 185 (heuristics.rs の `obj_support`、solver.rs の呼び出しと `obj_support_gap`) | 75 | 412/0/19 | 一致 | 根拠: docs/improvement_history.md・analysis/revisit_20261112/inventory_abc.md (不採用: app1-1 は 4 コアでは既定でも解ける。40 問で当たるのは graph20-20-1rand だけ) |
| 11 | `BRCUT` 一式 | (このコミット) | 133 (brcut.rs を削除、separation.rs の効き目の記録、pseudocost.rs の点数の項と印、solver.rs の初期化) | 74 | 412/0/19 | 一致 | 根拠: analysis/brcut_20261110.md (58 問で SGM +2.3%・−0.3%、解けた 84 → 82、効き目は揺れの内) |
| 12 | `IMPLINT_TU` (旧版) | (このコミット) | 7 (solver.rs の刻みと `cut_int` の 2 つの分岐) | 74 | 412/0/19 | 一致 | 根拠: analysis/implint_tu_20261110.md (印が変わる 3 問で +164%、exp-1-500-5-5 が 5 秒 → 時間切れ)。**一部だけ**: 旧版の判定 `MipProblem::implied_integers_tu` (problem.rs) は、残す `IMPLINT_TU2` の診断 `DEBUG_IMPLINTCOUNT` (TU2 が旧版の印を全部含むかの比べ) とその単体テストが使うので、共有の部品として残し、注記を直した。環境変数の分岐だけを消した |

### 最後の確かめ (8cd80e088)

- `ident.py check`: 12 問とも消す前 (ecb1ded51) と (状態・目的値・ノード数) が完全に一致。
- `FUZZ_TAG=final`: fuzz_tiny 400 0 bad 0 (strict 13)・mip_fuzz_vs_highs 300 bad 0・fuzz_objint 1000 0 bad 0 (optimal 967・infeasible 33)・
  fuzz_slack 1000 0 mismatches 0・fuzz_tiny 400 1000 bad 0 (strict 14)・fuzz_objint 1000 5000 bad 0 (optimal 971・infeasible 29)・
  fuzz_slack 1000 5000 mismatches 0。cargo test 412 通過・0 失敗・19 無視 (消す前 422。消えた 10 本は消した試作の単体テスト)。
- 警告の行: 77 → 74。

## まとめ

- 消したのは 11 件 (#2〜#12。#12 は環境変数の分岐だけ)、src の正味 −4510 行。飛ばしたのは #1 `PROBE_EQUIV2_RESTART` (根拠が記録と合わない)。
- 共有の部品として残したもの: snfmodk.rs (SNFMODK)、`SRC_MODK` (MUSHAGG)、seplearn.rs の計測 (`SEPSTAT`)、`implied_integers_tu` (TU2 の診断)。
- 消した試作を参照する analysis/ の台本は直していない (過去の記録として残す)。

# 第 2 弾 (2026-11-18)

依頼は BRIEF.md の「第 2 弾」と `cands2.txt` (84 件、848f4bf9d)。手順・確かめは第 1 弾と同じ (ident.py の 12 問・cargo test・警告・
4 件ごとと最後に fuzz 7 本)。根拠は主に棚卸しの表 analysis/revisit_20261112/inventory.tsv (見送った理由と分類) と元の記録で確かめた。
始める前に 848f4bf9d で ident.py が base.json と一致することを確かめた (他のセッションの変更で既定は変わっていない)。

## 触らないもの (依頼書の除外)

`SYMREFL`・`SYM_LEX_CANON` (SYMDYN 系)、`IMPLINT_TU2`、`ROOTLP3` は依頼書の「計測中・判断待ち」なので触っていない。

## 1 コミットごとの結果

| 試作 | コミット | 減った行 (src) | 警告 | cargo test | ident | 根拠・備考 |
|---|---|---|---|---|---|---|
| `ZH_ONLY`・`ZH_PRE`・`ZH_TRIAL` (`T_MIP_ZH_TRIAL_*`) | (このコミット) | 298 | 74 | 411/0/19 (`random_validity_pre` が消えた) | 一致 | analysis/gap3_zhtrial_20261109.md (ZH_TRIAL=r は 58 問で既定に負ける、=1 は見分けにならない。ZH_ONLY は木で使う害)、analysis/cbcimpl_20261029.md (ZH_PRE は揺れの範囲)。zerohalf.rs の `coef_gcd` は ZH_PRE だけが使っていた |
| `RESTORE_EXACT_DSE`・`RESTORE_NEVER`・`SMALL3_SUBMIP_LIGHT`・`SMALL2_SUBLIGHT`・`LNS_NO_CAP`・`EARLY_RESTART_SKIP_LIGHT` | (このコミット) | 23 | 74 | 411/0/19 | 一致 | analysis/nodelp2_20261107.md (EXACT_DSE は行の多い LP で重い、NEVER は親の基底を戻す方が良い)、small_overhead3・2_20261102.md (58 問全体で悪い)、mip_primal_weakness_20261016.md (LNS_NO_CAP 不採用)、skiplight_20261029.md (SGM 同じか遅く解けた数が減る)。厳密な DSE 重みの経路は `NODELP2` の `X`・`x` が使うので残し、環境変数の判定だけ消した |
| `NODE_CUTS`・`TREE1_SB` (`T_MIP_TREE1_MINREL`)・`SUBMIP_FIRST_DONE`・`OPTFACE_FIRST`・~~`SB_POLY_SCORE`~~ (後で戻した、下の表) | (このコミット) | 110 | 74 | 411/0/19 | 一致 | beasleyC3_mc11_20261019.md (NODE_CUTS は一律にすると他の問題が悪化)、tree_close1_20261104.md (TREE1_SB は効かなかった)、smallprob_overhead_20261028.md (FIRST_DONE は単独で SGM が悪い)、optface_cuts_20261105.md (OPTFACE_FIRST はどちらも悪い)、cbcimpl_20261029.md (折れ線のスコアは効かない)。終わった順に受け取る経路は `submip_outcome_stop` (既定) が使うので残し、環境変数の判定だけ消した。折れ線の `poly_child_gains` は SB_TIGHTEN の診断が使うので残した |
| `TREE_SHARED_QUEUE`・`TREE_KEEP_SIBLINGS`・`TREE3_JOINCAP` (`T_MIP_TREE3_JOINCAP_*`)・`LOSE6_2_PROBE_WAIT`・`LOSE6_2_BGPROOF` | (このコミット) | 142 | 74 | 411/0/19 | 一致 | mip_primal_weakness_20261016.md (共有の待ち行列・兄弟を残す形は不採用、解けた数が 2 減る)、tree_close3_20261104.md (JOINCAP は待ったサブ MIP が良い解を出す問題で損)、halve_submip_20261030.md (LOSE6_2 の 2 件は drayage-25-23 の悪化で採らない)。共有の待ち行列の専用の部品 (`pop_hybrid`・`pool_donate_all`・`pool_donate_except_siblings`・`take_all_except`・`pops`) も消した。サブ MIP を止める仕組みは `SUBCAP` の `j` が使うので残した。`LOSE6_2_SUBLB` は対象外なので残した |
| `COST_PRIORITY`・`BG_RETURN_DSE` | (このコミット) | 66 | 74 | 411/0/19 | 一致 | cbcimpl_20261029.md (COST_PRIORITY は揺れの範囲)、halve_submip_20261030.md (BG_RETURN_DSE は得が揺れを超えない、採らない)。裏のカットの DSE 重みの受け渡し (`CutJob::dse`) は BG_RETURN_DSE だけが値を入れていたので一緒に消した (`set_dse_vars`・`dse_snapshot` は他が使うので残る) |
| `FC_NODE_SEP`・`FC_NODE_SEP2`・`FC_NODE_SEP_ADAPT` (それにしか効かない `FC_NODE_PATH_HIGHS`・`FC_SEP2_REPAIR`・`T_MIP_FC_SEP_*` を含む) | (このコミット) | 105 | 74 | 411/0/19 | 一致 | beasleyC3_mc11_20261019.md・20261020.md (FC_NODE_SEP は不採用、SEP2 は rout が 2〜3 倍・40 問未計測のまま、ADAPT は分離がほとんど走らない・方向を捨てた、PATH_HIGHS は mc11 で悪い)。木の中の分離は後の `TREE_SEP` (`tree_sep_fc`) が引き継いでいるので、その経路は残した |
| `MC11FIX1_RACE` (`T_MIP_MC11FIX1_RACE_SIZE`)・`DIVE_RCFIX`・`ROOT_SUBMIP_REL` (`T_MIP_ROOT_SUBMIP_REL_MIN`)・`ROOT_WIDE_SUBMIP` (`T_MIP_WIDE_SUBMIP_*`)・`FIX_CAP` | (このコミット) | 124 | 74 | 411/0/19 | 一致 | mc11_wait1_20261103.md (RACE は揺れの範囲、`i` は採らない)、cbcimpl_20261029.md (DIVE_RCFIX は 1 段目の良さが再現しない)、smallprob_overhead_20261028.md (ROOT_SUBMIP_REL は 10teams を 3→7〜45 秒)、highsonly16_summary_20261020.md (WIDE は一度既定にして外した、h80x6320d を遅くする)、mip_primal_weakness_20261016.md (FIX_CAP は A・C で悪化)。固定率で止める仕組み (`fix_cap_on`・`force_fix_cap`) は `SUBA_RATE` の `n` が使うので残し、環境変数の判定だけ消した。`root_submip_ref_secs` は SUBCAP が使うので残した |
| `CMIR_DELTA1`・`CMIR_DELTAS_HIGHS`・`CUT_AGE_HIGHS`・`CUT_EXTEND_CATCHUP`・`CUT_LP_CONTINUE` | (このコミット) | 52 | 74 | 411/0/19 | 一致 | mip_heur_cuts_impl_eval_20261012.md (DELTA1 は採らない)、sepspeed_20261023.md (DELTAS_HIGHS は mc11 の根の下界が大きく落ちる)、nodesep_aging_20261020.md (AGE_HIGHS は単独では逆効果)、physprimal_20261024.md (CATCHUP は候補にしない)、physiciansched_r3_20261020.md (CUT_LP_CONTINUE を含む組は nw04・misc07・neos-911970 を悪くする)。CMIR の本体 (既定の `SEP_FAST` の経路) からは分岐だけを外した。止まった LP を解き続ける仕組みは CUT_EXTEND・EQUIV2_CUTS・rch の `L` が使うので残した |
| `CUT_KEEP_LU`・`CUT_EXTEND`・`CUT_HARRIS2`・`ORIGROW_TIGHT` (`T_MIP_ORIGROW_NODE_SLACK`) | (このコミット) | 119 | 74 | 411/0/19 | 一致 | cbcimpl_20261029.md (KEEP_LU は両方の組で悪い側・解けた数も減る、ORIGROW_TIGHT は揺れの幅を超えない)、mzzv42z_r4_20261021.md (CUT_EXTEND は悪くなった)、puriri_cutlp_20261106.md (HARRIS2 は単独では根の下界が変わらず、組でも「このままでは既定にしない」)。KEEP_LU だけが使う `replace_cut_rows`、ORIGROW_TIGHT だけが使う `orig_row_modes`・`has_cont_col` も消した。時間の上限を延ばす仕組みは `CUT_AGG` の `e` が、`Harris2Scope` は `NODELP_PERTURB` の `h` が使うので残し、環境変数の判定だけ消した (`MipLp::delete_and_add_rows` は LP の API として残る) |
| `RINS_ROOT_BOOST`・`RINS_DIVE`・`RENS_DIVE`・`RR_HIGHS` (`=lock` を含む)・`SUBMIP_NESTED` (`T_MIP_SUBMIP_NESTED_*`) | (このコミット) | 88 | 74 | 411/0/19 | 一致 | physprimal_20261024.md (RINS_ROOT_BOOST が悪化の元)、mip_primal_weakness_20261016.md (RINS_DIVE は改善と悪化が混ざる、RR_HIGHS 不採用)、beasleyC3_r3_20261020.md・mzzv42z_neos-960392_20261020.md (RENS_DIVE は決め手にならない・効果なし)、halve_submip_20261030.md・neos960392_nested_20261101.md (SUBMIP_NESTED は採らない)。`rins_dive` も消した。ダイビング式の近傍 (`dive_neighbourhood`) は暫定解なしの RENS・木の腕が使うので残し、暫定解を渡す RINS の形の分岐は今は使われない (引数 `inc` はいつも `None`) と注記した |
| `INEQ_SINGLETON`・`CS_OBJ_PIVOT`・`DUAL_UB`・`FJ_LP_REPAIR`・`IMPL_PROP` | (このコミット) | 8 | 74 | 411/0/19 | 一致 | highsonly16_summary_20261020.md・physiciansched6-2_fj_20261020.md (前の 4 件を全体で使うと physiciansched6-2 に解は出るが解けず、DUAL_UB + CS_OBJ_PIVOT が uccase12 を 11507 → 798739 にする害)、cutmix_20261111.md (IMPL_PROP は見込みが小さい)。**全体で有効にする環境変数だけを消した**: 前の 4 件の処理は既定の `FJ_P3` (FJ だけに渡す問題の前処理) が `fj_p3_force` で使い、含意の表は `PROBE_CLIQUE`・`CLQPRB` が使うので本体は残した |
| `DUPCOL`・`PROBE_EQUIV_FIX`・`ROWNORM`・`HO3_ENUM` (`T_MIP_HO3_ENUM_*`) | (このコミット) | 243 | 74 | 409/0/19 (DUPCOL・ROWNORM の単体テスト 2 本が消えた) | 一致 | presolve_size_20261025.md (DUPCOL は nw04 が 3 倍遅い)、physiciansched_r4_20261021.md (PROBE_EQUIV_FIX は条件が広すぎる)、rownorm_20261106.md (SYMNORM が既定になった今、既定にする理由はない)、gate2_20261117/ho3sl/README.md (HO3_ENUM は net12 の 1 問合わせになり見込みが薄い)。`duplicate_binary_fixings`・`normalize_row_signs` も消した。同値な列の代入へ進む経路は `PROBE_EQUIV2` の `PROBE_EQUIV_FORCE` が、行の列挙は `ENUM_PRESOLVE` が使うので残し、試作の分岐だけ消した |
| `RESTART_PC`・`RESTART_REF`・`RESTART_CUTS_READD`・`IMPLBD_GENINT` | (このコミット) | 17 | 74 | 409/0/19 | 一致 | restart_pc_20261031.md (RESTART_PC は効かない・損は減らず得を失う)、mzzv42z_r3_20261020.md と後の対の計測 (RESTART_REF は drayage-25-23・uccase12・neos-911970 を落とし不採用)、mzzv42z_r4_20261021.md (READD は mik-250 の根を弱める)、mzzv42z_r3・gfr_split (IMPLBD_GENINT を全体で使うと neos-1456979・roll3000 が悪化気味)。**全体で有効にする環境変数だけを消した**: 仕組みは既定の `GR_AUTO`・`GR_RESTART` (一般整数のある問題だけで使う) と木の中の再スタートの引き継ぎが使うので残し、注記の名前もその仕組みの名前として残した |
| `NODE_PERTURB`・`SIMPLEX_POLISH_DSE`・`POOL1_DONATE` | (このコミット) | 63 | 74 | 409/0/19 | 一致 | cbcimpl_20261029.md (NODE_PERTURB は悪い側に出た)、rootlp_simplex_fail_20261105.md (POLISH_DSE は `1`・`2` とも効かない)、pool_bound1_20261104.md (POOL1_DONATE は既定にする理由はない)。摂動の判定 (`node_perturb_criterion`) は `NODELP_PERTURB` の `p` が使うので残し、環境変数の判定だけ消した。`pop_for_donate` も消した |
| `OPTFACE_MOVE` (`T_MIP_OPTFACE_FLAT`・`_GAIN`・`_TRIES` …) | (このコミット) | 108 | 74 | 409/0/19 | 一致 | optface_cuts_20261105.md (`f`・`d` とも neos859080 を悪くした)。根のカットのループの前後の試作 (`optface_pre`・`optface_second_loop`) を消した。最適面の上の別の頂点を作る部品 (`optface_alt`・`TwoStageLp::optface_move`・`OPTFACE_MOVE_*` の時間の調整値) は `OPTFACE_PARCUTS` (revisit で測り直した候補)・`MPSEP` が使うので残した |
| `CUTSEL_PROBE_KEEP`・`INTVUB`・`FACE_ADAPT` (`T_MIP_FACE_ADAPT_*`) | (このコミット) | 63 | 74 | 409/0/19 | 一致 | small_overhead3_20261102.md (PROBE_KEEP は nubu で悪い)、intvub_20261106.md (INTVUB は noswot の木が閉じなくなる害、=2 は効かない)、revisit_20261112 の face (+3.3%・84→82) と gate2_20261117/sqface (今の既定では面の分離がほぼ動かず FACE_ADAPT は実質何もしない)。比べの終わりの状態 (`ProbeEnd`) は別の呼び出し (`keep` = 真) が使い、一般整数列の変数上下限の置き換えは `MUSHAGG` (`with_intvub_mode`) が使うので残し、環境変数の判定だけ消した。`face_off_loop`・`last_cut_lp_secs` は比べの状態の保存に入っているので残した (今は使われない) |
| `LOCALMIP` (`=1`・`=2`)・`LOCALMIP_CHAIN` | (このコミット) | 67 | 74 | 409/0/19 | 一致 | gate2_20261117/ho3sl/README.md (今の既定は FJ のスレッドで Local-MIP + 連鎖を回していて (`FJ_LMIP_MIX`)、別スレッドを足す差は揺れの範囲。既定に吸収済み)。別スレッドの Local-MIP (`lmip_thread`) と `local_mip` を消した。Local-MIP の本体 (`local_mip_opts`・連鎖の手) と受け渡し箱は既定の `FJ_LMIP_MIX` が使うので残した (`poll_lmip_thread` の名前も残る) |
| `LP_PROBE_AFTER_EXT` | (このコミット) | 87 | 74 | 409/0/19 | 一致 | lp_probe_after_ext_20261102.md (58 問 2 回: SGM +2.7%・解けた 87 → 86、常に基底を渡す形は +9.4%・87 → 80。根の頂点が変わる揺れに負ける)。クリーク行の拡張の後の LP による固定とかけ直しの道を消した。`lp_probe` の引数 `try_fix` は今はいつも真 (注記した) |
| `BG_DETERM_ROBUST` (`T_MIP_BG_DETERM_RATIO`・`_LEN`・`_GAP`・`_WAIT_MULT`) | (このコミット) | 121 | 74 | 409/0/19 | 一致 | robust_stall_20261102.md・halve_submip_20261030.md (=o は h80x6320d が 3 回とも時間切れ・58 問 −1.6%・解けた −3、=ao は待ちすぎの費用、=p+o の組も h80x6320d の揃った損で採らない)。`BgRobust`・裏のカットの拒否 (`veto_own_stall`)・第 2 段階の強制 (`robust_phase2`)・その状態 (`flat_run`・`robust_stalled`・`vetoes`) と、`wait_determ` の使われなくなった引数を消した。裏のカットを始めたときの値 (`start_bits`) は `HO5_BGSTALL`・`CUTSTALL_GAP`・`S4_BGZERO` が使うので残した |

## 飛ばしたもの (第 2 弾、13 件)

| 試作 | 理由 |
|---|---|
| `NODELP2` | analysis/gate2_20261117/nodelp2s/README.md (11-17) が「試作のコードと台本は残す」と決めている。元の部品の `p`・`b` は既定に入っている |
| `DFSPQ` | analysis/gate2_20261117/dfspq/README.md (11-17) が「既定では働かないので残す」と決めている |
| `MC11FIX2` | analysis/revisit_20261112.md の「次の候補になりうるが今回は外したもの」に `MC11FIX2=b` が挙がっている |
| `ROOT_BASIS_FIX` | analysis/highsloss_20261113.md で neos-873061 の根の LP を 72 → 10 秒にした記録があり、派生の試作 `ROOT_BASIS_FIX_BIG` が同じ部品を使う |
| `SUBA_RATE` | 不採用の記録は文字 `n` だけ (halve_submip_20261030.md の bench30)。`i`・`t`・`z` の根拠がない |
| `FOCUS` | 不採用の記録は `primal` だけ。`auto` の根拠がない |
| `FACE_SEP` | 既定の形 (`int2`) を選ぶ環境変数で、不採用は `=off` の形だけ |
| `SYNTH_CLOCK_FACTOR` | 試作ではなく、出荷時の値 `SYNTH_CLOCK_FACTOR` を上書きする再較正のための調整つまみと注記されている (不採用は値 4・8 の実験だけ) |
| `CUTSEL_HIGHS` | 単独の不採用の記録がない (組み合わせでの不採用と「採否なし」だけ)。既定の `CUTSEL_PROBE` の `S` と部品を共用する |
| `CUTSEL_HIGHS_GUARD` | `fc` の形は既定 (`CUTSEL_PROBE=rate` の `cutsel_guard_fc`) に取り込まれていて、残る「混ぜる形」の不採用の記録がない |
| `NODESEL_BANDIT` | analysis/treeclose_20261114.md 9.13・9.30 で 60 秒は中立、600 秒では既定より速い面があり (`=1` は解けた数 +3)、不採用とは書かれていない (後の DFSBB に移ったが判断の記録がない) |
| `SB_POLY_SCORE` | **一度消してから戻した**: 親から「条件付きで入れ直す候補 (sbt = SB_TIGHTEN・SB_POLY_RC など) と同じ部品に触れうるので消さない」と追加の注意があり (消した後に読んだ)、select_branch_inner の分岐を元のとおりに戻した (戻した後も ident.py の 12 問が一致・cargo test 409 通過) |
| `HNFCUT` | `HNFPRE` (PRUNE の受け持ちの既定の外し) と部品 (`hnf::mode`・`cuts.rs` の `Sub::Cong`・分離の `set_kind`) を共用し、HNFPRE の前処理の起動にも関わる |

依頼書の除外 4 件 (`SYMREFL`・`SYM_LEX_CANON`・`IMPLINT_TU2`・`ROOTLP3`) は触っていない。

## fuzz (第 2 弾)

どれも analysis/cleanup_20261118/fuzz.sh (release019 と同じ 7 本、既定) で、7 本とも rc=0・bad 0・mismatches 0、cargo test 全通過:

| タグ | ビルド | それまでに消した件数 | 結果 |
|---|---|---|---|
| p2a | d615d07f0 | 9 | bad 0 (strict 13・14、objint optimal 967/971)、cargo test 411 |
| p2b | 679f878e4 | 24 | 同上、cargo test 411 |
| p2c | efa4591ad | 34 | 同上、cargo test 411 |
| p2d | c87a85ea8 | 43 | 同上、cargo test 411 |
| p2e | 16f17b2ca | 56 | 同上、cargo test 409 |
| p2f | 3ae3f2653 | 64 | 同上、cargo test 409 |
| p2final | 6acccd5b2 | 68 (全部) | 同上 (objint optimal 967/971・slack mismatches 0)、cargo test 409。`ident.py check` も一致 |

## まとめ (第 2 弾)

- 対象 84 件のうち、依頼書の除外 4 件を除く 80 件を処理した。**消したのは 67 件** (19 コミット + SB_POLY_SCORE を戻した 1 コミット)、src の正味 **約 −1892 行**。飛ばしたのは 13 件 (上の表)。
- 多くは「全体で有効にする環境変数」だけを消し、既定の経路や他の試作が使う本体は残した (FJ_P3 の前処理 4 件、GR の再スタート・
  一般整数の含意 4 件、固定率の止め方、終わった順の受け取り、Harris2Scope、CUT_AGG の `e` の延長、ProbeEnd、MUSHAGG の
  `with_intvub_mode`、PROBE_CLIQUE の含意の表など。行ごとの備考に書いた)。
- 毎回: cargo test 全通過 (411 → 409。減った 2 本は消した試作の単体テスト、第 2 弾の始めの 412 から ZH_PRE の 1 本を含め計 3 本)、
  警告の行は 74 のまま、1 スレッドの 12 問 (状態・目的値・ノード数) が消す前 (base.json) と完全に一致。fuzz 7 本は 7 回とも bad 0。
- 消した試作を参照する analysis/ の台本は直していない。docs/ に使い方の説明があったものはない (backlog の SEPLEARN は第 1 弾で注記済み)。
