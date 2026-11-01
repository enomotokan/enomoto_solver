# cryptanalysiskb128n5obj14・16 の時間の上限の超過 (FJ のスレッドの前処理が期限を見ない、2026-11-01)

- 機械: 4 CPU。ソルバーは同時に 1 つだけ動かした。HiGHS (highspy) は 1.15.1。4 スレッド・60 秒、`ENOMOTO_MIP_LOG=1`。
- 土台: ブランチ `ccr-1beed81b-ex0uhq` の `5e66bc2` (bench10 の記録)。
- 不具合: bench9 (rep 103) で cryptanalysiskb128n5obj14 が全設定で 74.1〜81.8 秒、bench10 (rep 104) で cryptanalysiskb128n5obj16 が
  全設定で 63.2〜64.6 秒 (`analysis/halve_submip_20261030/bench9_s*.jsonl`・`bench10_s*.jsonl`)。同じ rep の中では設定によらず
  超えるか超えないかがそろい、もう一方の rep では 60.1 秒。
- 台本とログは `analysis/overrun_crypt_20261101/`:
  - `watch.sh 問題 上限 回数 接頭辞 [環境変数 ...]`: `analysis/cand_default_20261031/watch.sh` と同じ (上限 + 1.5 秒 (wall) を過ぎたら gdb で全スレッドのスタック)。
  - `watch_at.sh 問題 上限 開始秒 接頭辞 [環境変数 ...]`: 起動から 開始秒 (wall) を過ぎたら終わるまでスタックを取り続ける (期限の直後を見る。
    gdb が止める分だけ時刻は伸びる)。
  - `hog.sh 本数 コマンド...`: CPU を食う裏の処理 (busy loop) を 本数 だけ走らせてコマンドを実行する (遅い機械の代わり)。
  - `ab.sh 裏の処理の本数 回数 問題...`: 直した形 (on) と `ENOMOTO_MIP_NO_FJ_P3_DEADLINE=1` (off) を交互に、`hog.sh` の下で回す。
  - `logs/`: 接頭辞 `base` 直す前 (裏の処理なし)、`hog4` 直す前 (裏の処理 4 本)、`ab4_v1/`・`ab4_v2/`・`diag`・`h4fix` 途中のビルド (裏の処理 4 本、下の 2 節)、
    `at_r*` 期限の直後のスタック、`ab8_*` 裏の処理 8 本、`fin`・`oth` FJ・対称性の直しの後 (分離の直しの前)、`bglog` rail507 の裏のカットのログ、
    `final`・`final2` 最終のビルド (裏の処理なし)。`*_summary.txt` は 1 行ずつのまとめ (wall は Python の起動・読み込みを含む)。
- 秒は `run_miplib_benchmark.py` の `time` (`m.solve` だけを測る。読み込みは含まない。ソルバーのログの `solve returned ... after` との差は 0.01〜0.2 秒で、
  超過はどれもソルバーの中の時間)。
- `CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-C link-arg=-lpython3.13" cargo test --release --lib` は 333 件通過。

## 1. 原因

事実:

1. 今の既定を裏の処理なしで 4 回 (`base`) 回すと 60.10〜60.17 秒で、再現しない。時間の配分は、LP によるプロービング 30 秒
   (安全網の上限いっぱい、根の LP は解けない)、プロービング 3 回 + 同値の探り約 17 秒、根の LP 約 13〜14 秒 (解けずに時間切れ)。
   前処理が上限の 3/4 を使う。
2. CPU を食う裏の処理を 4 本走らせて機械を遅くすると再現する (`hog4`、67.26 秒)。上限 + 1.5 秒以降のスタックは 4 回とも、主が `Solver::run` →
   `join_fj_thread` (根の LP が時間切れで止まった後、FJ のスレッドに止めの合図を出して待つ)、FJ のスレッドが `fj_p3_problem` → `presolve_mip_depth` →
   `substitute_probe_equiv` → `presolve_mip_depth` → `probe` (FJ だけが使う問題 FJ_P3 を作る前処理のプロビング)。
