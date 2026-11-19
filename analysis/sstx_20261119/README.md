# SSTX: SST の行に反するヒューリスティクスの解を写して使う試作 (2026-11-19)

依頼 (親、利用者の発案): 試作 `ENOMOTO_MIP_SYM_SST` (orbital fixing の代わりに Schreier-Sims の表のカット `x_l − x_k ≥ 0` を足す、
analysis/hang_20261117 の 7・15 節) は P +21%・M +41% 遅い。一因として「ヒューリスティクスの解が SST の行に反して捨てられる」と推測されている。
そこで、SST の行に反する解も使える形を試作する。

## 1. 試作 `ENOMOTO_MIP_SYM_SST=x` (既定は不変)

- `symmetry::sst_pairs_except(…, keep_chain)` が、行を足した段ごとの leader `l_t` とその段の群 `G_t` の生成元 (`SstLevel`、動かす列だけ) も返す。
  スレッドごとの `SYM_SST_CHAIN` に置き、求解 (`sst_chain`)・木の写し (`TreeSnapshot`、並列のスレッド)・サブ MIP の退避 (`SymScope`) に渡す。
  対称性の検出のたびと、外側の求解の終わりに消す。
- `try_incumbent` で、解が今の問題 (SST の行を含む) で実行不能なら:
  1. 元の問題 (`orig` = 対称性の行を足す前。行・境界・整数性) で判定する (従来どおり)。実行不能なら捨てる。
  2. オービトープの並べ替え (従来どおり) の後、`symmetry::sst_canonicalize` で写す。段ごとに、`l_t` の軌道 (`G_t` で) の中で値が最大の列 `k` を
     幅優先の木の道を逆にたどり、生成元の逆で `l_t` へ写す。`G_t` の元は前の段の leader を固定し、前の段の軌道をそれ自身に写すので、
     前の段の行は満たされたまま。写しは対称性なので、元の問題での実行可能性と目的値は変わらない (debug ビルドで assert)。
  3. 写した解が SST の行を満たせば、普通の暫定解 (解の溜め・1-opt などもそのまま)。
  4. 写しに失敗したとき (写しても SST の行に反する (許容差の際など)・写した解の確かめ (元の問題で実行可能・目的値が同じ) に落ちる) は、
     **元の x に戻して**「元の問題の解」として暫定解にする (`ENOMOTO_MIP_SYM_ORIG_INC` と同じ道。利用者の追加の指示)。
     - 目的値は写しの成否にかかわらず暫定値 (打ち切り・目的の伝播・被約費用の固定) に使う。最適解は失われない (目的と行が生成元で
       不変なので、同じ値の像が SST の行を満たす)。
     - 返す答えは元の x (元の問題で実行可能)。
     - 改善ヒューリスティクス: 解の溜め (クロスオーバー・ミューテーションの親) に入れず、1-opt・連続変数の解き直しもしない。
       暫定解の列の値を読む処理 (2 節の RINS・ローカル分枝など) には **元の x をそのまま渡す** (従来の ORIG_INC と同じ。SST の行に反する点なので、
       SST の行を含むサブ MIP では開始解にならないことがある)。
     - 写しに手間の上限は付けていない (段ごとに軌道を幅優先で 1 回たどるだけで、段の数と生成元は SST の行を作るときの上限で抑えられている)。
  5. 再スタートの後 (`restarts > 0`) で写しても反する解は、従来どおり捨てる (`orig` が再スタート前の問題のことがあり、元の問題での
     実行可能性を判定できない)。写して SST の行を満たした解は再スタートの後も使う。
  6. 回数: 外側の求解の終わりに、ログの有無にかかわらず `MIP: SST=x: mapped A, failed (rows still violated) B, failed (check) C, dropped after a restart D`
     を標準エラーに出す (全スレッドの和)。`run.py` は sstx の回でこの行を結果の `result.sstx` に入れる。
- 実行可能性問題で既定の SST (`ENOMOTO_MIP_SYM_SST_FEAS`) にも、`SYM_SST=x` なら同じく写す。

## 2. 暫定解の列の値そのものを使う処理 (写した代表が使われる所。src/mip/ からの相対、2026-11-19 の版)

