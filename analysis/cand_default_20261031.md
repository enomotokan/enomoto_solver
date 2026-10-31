# 時間の上限の超過を直して、既定にする候補 cand を既定にする (2026-10-31)

- 機械: 4 CPU。ソルバーは同時に 1 つだけ動かした。HiGHS (highspy) は 1.15.1。
- 土台: ブランチ `ccr-1beed81b-ex0uhq` の `6bfcc89d` (bench7 の記録、`analysis/halve_submip_20261030.md` の「bench7」)。
- cand = `ENOMOTO_MIP_SUBMIP_HIGHS=-s ENOMOTO_MIP_SUBMIP_PC_INHERIT=1 ENOMOTO_T_MIP_SUBMIP_RELIABLE=0 ENOMOTO_MIP_BG_CUTS_WAIT=1
  ENOMOTO_MIP_SUBMIP_HIGHS_TRIM=r ENOMOTO_T_MIP_RESTART_FIRST_FAC=0.01 ENOMOTO_MIP_BG_CUTS_PROOF_STOP=1` (bench7 で 58 問 2 回 SGM −6.1%・
  解けた数 +2、Mittelmann 186 問 −2.9%・+5)。
- 台本とログは `analysis/cand_default_20261031/`:
  - `watch.sh 問題 上限 回数 接頭辞 [環境変数 ...]`: `ENOMOTO_MIP_LOG=1`・4 スレッドで回し、起動から上限 + 1.5 秒を過ぎても終わらなければ
    1 秒ごとに gdb で全スレッドのスタックを `*.gdb` に取る (`analysis/bgw_overrun_20261031/watch.sh` に環境変数を渡せるようにしたもの)。
  - `ab.sh 回 問題...`: 旧ビルド (`6bfcc89d`) + 旧 cand の環境変数 (`cand.env`) と、新ビルド (環境変数なし) を交互に回す。
  - `cand.env`: 旧 cand の環境変数。`olddef.env`: 新ビルドで旧既定に戻す環境変数 (下の 3 節)。
  - `logs/`: 接頭辞 `c0`・`c1` 旧ビルド + cand (`c1` はサブ MIP の経過も出す `ENOMOTO_MIP_SUBMIP_VERBOSE=1`)、`d0` 旧ビルドの既定、
    `n0`・`ov` 新ビルドの既定 (超過した問題)、`old0`・`new0` は `ab.sh` の 2 つの形、`olddef` 旧ビルドの既定、`newrev` 新ビルド + `olddef.env`。
    `*_summary.txt` は 1 行ずつのまとめ (wall は Python の起動・読み込みを含む)。
- `cargo test --release --lib` は 330 件通過 (`CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-C link-arg=-lpython3.13"`)。

## 1. 時間の上限の超過の原因と直し方

### 1.1 rmatr200-p5 (cand で 91.7 秒): 根の並列サブ MIP が空きを待った分だけ期限を越える

事実 (`c0`・`c1` の rmatr200-p5、旧ビルド + cand で 89.3・89.4 秒に再現):

1. 上限 + 1.5 秒の時点のスタックは、主が `Solver::run` → `join_submips` (根のヒューリスティクスの最後に別スレッドのサブ MIP を待つ)、
   サブ MIP のスレッドが `spawn_submip` → `solve_problem` → `Solver::run` → `randomized_rounding`・`feasibility_pump` の LP。
2. `ENOMOTO_MIP_SUBMIP_VERBOSE=1` のログで、遅れたのは RENS (カットの前の LP) のサブ MIP。根の被約費用・LP face・RENS のサブ MIP が
   同時に走れる 3 本 (`submip_par_limit` = スレッド数 − 1) を埋めていたので、`spawn_submip` が LP face のサブ MIP の終わりを
   **29.3 秒待ってから** 出した。サブ MIP の時間の上限は、`SUBMIP_HIGHS` の `t` で「呼び出し側がサブ MIP を作った時点の残り時間」
   (38.2 秒) のままなので、出した時点 (外側の求解の 45.5 秒、外側の期限は約 53.8 秒) から 37.5 秒走り、外側の 83.1 秒で終わった。
3. 根の被約費用 (39 秒の上限)・RENS (38 秒) のサブ MIP は、出した時点の残り時間で上限が決まるので期限内に終わっている。
4. def (`t` なし) ではサブ MIP の上限が残り時間の一部 (`submip_time_frac`) なので、同じ待ちがあっても超えにくい (bench7 の def は 60.1 秒)。

直し方 (`Solver::spawn_submip`、既定で使う。`ENOMOTO_MIP_NO_SUBMIP_SPAWN_CLAMP` で使わない (切り分け用)):
空きを待った後に、サブ MIP の時間の上限を外側の期限までの残りで押さえる。残りがなければサブ MIP を出さない
(ログ `parallel sub-MIP (...): time limit ...s cut to the remaining ...s`・`not started (no time left)`)。
期限内に出せるサブ MIP の上限は変わらない (待たずに出したものは、作った時点の残り時間 ≥ 上限なので押さえない)。