3. FJ のスレッドは `std::thread::spawn` で作るので、前回 (`analysis/bgw_overrun_20261031.md`) 足した前処理の止め `presolve_stop` の期限
   (`PRESOLVE_DEADLINE`、スレッドごとの値で `solve_problem_inner` が置く) も止めの合図 (`crate::cancel::halted`) も付いていない。FJ_P3 の前処理は
   主の前処理と同じ重さ (プロビング 3 回で約 15 秒) で、主の前処理が終わってから (約 46 秒) 始まるので、普段の機械でも 60 秒の少し前に終わる程度。
   機械が遅いと期限をまたぎ、主は `join_fj_thread` でその終わりを待つ (FJ の手は止めの合図を見るが、前処理は見ない)。
4. bench9 の rep 103・bench10 の rep 104 で、同じ rep の全設定がそろって超えたのは、その時間帯の機械が遅かったためと見られる。
   裏の処理 8 本 (約 3 倍遅い) の下で off (直す前と同じ FJ の形) は 70.7〜94.2 秒 (`ab8_off`) で、bench の 74〜82 秒と同じ大きさになる。
   前回の調べで旧ビルドで再現しなかった (60.05 秒) のも、機械が速い時間帯だったためと見られる。
5. 直す過程で、遅い機械でさらに次の手順が期限の後も回っていた (`diag`・`at_r*`・`h4fix` のログとスタック):
   - FJ_P3 の前処理の、プロビングの後の同値な列の代入による前処理のかけ直し (`substitute_probe_equiv` → `presolve_mip`)。
   - FJ のスレッドの、FJ_P3 の前処理の後の `fj2_search` の初期化 (`Fj::reset_moves`、全列) と最初の確認 (手間 50 万) までの手 (止めの合図を見ない)。
   - 主の前処理が期限を過ぎて終わった後の対称性の検出 (`add_symmetry_rows`、3〜4 倍遅い機械で 2.7〜2.9 秒)。
   - 主の前処理が期限を過ぎた後に始まった FJ のスレッドの FJ_P3 の前処理 (2.75 秒)。

## 2. 直し方 (`src/mip/mod.rs`・`src/mip/solver.rs`)

どれも期限を過ぎた後だけ効く (時間内の解・下界は変わらない)。`ENOMOTO_MIP_NO_FJ_P3_DEADLINE` で 1〜3 を外す (切り分け用、直す前と同じ)。
4・5 は前回の `ENOMOTO_MIP_NO_PRESOLVE_STOP` で外れる。

1. `with_presolve_deadline(期限, f)`: このスレッドの `PRESOLVE_DEADLINE` を置いて `f` を実行し、戻す。FJ のスレッドと Local-MIP のスレッド
   (`ENOMOTO_MIP_LOCALMIP`、試作) の `fj_p3_problem` をこれで包み、期限は主の求解の期限 (`Solver::deadline`)。これで FJ_P3 の前処理のプロビング・
   クリーク行の拡張が期限で止まる。
2. FJ のスレッドは、始めた時点で期限を過ぎていれば FJ_P3 の前処理をせず、FJ_P3 の前処理の後で期限を過ぎていれば FJ を始めずに戻る
   (`past_presolve_deadline`。期限の後に見つかる解は時間内の解ではない)。
3. ログ: `MIP: FJ problem (FJ_P3): ... (秒数[, past the deadline])`、`MIP: FJ thread joined ...s after the stop signal` (0.05 秒以上のとき)、
   `MIP: root LP stopped at the time limit (..., ...s past the deadline)`。
4. 同値の探りの 2 巡目 (`probe` の中、`equivalence pass with implications`) も 16 列ごとに `presolve_stop` を見る。期限を過ぎていれば
   同値な列の代入のかけ直し (`substitute_probe_equiv`) をしない (代入しない問題も正しい)。
