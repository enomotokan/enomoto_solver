# 並列の木の探索の終了判定の競合 (mc11 の誤った最適値 11712 の原因の候補) (2026-10-21)

- 発端: analysis/mc11_wrongopt_20261021.md。4 スレッド・60 秒の計測 (inv/subx-a-20261021 6ffe911、設定 S =
  `ENOMOTO_MIP_SUBMIP_CUTS=1 ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS=25 ENOMOTO_T_MIP_SUBMIP_CUT_STALL=10`) で 1 回だけ mc11 が
  34.24 秒・status optimal・目的値 11712 を返した (最適値 11689)。前の調査は約 450 回で再現せず、サブ MIP からの漏れは
  見つからなかった。
- 環境: 専用コンテナ 4 CPU。土台はこのブランチ (eb7f7af)。台本・要約・小さいログは `runs/partree/`。

## 0. 要約

- **事実 (コード):** ヒューリスティクス役 (4 スレッドでは tid 3) から木の探索に戻った直後の `pool_balance` が、
  自分が「待っている側」に数えられたまま (`pool_busy` が偽、`busy` を増やさずに) 共有の待ち行列からノードを取る。この
  スレッドが部分木を探索している間に、ほかの 3 スレッドが同時に待ち状態になり共有の待ち行列が空になると、`busy == 0` で
  探索が終わったと判定され、主は従を止めて、その部分木を探索しないまま `finish(Optimal, 暫定解)` を返す
  (`run()` の終わりは `join_workers()` が返す「止めた従・共有の待ち行列に残ったノードの下界」を捨てていた)。§1。
- **事実 (再現):** mc11 を問題の設定 S で、仕掛けなしの 6 回のうち 1 回 (S_old 1 回目) で、tid 3 が役から戻った
  直後にこの形でノードを取り、60 秒の時間切れまで約 34 秒間、待っている側に数えられたまま 275 ノードを処理し、終わりに
  233 ノードを持っていた。仕掛け (役を何度も出入りさせ、取る条件を緩め、待っている側のままのノードの処理を 50 ms
  遅らせる) を入れると、markshare_4_0 で 30 回中 17 回、探索の終わりの判定の時点でこのスレッドが打ち切り値未満の下界の
  ノード (数個〜十数個、下界 0) を持ったまま「最適」と報告した (部分木を失った。値 1 は偶然正しい)。§3。
- **未確認 (推測):** 誤った値 (mc11 の 11712) そのものは再現していない。mc11 の誤った回 (34 秒・3442 ノード・下界が
  一気に 11712) は、この経路で最良の下界の部分木を失った形とよく合うが、その回のログ (`ENOMOTO_MIP_LOG`) が残って
  いないので、この経路だったとは言い切れない。
- **修正 (既定の動作):** `pool_balance` で待っている側のままノードを取るときは `pool_acquire` と同じく錠の中で
  `busy += 1`・`idle -= 1`・`pool_busy = true` にする。安全網として、主が木の探索を終えるときに止めた従・共有の
  待ち行列に打ち切り値未満のノードが残っていれば optimal と報告せず `NotSolved` を返し警告する。修正後、同じ仕掛けで
  markshare_4_0 30 回・pk1 5 回・gen-ip054 3 回・neos-911970 3 回で 0 回 (取る出来事は markshare の 30 回とも起きたが
  数え直された)、仕掛けなしで 138 回 (mc11 38 回・beasleyC3 20 回・rout 55 回・neos-911970 20 回、CPU の取り合い 33 回を
  含む) で optimal の値は全部正しく、警告・部分木の喪失は 0。`cargo test --release --lib` は 308 通過。
- **FC_SUBMIP を既定に戻してよいか (見込み):** この経路は FC_SUBMIP・SUBMIP_CUTS に固有ではなく (ヒューリスティクス役が
  木に戻る全ての問題で起きうる)、サブ MIP の設定は時間の使い方を変えて当たりやすさを変えただけ、という説明が最も自然。
  修正後は FC_SUBMIP を既定に戻すのを妨げる材料はない。ただし 11712 の回がこの経路だったことは確かめられていないので、
  計測では `MIP: WARNING` (特に `remained when the tree search threads were stopped`) を見張り続けるのがよい。

## 1. 洗い出した経路 (コードを読んだ結果)