結果 (`n0`・`ov`): rmatr200-p5 は 60.09・60.06 秒。RENS (カットの前の LP) のサブ MIP の上限が 38.17 → 9.41 秒に押さえられ、
最後の locks のサブ MIP は残り 0 秒で出さなかった。見つかった解 (4810、RENS のサブ MIP) は直す前と同じ。

### 1.2 sorrell3 (def・cand とも 61.5 秒、今回の def の 1 回は 117.5 秒): Local-MIP の連鎖の手が時間を見ない

事実 (`d0_sorrell3`、旧ビルドの既定で 117.5 秒):

1. 主は根の LP を解いた後 `join_fj_thread` (FJ・Local-MIP のスレッドに止めの合図を出して終わりを待つ) で 57 秒待った。
   そのスレッドのスタックは 8 回とも `fj_lmip_mix` → `LocalMip::run` → `chain_move` (連鎖の手、`HashMap` の挿入・`gain` の評価)。
2. `LocalMip::run` は止めの合図と時間を 16 手ごとに見るが、連鎖の手 1 回の中では見ない。sorrell3 は前処理のクリーク行の拡張で非零が
   700 万増え (0-1 列 1024 本・行 52801 本)、1 列が数千行に出るので、入れ替えの候補 (集合分割の行の 0 の列) の評価 1 回に数千行を
   `HashMap` に積む。連鎖の手 1 回に数秒かかり、16 手で数十秒になる。
3. FJ (`fj2_search`) の連鎖の手も同じ関数 (`localmip::chain_move`) を使う。

直し方 (`localmip::chain_move`、既定で使う。`ENOMOTO_MIP_NO_CHAIN_ABORT` で使わない (切り分け用)): 呼び出し側から止めの条件
(`abort`: 時間の上限・止めの合図) を受け取り、関数の始め・深さごと・手の評価 32 回ごとに見て、真なら手を返さずに止める。
Local-MIP・FJ とも、それぞれの `t_end`・`stop` を渡す。止めたときは手を返さないだけなので、時間内の動きは変わらない。

結果 (`n0`・`ov`): sorrell3 は 60.60・60.94 秒 (解 −13、直す前の 117.5 秒の回は −11)。

### 1.3 ほかの超過 (再現せず)

旧ビルドで 1 回ずつ回した unitcal_7 (cand、60.13 秒)・neos-4763324-toguru (cand、60.15 秒)・rail507 (cand、60.64 秒)・
cryptanalysiskb128n5obj16 (def、60.05 秒) は、どれも上限 + 1 秒以内で、bench7 の超過 (64.8・63.0・61.3・66.5〜68.0 秒) は再現しなかった。
原因は確かめていない (cryptanalysiskb128n5obj16 は前処理のプロービングの含意が 180〜220 万と多いが、クリーク行の拡張で足す非零は
3788 で、sorrell3 の形とは違う)。新ビルドの既定でも 1 回ずつ回し、全部上限 + 1 秒以内 (下の表)。

| 問題 | bench7 (cand / def) | 旧ビルドで再現 (形) | 新ビルドの既定 |
|---|---|---|---|
| rmatr200-p5 | 91.7 / 60.1 | 89.27・89.37 (cand) | 60.09・60.06 |
| sorrell3 | 61.5 / 61.5 | 117.55 (def) | 60.60・60.94 |
| unitcal_7 | 64.8 / − | 60.13 (cand) | 60.13 |
| neos-4763324-toguru | 63.0 / − | 60.15 (cand) | 60.33 |
| rail507 | 61.3 / − | 60.64 (cand) | 60.54 |
| cryptanalysiskb128n5obj16 | 68.0 / 66.5 | 60.05 (def) | 60.05 |

(秒は `run_miplib_benchmark.py` の `time`、前処理を含む。)

## 2. 既定にした内容と戻し方

cand の 7 つの設定を既定にした。既存の tunable は既定値を変え、有無で見ていた `env_str!` は `env_def!` (付けなければ既定の値、
`0` か `off` で切る) にした (2026-10-29・30 の既定化と同じ形)。

| 設定 | 新しい既定 | 元の振る舞いに戻す |
|---|---|---|
| `ENOMOTO_MIP_SUBMIP_HIGHS` | `-s` (`p c g t i h f`、`s`・`q` なし) | `ENOMOTO_MIP_SUBMIP_HIGHS=0` |
| `ENOMOTO_MIP_SUBMIP_PC_INHERIT` | 使う | `ENOMOTO_MIP_SUBMIP_PC_INHERIT=0` |
| `ENOMOTO_T_MIP_SUBMIP_RELIABLE` | 0 | `ENOMOTO_T_MIP_SUBMIP_RELIABLE=-1` |
| `ENOMOTO_MIP_BG_CUTS_WAIT` | 使う | `ENOMOTO_MIP_BG_CUTS_WAIT=0` |
| `ENOMOTO_MIP_SUBMIP_HIGHS_TRIM` | `r` | `ENOMOTO_MIP_SUBMIP_HIGHS_TRIM=0` |
| `ENOMOTO_T_MIP_RESTART_FIRST_FAC` | 0.01 | `ENOMOTO_T_MIP_RESTART_FIRST_FAC=0` |
| `ENOMOTO_MIP_BG_CUTS_PROOF_STOP` | 使う | `ENOMOTO_MIP_BG_CUTS_PROOF_STOP=0` |