5. `solve_problem_inner`: 前処理が期限を過ぎていれば (`presolve_stop`) 対称性を探さない (ログ `MIP: symmetry: skipped (the presolve passed the time limit, ...)`。
   残り時間がないので求解はすぐ時間切れで終わる)。

### 2.1 ほかの問題の確かめで見つけた超過: 裏のカットのラウンドが期限の後も回る (`src/mip/separation.rs`)

FJ・対称性の直しの後に 8 問を回すと (`oth`)、rail507 が 65.11 秒、fast0507 が 61.39 秒だった。主は `BgCuts::finish` で裏のカットのスレッドの
終わりを待ち (`background cuts: no result (waited 4.98s)`・`1.22s`)、そのスレッドは根のカットのループの分離 (`separate` の表のカットの並列の生成、
`finish_cut`・`cuts::substitute`)・プールへの追加 (`add_to_pool`)・選択 (`select_cuts_hybrid_anchor`) を回していた。裏のカットのスレッドは止めの合図を
ラウンドの頭でしか見ず、rail507 では 1 ラウンドの分離が 3〜9 秒かかる (`bglog`)。今回の直しとは関係のない、元からある超過
(bench9・10 の既定でも rail507 61.54 秒・fast0507 61.02 秒)。

直し方 (既定で使う。`ENOMOTO_MIP_NO_SEP_DEADLINE` で使わない (切り分け用)):

- 根のカットのループで、分離から戻った時点で自分の求解の期限を過ぎていれば、そのラウンドを捨てて抜ける (ラウンドの頭の `time_up` で抜けるのと同じ状態。
  ログ `MIP: cut round N: the time limit passed during the separation (...); round dropped`)。裏のカットの求解の期限は主の期限と同じ。
- 表のカットの並列の生成 (2 か所、逐次の形は行ごとに `time_up` を見ている) も、期限を過ぎたら残りの行を作らない
  (元の行の並列の分離 `sep_parallel_parts` は前から 64 行ごとに期限を見ている)。

## 3. 確かめた結果

### 3.1 遅い機械 (裏の処理) での比べ

| 形 | 裏の処理 | obj16 | obj14 |
|---|---:|---|---|
| 直す前 (`hog4`) | 4 本 | 67.26 | − |
| off (`ab4_v1`・`ab4_v2` の off) | 4 本 | 63.81・68.69・68.71・70.73 | 65.24・68.71・67.24・73.08 |
| on (1 のみ、`ab4_v1`) | 4 本 | 60.87・61.08 | 60.29・60.31 |
| on (+ 4 の代入の止め、`ab4_v2`) | 4 本 | 61.14・60.38 | 60.90・60.89 |
| on (+ 2 の FJ を始めない、`h4fix` の r1・r2) | 4 本 | 61.09・60.31 | 60.90・60.97 |
| off (`ab8_off`) | 8 本 | 70.76・75.63 | 94.25・70.69 |
| on (最終の FJ・対称性の形、`ab8_on`) | 8 本 | 60.34・60.62 | 60.93・60.51 |

- `h4fix` の r0 (65.88・67.53 秒) は計測中にコンパイルを同時に走らせてしまった回で、主の前処理そのものが期限を越え、その後の対称性の検出・
  FJ_P3 の前処理が止まらなかった (上の 1 節の 5)。これを見て 2・5 を足した。
- `ab8_off` は `ENOMOTO_MIP_NO_FJ_P3_DEADLINE=1` で FJ の直し (1〜3) だけを外した形で、4・5 (前処理の止め) は入っている。
- 裏の処理 8 本の下の on の残りの超過 (0.3〜0.9 秒) は、主の前処理が期限を少し (0.2〜0.4 秒) 過ぎて終わり、根の LP を作って止まるまで (0.2〜0.4 秒)。