`WorkPool` の終了の判定は「待っていない (busy) スレッドの数 `busy` が 0 で、共有の待ち行列が空」(`pool_acquire`・
`pool_release`・`heur_role_run`)。各スレッドは `pool_busy` で自分が busy に数えられているかを持つ。ノード (手元の待ち行列・
潜っている子) を持つスレッドは必ず busy に数えられている、というのが正しさの前提。これを崩す経路を探した。

| 経路 | 見たこと | 判定 |
|---|---|---|
| `pool_acquire` (共有の待ち行列から取る) | 錠の中で取り、`pool_busy` が偽なら `busy += 1`。取れなければ `busy -= 1`、0 なら `done` | 正しい |
| `pool_donate*` (渡す) | 渡すのはノードを持つ (busy の) スレッドだけ。渡した後も自分は busy のまま | 正しい |
| `heur_role_run` (ヒューリスティクス役に就く) | 手元のノードを全部共有の待ち行列に出し、錠の中で `busy -= 1`・`pool_busy = false` | 正しい |
| **ヒューリスティクス役から木に戻った直後の `pool_balance`** | 役から `None` で戻ると `continue` でループの頭に戻る。ノード数 `nodes` は役の間に増えないので、役に就いた回と同じく `nodes % 32 == 0` なら `pool_balance` が走る。その「取る」側 (`pool_min < me - thr`、`me` は役に就く前の古い `cur_lb`) は **`pool_busy` を見ずに** 共有の待ち行列からノードを取り、手元の待ち行列に入れる。`busy` は増えない | **誤り。** このスレッドはノードを持ったまま待っている側に数えられる。以後、手元の待ち行列が空になって `pool_acquire` に入るまで (その部分木を探索し終えるまで) この状態が続く |
| 上の状態での終了の判定 | ほかの 3 スレッドが同時に待ち状態になり共有の待ち行列が空なら `busy == 0` で `done`。主はループを抜け、`join_workers` で止める合図を立てる。止められたスレッドは `finish_limit` で残りのノードを共有の待ち行列に返し、`join_workers` はその下界を返すが、**`run()` の終わりは `join_workers()` の戻り値を捨てて** `finish(Optimal, 暫定解)` を返す | 部分木を失ったまま optimal を報告する |
| `race_step` (racing、既定では使わない) | 負けたスレッドは木を捨てるが busy のまま `pool_acquire` に行く | 正しい (既定外) |
| 解の受け渡し (`try_incumbent` → `inbox` → `publish`) | 従は解を `inbox` に置いてから共有の最良値を下げる。主は `join_workers` の後に必ず `take_inbox` する。`inbox_n` の読みと錠の順の入れ替わりでも、最後の呼び出しで全部受け取る | 正しい |
| 目的値の空間 | 従の問題は主の `p` の写し (`snap_p`)。再スタートは従を始める前だけで、`from_rows` は同じ列・同じ定数項。共有の `Portfolio` は既定では `start_workers` で作る (ポートフォリオ・二分探索は既定外) | ずれなし |
| 共有の大域的な境界・学習 (`sync_global_bounds`・`learned`) | どれも「共有の最良値未満の解」に対して成り立つ締め付けで、最良値は実在の解から作る | 正しい (この調査の範囲では) |
| 待ち行列の一括の刈り込み (`queue.prune`) | 打ち切り値以上の下界のノードを捨てるだけ。下界が正しければ正しい | 検査を入れた (§2) |

## 2. 入れた検査 (診断用)

- `ENOMOTO_MIP_TREE_DIAG`: (1) 待っている側に数えられたまま `pool_balance` でノードを取った (`took a node ... while counted
  as waiting`)、そのままノードを処理した (`processes a node ... while counted as waiting`)、(2) 従の終わりの状態・目的値・
  下界・ノード数、(3) 主が木の探索を終えるときの、主の暫定解・共有の最良値・打ち切り値・止めた従と共有の待ち行列に残った
  ノードの下界、(4) 止められた従が持っていたノードの数と下界、を表示する。
- 常に出す警告 (安価なので環境変数で隠さない): 木の探索の終わりに共有の最良値が主の暫定解より良い
  (`the shared best value ... is better than the main thread's incumbent`)。止めた従・共有の待ち行列に打ち切り値未満の
  ノードが残っていた (`nodes with lower bound ... remained ...; not reporting optimality`)。後者のときは optimal と報告せず
  `NotSolved` (下界は残ったノードの下界) で返す (安全網。`ENOMOTO_MIP_TREE_NO_END_CHECK` で以前と同じく確かめない)。