| 種類 | 箇所 |
|---|---|
| RINS・DINS の近傍の固定 | heuristics.rs `rins_inner` (2934・2951、SUBFIX の道は heur_subfix.rs `subfix_dive_with` 256)、heur_scip.rs `dins` (99)、solver.rs `fjsub_offer` (15720) |
| ローカル分枝・近接探索の行 | heuristics.rs `local_branching` (3228)・`proximity_search` (3262)、`hamming_row` |
| ミューテーション・固定の広げ狭め | heuristics.rs `mutation` (3297)、subb.rs `subb_diversify` (186)・`subb_pool_lns` (286〜) |
| クロスオーバーの親 (解の溜め) | heur_scip.rs `crossover_from` (57〜76)、subb.rs `subb_pool_lns`、木の ALNS (heuristics.rs 3378)。溜めの重複の判定 heur_scip.rs `pool_add` (16) |
| ガイド付きダイブ | heuristics.rs `dive` (569) |
| Local-MIP・FJ の開始点・注入 | heuristics.rs `subheur_local_mip` (2138)、solver.rs `fjsub::spawn` (11832 → fjsub.rs 66)・`role_lmip_slice` (15149)・`restchain_w` (15180)・`root_lmip` (15225)・`lmip_boost` (22961)・`post_prechain` (22884 → prechain.rs 322) |
| ALNS の基準点 | solver.rs `heur_role_run` (15394) |
| スレッド間の共有 | solver.rs `start_workers` (23341)、`try_incumbent` の公開 (7486)、`heur_role_import` (15107)、`join_workers` (25006) |
| サブ MIP・再スタートへ渡す開始解 | solver.rs `maybe_restart` (22242、RESTART_SOL)、mod.rs 1365・508・544・1415・1686〜1700 (START_SOL・REF_SOL)、solver.rs `run` (9266〜9275) |
| カットの判定 (暫定解を切るカットを捨てる) | separation.rs `separate` (6082・6142・6461・6827) → `finish_cut` (7717) |
| 最終の答え・結果 | solver.rs `finish` (25628、orig_best は 25177)、`maybe_restart` (22302)、`resume_pause` (17400)、DIVE_ONLY (10545・10598・10585) |

- 1-opt・2-opt (`try_incumbent` の中) は新しい候補の x を磨く (保存済みの暫定解は読まない)。`finish` の周りの 14334・25647 は目的値だけ。
- 1 の 4 (写しに失敗した解) は元の x のまま上の処理に渡る。SST の行を前提にするもの (サブ MIP の行・カットの判定・RINS の固定) では
  SST の行に反する点になる。群の元で写せば行を満たす (1 の 2) ので、失敗は許容差の際などに限る想定 (回数は 1 の 6 で数える)。

## 3. 確かめ

- `cargo test --release --lib`: 410 通過・0 失敗 (写しの単体テスト `sst_canonicalize_satisfies_sst_pairs` を含む。S4 と C3 の群で 200 点を写し、
  SST の行を全て満たし群の像であることを確かめる)。
- fuzz (`fuzz/`、`CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true pip install -e .` で debug の assert を効かせたビルド、`ENOMOTO_MIP_SYM_SST=x`):

| fuzz | bad | panic | 写せた | 写しても反した | 確かめに落ちた | 再スタート後に捨てた |
|---|---|---|---|---|---|---|
| `mip_fuzz_symmetric.py 300 0 1` | 0 / 300 | 0 | 21 | 0 | 0 | 0 |
| `mip_fuzz_symmetric.py 300 0 2` | 0 / 300 | 0 | 10 | 0 | 0 | 0 |
| 同 1 + `ENOMOTO_MIP_NO_ORBITOPE=1` | 0 / 300 | 0 | 98 | 0 | 0 | 0 |
| 同 2 + `ENOMOTO_MIP_NO_ORBITOPE=1` | 0 / 300 | 0 | 169 | 6 | 0 | 6 |
| `mip_fuzz_vs_highs.py 300` | 0 / 300 | 0 | 0 | 0 | 0 | 0 |

- 「写しても反した」6 回は全て再スタートの後 (同じ数だけ捨てた)。再スタートの後は列の番号が変わるのに、段の控えは最初の回のまま
  (再スタートで対称性を検出し直さないとき) なので写しが当たらない (推測。害はない: 写した解は SST の行で判定し、反すれば元の x に戻す)。

## 4. 計測 (分担は親が割り振る)

- 対象: hang_20261117 の `symscan_M.txt`・`symscan_P.txt` で生成元が 1 つ以上の問題 (M 19 問、P 21 問)。× 種 1〜3 × {def, sst, sstx}、4 スレッド。
- `shards_M.json` (600 秒、8 分担、各約 2.0 時間): `python run.py M K` (K = 0〜7)
- `shards_P.json` (60 秒、2 分担、各約 0.9 時間): `python run.py P K` (K = 0, 1)
- 集計: `python summ.py M def sst sstx > summ_M.txt` (def と sst・def と sstx・sst と sstx を比べる)。P も同じ。
- 準備: `pip install maturin highspy==1.15.1 && pip install -e .` (この試作を含むコミット以後でビルド)。作り直しは `python plan.py 8 2`。
