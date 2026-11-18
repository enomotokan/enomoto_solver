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