- デバッグ解 (`ENOMOTO_MIP_DEBUG_SOL`) の検査を広げた: `queue.prune` による一括の刈り込み (`Solver::prune_queue`、3 か所)、
  止められた従が返したノード、木の探索の終わりの打ち切り値とデバッグ解の目的値の比較。
- 競合を起こしやすくする仕掛け: `ENOMOTO_T_MIP_TREE_DIAG_WAITING_SLEEP_MS` (待っている側に数えられたままノードを処理する
  スレッドを、ノードごとにこの ms だけ眠らせる)。既存の調整値 `ENOMOTO_T_MIP_TREE_BALANCE_EVERY=1`・
  `ENOMOTO_T_MIP_TREE_BALANCE_Q=0` (役から戻るたびに `pool_balance` が走り、取る条件を緩める) と組む。
- 修正を外す診断用の切り替え: `ENOMOTO_MIP_TREE_NO_BALANCE_BUSY_FIX` (以前と同じく busy に戻さない)。

## 3. 再現の条件と回数

台本は `runs/partree/` (`run1.py`・`loop.sh`・`batch1〜5.sh`)、要約は `*.summary` (列: 回、結果、`warn` は
`took a node`・警告・デバッグ解の記録の行数)、抜き出したログは `keylogs/`。全て 4 スレッド・60 秒、1 つずつ (CPU の取り合いの
回だけ 2 つ重ねた)。「旧」は `ENOMOTO_MIP_TREE_NO_BALANCE_BUSY_FIX=1 ENOMOTO_MIP_TREE_NO_END_CHECK=1` (修正前と同じ動作)。
仕掛け FC = `ENOMOTO_T_MIP_TREE_BALANCE_EVERY=1 ENOMOTO_T_MIP_TREE_BALANCE_Q=0 ENOMOTO_T_MIP_TREE_DIAG_WAITING_SLEEP_MS=50
ENOMOTO_MIP_HEUR_ROLE_TRIGGER=off ENOMOTO_MIP_HEUR_ROLE_REENTER=1 ENOMOTO_T_MIP_HEUR_ROLE_PATIENCE=1
ENOMOTO_T_MIP_HEUR_ROLE_COOLDOWN=0.05` (役を何度も出入りさせる)。PV = FC の前 3 つだけ (役の判定は既定)。

| # | 問題 | 条件 | 回数 | 結果 |
|---|---|---|---|---|
| 1 | mc11 | 旧 + S (仕掛けなし) (`S_old`) | 6 | 全部時間切れ (値は正しい範囲)。1 回目で tid 3 が役から戻って 8 回取り、275 ノードを待っている側のまま処理、時間切れで 233 ノードを持っていた (`keylogs/S_old_mc11_1.diag.txt`)。ほかの 5 回は起きず |
| 2 | mc11 | 旧 + S + PV (`Spv_old`) | 6 | 全部時間切れ。4 回で起き (取る 1〜24 回、処理 2〜110 ノード)、2 回目は時間切れで 26 ノードを待っている側のまま持っていた |
| 3 | rout | 旧 + FC (仕掛けの調整中、`prov*_old`) | 計 17 | 全部 optimal 1077.56。取る出来事は 17 回中 8 回 |
| 4 | rout | 旧 + PV (`pv_old`) | 10 | 全部 optimal。出来事 0 (rout では役から戻らない) |
| 5 | gen-ip054・markshare_4_0・pk1・neos-911970・roll3000・beasleyC3 | 旧 + FC (`fc_old`) | 各 3 | 値は全部正しい。markshare_4_0 は 3 回とも、gen-ip054 は 3 回とも (時間切れ)、roll3000 は 1 回、終わりに待っている側のままノードを持っていた。**markshare_4_0 の 3 回は、打ち切り値 1e-6 未満の下界 (0) のノードを持ったまま optimal と報告した** |
| 6 | markshare_4_0 | 旧 + FC | 追加 27 (5 と合わせ 30) | 30 回とも optimal 1 (値は正しい)。**17 回で部分木を失ったまま最適と報告** (`end of tree search ... workers/pool bound 0`) |
| 7 | markshare_4_0 | 修正なし + 安全網あり + FC (`fc_endchk`) | 20 | 13 回 `not_solved` (警告 `remained ...`、下界 0)、7 回 optimal |
| 8 | markshare_4_0 | 修正 + FC (`fc_new`) | 30 | 30 回とも optimal 1。取る出来事は 30 回とも起きたが数え直され、待っている側のままの処理・喪失・警告は 0 |
| 9 | pk1・gen-ip054・neos-911970 | 修正 + FC (`fc_new`) | 5・3・3 | pk1 optimal 11 ×5、gen-ip054 時間切れ (6840.97) ×3、neos-911970 optimal 54.76 ×3。喪失・警告 0 |
| 10 | mc11 | 修正 + S (`newS`) | 20 | 全部時間切れ (暫定解 11689〜11708、下界 11489〜11556) |
| 11 | mc11 | 修正 + S + `ENOMOTO_MIP_FC_SUBMIP=1` (`newSfc`) | 10 | 全部時間切れ |
| 12 | beasleyC3 | 修正 + S / 既定 (`newS`・`newdef`) | 10・10 | optimal 754 が 10 回・5 回、残りは時間切れ |
| 13 | rout | 修正 + S / 既定 | 15・15 | 全部 optimal 1077.56 |
| 14 | neos-911970 | 修正 + S / 既定 | 10・10 | optimal 54.76 が 8 回・10 回、残りは時間切れ |
| 15 | mc11 + rout を同時に (CPU の取り合い) | 修正 + S (`newcpuA`・`newcpuB`) | 8・25 | mc11 全部時間切れ、rout 全部 optimal 1077.56 |