### 3.2 普段の機械 (裏の処理なし、最終のビルド、`final`・`final2`)

| 問題 | 今回 (秒) | 状態・解 | bench9 / bench10 の既定 | 直す前の中間のビルド (`oth`、FJ・対称性の直しのみ) |
|---|---|---|---|---|
| cryptanalysiskb128n5obj16 | 60.19・60.19・60.12 | 時間切れ・解なし | 60.10 / 64.58 | 60.14・60.09・60.07 (`fin`) |
| cryptanalysiskb128n5obj14 | 60.05・60.09・60.15 | 時間切れ・解なし | 74.13 / 60.15 | 60.13・60.16・60.09 (`fin`) |
| sorrell3 | 61.03 | −13 | 61.41 / 60.94 | 61.15 (−13) |
| rail507 | 60.22・60.37・60.25 | 178 | 61.54 / 60.19 | 65.11 (178) |
| fast0507 | 60.16・60.25・60.23 | 176 | 61.02 / 60.67 | 61.39 (176) |
| neos-957323 | 60.25 | −237.7565 | 61.68 / 60.14 | 60.12 (−237.7567) |
| sp98ar | 60.28 | 539927687.84 | 60.10 / 61.69 | 60.11 (535793846.56) |
| supportcase42 | 61.40 | 40 | 61.22 / 61.33 | 60.04 (40) |
| radiationm40-10-02 | 61.91 | 163368 | 60.24 / 62.11 | 60.26 (158565) |
| snp-02-004-104 | 62.28 | 601404252.4 | 60.26 / 62.35 | 60.27 (586915253.8) |

- 2 問は 6 回とも上限 + 1 秒以内 (最大 +0.19 秒)。rail507・fast0507 も 3 回ずつ上限 + 1 秒以内になった。
- 解の値の違い (sp98ar・radiationm40-10-02・snp-02-004-104) は時間切れの回の揺れ (bench9・10 の既定でも同じ幅で揺れる) で、
  今回の直しは期限の後だけ効くので関わらない。
- sorrell3・supportcase42・radiationm40-10-02・snp-02-004-104 の +1.0〜2.3 秒は、bench9・10 の既定と同じ大きさで、今回の直し (期限の後だけ効く)
  による悪化ではない。どれも LP (双対単体法) の中の期限を見ない手順で、上限 + 1.5 秒の時点のスタックは:
  - sorrell3: 主の根の LP の `warm_start_basis` → LU 分解 (`find_best_pivot`)。根の LP が期限を 0.85 秒過ぎて止まった。
  - supportcase42: 主の根の LP の `rollback_pivots` → `rollback_core` → LU 分解。根の LP が期限を 1.36 秒過ぎて止まった。
  - radiationm40-10-02: 主の RINS のサブ MIP の LP (`PriceMat::repartition`)、裏のカットのスレッドの zerohalf の分離。裏のカットの待ち 1.75 秒。
  - snp-02-004-104: 主が `join_submips` で根の並列サブ MIP を待ち、サブ MIP のダイビングの LP が `PriceMat::repartition`・`cost_perturb_base` の中。

## 4. 残り

- 双対単体法の中の重い手順 (LU 分解・`PriceMat::repartition`・費用の摂動など) は期限を見ず、大きな問題で 1〜2 秒の超過が残る (上の 4 問)。
  LU 分解の途中で止めると数値の失敗として扱われるおそれがあるので、今回は手を付けていない。
- 根の LP の競争で負けた内点法 + クロスオーバーのスレッド (`megiddo_push` の LU 分解) は期限の後もしばらく回る (join されないので返りは遅らせないが、
  遅い機械では CPU を取り合う)。
- 遅い機械 (裏の処理 8 本) では、主の前処理のうち止めを見ない手順 (前処理の縮約のラウンドなど) が期限を 0.2〜0.4 秒過ぎる。
