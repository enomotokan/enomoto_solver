# hs_pi0_bgw の neos-960392 が時間の上限を超えた原因 (木の従のサブ MIP の前処理が止まらない、2026-10-31)

- 機械: 4 CPU。ソルバーは同時に 1 つだけ動かした。HiGHS (highspy) は 1.15.1。
- 土台: ブランチ `ccr-1beed81b-ex0uhq` の `262d4694`。
- 設定 (hs_pi0_bgw): `ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_SUBMIP_HIGHS=-s ENOMOTO_MIP_SUBMIP_PC_INHERIT=1
  ENOMOTO_T_MIP_SUBMIP_RELIABLE=0 ENOMOTO_MIP_BG_CUTS_WAIT=1`、`run_miplib_benchmark.py --worker-ours ... --threads 4`。
- 台本とログは `analysis/bgw_overrun_20261031/`:
  - `run.sh 問題 上限 回数 接頭辞`: 1 本ずつ回す。`watch.sh`・`watch2.sh` は同じく回し、起動から上限 + 1.5 秒を過ぎても終わらなければ
    1 秒ごとに `gdb -p ... -batch -ex "thread apply all bt 25"` で全スレッドのスタックを `*.gdb` に取る (`watch.sh` は上限 + 1.5 秒まで必ず待つ版、
    `watch2.sh` は終われば次へ進む版)。`ab.sh 問題 上限 回数` は直した形 (on) と `ENOMOTO_MIP_NO_PRESOLVE_STOP=1` (off、直す前と同じ) を交互に回す。
  - 台本はログを `/tmp/rep/` に書く (`logs/` へは後で写した)。
  - `summary.py` が `logs/` から下の表 (`summary.md`) を作る。接頭辞: `before`・`w` は直す前のビルド、`a` は直した後の最初のビルド
    (切り分けのスイッチを足す前)、`ab_on`・`ab_off` は最終のビルド、`o` は最終のビルドでのほかの問題。
  - `neos-960392_hs_pi0_bgw_0.log` は前に 1 回だけ取ったログ (33 秒で解け、再現しなかった回)。
- `cargo test --release --lib` は 330 件通過 (`CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-C link-arg=-lpython3.13"`)。

## 1. 結論

事実:

1. **はみ出しの原因は裏のカットの待ちではなく、木のヒューリスティクス役の従のスレッドが出したサブ MIP の前処理**。主が探索を終えて
   (最適の証明・時間の上限) 従に止めの合図を出しても、従は ALNS の local branching のサブ MIP の前処理の中の
   **クリーク行の拡張 (`extend_clique_rows`、衝突グラフを約 990 万辺で作る)** とプロビング (`probe`) を回し続け、主は `join_workers` でそれを待つ。
   - 直す前: 上限 55 秒の回 (`w_..._55_0`) が 57.73 秒 (最適、311 ノード)。`tree thread 1 joined 4.784s after the stop signal`。
     上限 + 1.5 秒の時点のスタックは、主が `join_workers` → `join`、従が `heur_role_run` → `alns` → `local_branching` →
     `solve_submip_problem` → `solve_problem_inner` → `extend_clique_rows` → `CliqueGraph::build_with` (`HashSet::insert`)。
   - 直す前と同じ形 (`ab_off_..._60_5`) でもう 1 回: 64.50 秒 (時間切れ、423 ノード)、join 4.41 秒。スタックは同じ道筋で、
     最初は `probe`、その後 `extend_clique_rows`。
   - bench5 の 63.5 秒の回 (最適、318 ノード) もログはないが、木を通って最適で終わる形が同じで、超過の量 (3.5 秒) も合う。
2. `ENOMOTO_MIP_SUBMIP_HIGHS` の `p` (`-s` は `s`・`q` 以外を全部入れるので `p` を含む) で、サブ MIP にも親と同じ前処理
   (クリーク行の拡張を含む) をかける。サブ MIP の前処理は時間の上限・止めの合図をどこも見ていなかった (プロービングの打ち切りは
   固定の秒数と手間、クリーク行の拡張は打ち切りなし)。neos-960392 は local branching が列を全部残すので、サブ MIP の衝突グラフが
   外側と同じ大きさ (外側の根で 2.0〜2.8 秒) になる。
   - 既定 (`p` なし) ではサブ MIP でクリーク行の拡張は回らない (`ENOMOTO_MIP_CLIQUE_EXTEND_SUBMIP` のときだけ)。サブ MIP のプロビングは
     0.5 秒ほどの打ち切りなので、既定の経路での超過は小さいはず (確かめていない)。
3. 裏のカットの待ちは超過した 2 回とも 0.00 秒 (上限 2.98・3.23 秒、`stalled`。根の LP が −238 で裏のカットが上げられない) で、超過には関わっていない。直す前の 22 回で超過は 2 回 (上の 2 つ)、どちらも join の待ちだけで説明できる。
4. 前に取ったログ (`neos-960392_hs_pi0_bgw_0.log`) の `finish done (22.83s)` と `solve returned ... after 32.94s` の 10 秒の差は、
   前者がソルバーの開始 (前処理の後) から、後者が前処理を含めて数えているためで、超過ではない。