- 10〜15 (計 138 回) で、役から木に戻った回は 62 (`back to tree`)、待っている側のままの処理・喪失・`MIP: WARNING` は 0。
  optimal の値は全部正しい。仕掛けなしでは取る出来事自体がまれ (mc11 の旧 6 回で 1 回) なので、138 回の 0 は修正の
  裏付けとしては弱く、確かな裏付けは仕掛けを入れた 7・8 (旧 17/30 → 修正 0/30)。
- 誤った値 (最適でない値を optimal) は、旧でも再現しなかった。markshare_4_0 は最適値 1 の解を早く見つけるので、失った
  部分木により良い解がなかった。

## 4. 原因 (事実と推測)

- 事実: §1 の経路で、ノードを持つスレッドが待っている側に数えられ、探索の終わりの判定がそのスレッドの部分木を見落とす。
  旧の markshare_4_0 で、終わりの判定の時点で打ち切り値未満の下界のノードを持つスレッドを 17/30 回観測した。
- 事実: 仕掛けなしの mc11 (設定 S) で、この状態が 34 秒間続いた回がある (6 回中 1 回)。
- 推測: mc11 の 11712 の回は、この状態のスレッドが最良の下界の部分木 (`pool_balance` は共有の待ち行列の下界最小の
  ノードを取る) を持っている間に、ほかの 3 スレッドが同時に待ち状態になって探索が終わった。報告された下界 11712 は
  終わりの時点の全体の下界 (空) と暫定解の小さい方で、部分木の下界は入っていない。

## 5. 修正

- `src/mip/solver.rs` `pool_balance`: 待っている側のまま取るときは錠の中で busy に戻す (`pool_acquire` と同じ)。
  `ENOMOTO_MIP_TREE_NO_BALANCE_BUSY_FIX` (診断用) で以前の動作。
- `run()` の終わり: `join_workers()` の戻り値 (止めた従が返したノード・共有の待ち行列に残ったノードの下界) が打ち切り値
  未満なら、警告を出して `NotSolved` (下界はその値) を返す。`ENOMOTO_MIP_TREE_NO_END_CHECK` (診断用) で以前の動作。
  修正後は起きないはずの状態の安全網。
- 診断用の表示・デバッグ解の検査・遅延の仕掛けは §2。既定の動作は上の 2 つ以外変えていない (診断の表示は環境変数で出す。
  2 つの警告だけは常に出す)。
- 確かめたこと: `cargo test --release --lib` 308 通過 (17 は無視、失敗 0)。目的値: rout 1077.56・beasleyC3 754・
  neos-911970 54.76・pk1 11・markshare_4_0 1 (40 問のうち 3 問 + 2 問) が修正前と同じ。

## 6. 残ること

- 計測で `MIP: WARNING` の行を数える (特に `remained when the tree search threads were stopped` と `the shared best value
  ... is better`)。出たら §1 のほかにまだ見つけていない経路がある。
- 既定の `ENOMOTO_MIP_TREE_DIAG` なしの「all symmetric images gone」は主の手元の待ち行列しか見ないので 4 スレッドでは
  誤報になりうる (前の調査のとおり。直していない)。