- 7 つ全部を戻す並びは `analysis/cand_default_20261031/olddef.env`。
- `ENOMOTO_MIP_SUBMIP_HIGHS` は以前 `0` も「`q` 以外全部」として扱っていたが、今は `0`・`off` で使わない。全部は今まで通り `1`。
- コメント・説明も直した:
  - 各機能の「試作、既定では使わない」を「既定で使う。`0` で止める」などに直した (`mod.rs` の `submip_highs`、`heuristics.rs` の
    `hs_drop`・サブ MIP の pseudocost、`solver.rs` の `bg_cuts_wait_on`・`bg_proved`・`SUBMIP_PC`・強分岐の閾値)。
  - `maybe_restart`・`proto_restart_ok`: 「HiGHS と同じく、初回は固定された整数列が 1 本でもあれば」を直した。HiGHS も初回は 1 本でも
    再スタートするが、割合 (`percentageInactiveIntegers`) は前処理の後の模型の (自由な) 整数列に対するもので、クリーク表の代入も数える。
    こちらは分母に根の始めに固定済みの列も入る全部の整数列 (`p.nint`) で、初回 1%・2 回目以降 2.5% (`restart_thr_20261031.md` の 1 節)。
- `analysis/halve_submip_20261030/run.py` の `"def"` (環境変数なし) は、これからは新しい既定 (= `"cand"`) を指す。
  それより前の記録の def は旧既定。新しいビルドで旧既定を回す `"olddef"` (7 つを戻す環境変数) を足した。
- `docs/improvement_history.md` に既定化の項を足した。

## 3. 確かめ

### 3.1 新ビルドの既定 = 旧ビルド + cand (`ab.sh`、4 スレッド・60 秒、1 回ずつ)

| 問題 | 旧ビルド + cand (状態 / 目的値 / ノード / 秒) | 新ビルド・環境変数なし |
|---|---|---|
| mc11 | 最適 / 11689 / 1411 / 28.3 | 最適 / 11689 / 1259 / 25.3 |
| physiciansched6-2 | 最適 / 49324 / 0 / 21.8 | 最適 / 49324 / 0 / 19.6 |
| roll3000 | 最適 / 12890 / 1974 / 39.8 | 最適 / 12890 / 1448 / 38.0 |
| h80x6320d | 最適 / 6382.09905 / 0 / 20.0 | 最適 / 6382.09905 / 0 / 19.8 |
| binkar10_1 | 最適 / 6742.200024 / 1454 / 6.2 | 最適 / 6742.200024 / 1620 / 6.1 |

- ログの決定的な部分 (根のヒューリスティクスの並び・並列サブ MIP の起動・再スタートの判定 (`restart check`、初回の閾値が
  整数列の 1% (mc11 15.2・roll3000 7.4・h80x6320d 63.2・binkar10_1 1.7)) は 5 問とも同じ。
- 違うのは裏のカットのラウンド数 (physiciansched6-2 5 → 4、roll3000 69 → 70) とノード数で、どちらも裏のスレッドとの時間の揺れ。
  physiciansched6-2・h80x6320d は旧既定では解けにくい問題で、両方とも根で解けた (cand の得の形のまま)。

### 3.2 戻し方 (新ビルド + `olddef.env` = 旧ビルドの既定)

| 問題 | 旧ビルドの既定 | 新ビルド + `olddef.env` |
|---|---|---|
| binkar10_1 | 最適 / 1636 ノード / 5.6 秒 | 最適 / 1529 ノード / 5.9 秒 |
| h80x6320d | 最適 / 261 ノード / 44.1 秒 | 最適 / 199 ノード / 40.4 秒 |

- binkar10_1 は根のヒューリスティクス・再スタートの判定まで同じ。h80x6320d は再スタートの判定の新たな固定の数が少しずれる
  (4665 / 4667、66 / 27) ほかは同じで、旧既定の形 (初回の閾値 0 (`threshold 0.0`) で再スタート 2 回の後に木で 200〜260 ノード、40 秒台) になる。

### 3.3 超過した問題

1 節の表のとおり、新ビルドの既定で 6 問とも上限 + 1 秒以内 (最大 60.94 秒)。

## 4. 残り

- unitcal_7・neos-4763324-toguru・rail507・cryptanalysiskb128n5obj16 の bench7 の超過は再現せず、原因は分かっていない。
- 遅くなる 3 問 (n5-3・neos-1582420・neos-2978193-inde) は既定にした後の課題 (bench7 の判断のとおり)。
- 直し方 1.1 は、根の並列サブ MIP を出すのが遅れたとき、その上限を残りで押さえるだけ。サブ MIP 自身が自分の期限を越える
  (根のヒューリスティクス 1 つが期限を見ずに長く走る) 場合は、今回は見ていない (rmatr200-p5 の他のサブ MIP は期限内に終わった)。