## 2. 直し方 (`src/mip/mod.rs`・`src/mip/clique.rs`・`src/mip/solver.rs`)

1. **前処理の止め (`presolve_stop`、既定で使う。`ENOMOTO_MIP_NO_PRESOLVE_STOP` で使わない (切り分け用))**: このスレッドの止めの合図
   (`crate::cancel::halted()`、木の従のスレッドには `with_halt` で付いている) が立った、またはこの求解の時間の上限 (前処理の始め + `time_limit`)
   を過ぎたら真。期限は `solve_problem_inner` がスレッドごとの値 (`PRESOLVE_DEADLINE`) に置き、戻るときに外側の値に戻す (入れ子のサブ MIP も正しく戻る)。
   - `extend_clique_rows`: 衝突グラフを `CliqueGraph::build_with_stop` (含意の組 65536 ごと・行 64 本ごとに見る) で作り、作り切れなければ
     拡張しない (問題をそのまま返す)。行の拡張のループも 64 本ごとに見て止める (そこまでに足した文字はどれも衝突グラフの辺によるので正しい)。
     止めたときは支配された行の除去をしない (同じ行の除去はする)。
   - `probe` (プロビング): 16 候補ごとの打ち切りの条件に足す。
   - どちらも時間の上限・止めの合図を過ぎたときだけ効くので、時間内の動きは変わらない (既定の外側の求解は前処理が時間の上限を過ぎない限り同じ)。
   - 単体テスト `extend_clique_rows_stops_on_halt` を足した (合図が立っていれば何も変えない、なければ拡張する)。
2. **裏のカットの待ちを、待った後の残りが少ないときは始めない** (`ENOMOTO_MIP_BG_CUTS_WAIT` の中だけ): 残り時間 − 待ちの上限が
   時間の上限の `ENOMOTO_T_MIP_BG_CUTS_WAIT_MIN_LEFT` (既定 0.05) 倍を切るなら待たない
   (ログ `MIP: background cuts: not waiting (remaining ..., cap ..., minimum left ...)`)。今回の計測ではどの回でも効いていない
   (待ちの上限が残りの 10% 以下なので、残りが上限の 5.6% を切るまでは効かない)。

## 3. 確かめた結果 (`summary.md`)

| 形 | 回数 | 上限を超えた回 (上限との差) | 上限との差の最大 | 従の join の最大 |
|---|---:|---|---:|---:|
| 直す前 (`before`・`w`、上限 45〜60 秒) | 22 | 1 回 (+2.73 秒) | +2.73 | 4.78 秒 |
| 直す前と同じ (`ab_off`、60 秒) | 6 | 1 回 (+4.50 秒) | +4.50 | 4.41 秒 |
| 直した後 (`a`、55・60 秒) | 12 | 0 | +0.53 | 0.45 秒 |
| 直した後 (`ab_on`、60 秒) | 6 | 0 | +0.50 | 0.42 秒 |
| 直した後 (`o`、h80x6320d・physiciansched6-2・mc11 を 2 回ずつ、60 秒) | 6 | 0 | (23.9〜32.3 秒で解けた) | 0.00 秒 |

- 直した後は、bench5 のほかの 695 回と同じく上限 + 1 秒以内 (最大 +0.53 秒)。0.3〜0.45 秒の join は 6 回に 1〜2 回あり、何に使っているかは見ていない
  (サブ MIP の前処理のうち止めを見ない部分か LP)。
- 解けた回数は、直す前 60 秒で 6 回中 3 回、`ab_off` 6 回中 0 回、`ab_on` 6 回中 2 回、`a` 12 回中 0 回で、揺れが大きく直し方による差は
  読み取れない (最適解 −238 を見つけるのは木の探索のスレッド (`other thread (tree)`・`tree rounding`) か根の RENS で、ヒューリスティクス役の
  サブ MIP ではない。直し方が変えるのは、サブ MIP の前処理が自分の時間の上限か止めの合図を過ぎた後だけ)。
- h80x6320d・physiciansched6-2 は 2 回とも根で解け (23.9〜25.4 秒)、mc11 は 2 回とも木で解けた (29.9・32.3 秒)。

## 4. 残り

- 既定の経路 (`SUBMIP_HIGHS` なし) のサブ MIP の前処理も時間の上限を見るようになったが、既定で超過が起きていたかは測っていない。
- 前処理のほかの手順 (同値の列の探り・LP によるプロービング・対称性の検出など) は止めを見ていない。外側の求解では LP によるプロービングに
  時間の上限 (上限の 5%) があり、サブ MIP では回らないので、今回は足していない。
