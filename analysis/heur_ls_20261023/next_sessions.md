# 次のセッションへの依頼文 (2026-10-23)

8 本の依頼文があり、1 本ずつ別のセッションに渡す。各依頼文は次の 2 つをつないだもの。
- 「共通の前置き」
- 「依頼 A〜H」のどれか 1 つ

共通の前置きの `<話題>` は、依頼ごとに次の名前に置き換える。

| 依頼 | `<話題>` |
|---|---|
| A | overrun |
| B | timtab1 |
| C | bppc408 |
| D | physiciansched |
| E | neos960392 |
| F | mzzv42z |
| G | roll3000 |
| H | proofvar |

---

## 共通の前置き (どの依頼にも付ける)

あなたは enomoto_solver (Rust 製の MIP ソルバー) の調査・実装セッションです。
報告・メモ・コミットメッセージは日本語で書いてください。メモは事実と推測を分けて書いてください。

### 背景 (読むこと)

- 土台はブランチ `inv/heur-ls-20261023` (05deed7 以降)。
- 経緯と計測の結果は `analysis/heur_ls_20261023.md` (特に追記 4〜8)。
- 試作の切り替え:
  - `ENOMOTO_MIP_FJ_LMIP_MIX=1`: FJ のスレッドで FJ と Local-MIP + 連鎖の手に時間を分け合う (`src/mip/heuristics.rs` の `fj_lmip_mix`、`src/mip/localmip.rs`)。
  - `ENOMOTO_MIP_SYM_ORIG_CUT=1`: 対称性の行への並べ替えに失敗した解を、値だけ打ち切り値に使う (`src/mip/solver.rs` の `orig_best`)。
- この 2 つを合わせた形を **mixoc** と呼ぶ。既定にする候補。
- 計測の表 (4 スレッド・60 秒、40 問 + 16 問、2 組):
  - `analysis/heur_ls_20261023/table_bench_mix6.md` (ba7f18b: newdef6・mix6・mixoc6)
  - `table_bench_mix5.md`・`table_bench_mix4.md`
- 各回の生の結果はブランチ `inv/heur-ls-bench-20261023`・`-s2`〜`-s8` の `benchmarks/*_ba7f18b_*` と `*_9e34b8e_*` (JSON)。
- HiGHS の値 (状態・目的値・時間) も同じ JSON の `highs` に入っている (HiGHS 1.15.1、4 スレッド・60 秒、mip_rel_gap 0)。

### 作業の決まり

- 作業はブランチ `inv/<話題>-20261023` (inv/heur-ls-20261023 の最新から作る) だけで行い、ここにだけ push する。develop や inv/heur-ls-20261023 には push しない。
- ビルドとインストール:
  - `pip install highspy==1.15.1 maturin scipy`
  - `maturin build --release -o dist`
  - `pip install --no-deps --force-reinstall dist/*.whl`
- テスト: `L=$(python3 -c "import sysconfig;print(sysconfig.get_config_var('LIBDIR'))"); V=$(python3 -c "import sysconfig;print(sysconfig.get_config_var('LDVERSION'))"); RUSTFLAGS="-L $L -lpython$V" LD_LIBRARY_PATH=$L cargo test --release --lib`。これを通すこと。
- MPS (コミットしない): `git fetch origin inv/mps-cache-20261021 && mkdir -p .miplib_cache && for f in $(git ls-tree --name-only origin/inv/mps-cache-20261021 mps/); do git show origin/inv/mps-cache-20261021:$f | gunzip > .miplib_cache/$(basename $f .gz); done`
- 1 問の実行: `analysis/heur_ls_20261023/run_cfg.sh <問題> <秒> <名前> [ENV=VAL ...]`
  - ログは `analysis/heur_ls_20261023/logs/<問題>_<名前>_<n>.log`。`ENOMOTO_MIP_LOG=1` は付く。
  - 最後の行の JSON に、状態・目的値・時間・最初の解。
- 診断の切り替えの例:
  - `ENOMOTO_MIP_FJ_DEBUG=1`
  - `ENOMOTO_MIP_LOCALMIP_DEBUG=1`
  - `ENOMOTO_MIP_TREE_DIAG=1`
  - `ENOMOTO_MIP_NO_SYMMETRY=1`
  - ほかは `src/` を `env_str!` で探す。
- 多くの問題で測るときは `analysis/heur_ls_20261023/bench_mix.sh <名前> <回> [ENV=VAL ...]` を使う。
  - `ENOMOTO_BENCH_ONLY="問題 ..."` で問題を絞る。`ENOMOTO_BENCH_SHA=<ビルドしたコミット>` を付ける。
  - 結果は `benchmarks/`、集計は `analysis/heur_ls_20261023/agg_bench.py`。
- 長い計測は Bash の run_in_background (timeout 7200000) で動かす。
  - sleep を連ねない。setsid・nohup で切り離さない。
  - ソルバーは同時に 1 つ。計測中にビルドしない。
- 比べ方:
  - 4 スレッド・60 秒。
  - 同じ機械で、既定 (何も付けない) と mixoc (`ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1`) を、少なくとも 2 回ずつ。
  - 1 回ごとの揺れが大きい問題が多いので、主張は「2 回とも同じ向き」のものだけにする。
- 新しい手当ては、既定を変えない切り替え (環境変数) で入れる。既定にする判断は親が 40 問 + 16 問の計測で行う。
  - 自分で 40 問 + 16 問を回す必要はない。回すなら bench_mix.sh で 1 組だけ。
- 目的値の不一致 (HiGHS の最適値と違う「最適」) が出たら、最優先で報告する。

### 終わったら

- メモ `analysis/<話題>_20261023.md` を書く。
- メモ・台本・ログをブランチに push する。
- 親セッションがあれば send_message (to: @parent) で要点を 5 行以内で送り、同じ内容を最後の返答にする。要点は次の 4 つ。
  - 原因
  - 手当て (切り替えの名前)
  - 効果 (2 回ずつの数)
  - 既定にする候補か

---

## 依頼 A: 解があるときの時間の上限の超過 (`<話題>` = overrun)

### 事実

- mix6・mixoc6 で、解がある回だけ 62.4〜62.8 秒かかる (時間の上限は 60 秒)。
  - physiciansched6-2: mix6・mixoc6
  - uccase12: mixoc6 の 1 回
- 解のない回は 60.3 秒。
- mzzv42z は既定でも 62 秒の回がある。
- 既に直したもの:
  - Local-MIP は止める合図を 16 手ごとに見る。
  - 混ぜる形の FJ2 は、止める合図の後に LP 修復をしない (`SKIP_REPAIR_ON_STOP`)。

### やること

1. 時間の上限を超える回を再現し、終わり際の時刻をログで追う (`ENOMOTO_MIP_LOG=1` の各行の時刻)。
   - 疑い: 終わり際の暫定解の受け付けの LP 解き直し (`resolve_continuous`)、oneopt、`join_fj_thread` で待つ時間、`finish` の中の処理、Python 側の解の戻し。
2. どこで何秒使っているかを示す。
3. 時間を見て打ち切る手当てを入れ、超過が 0.5 秒以内に収まるかを確かめる。対象は physiciansched6-2・uccase12・mzzv42z。
   - 既定と mixoc の両方で、2 回ずつ。
4. 主ギャップや最適の証明を悪くしていないか (同じ 3 問) も確かめる。

---

## 依頼 B: timtab1 の後退と HiGHS との差 (`<話題>` = timtab1)

### 事実 (ba7f18b、4 スレッド・60 秒)

- 既定は 2 回とも 60 秒以内に最適 764772 を証明。
- mixoc6 は 776667・774858 で時間切れ。mix6 は 764772 (証明なし)・768807。
- HiGHS は 59.9 秒で最適。
- timtab1 は対称性が強い。196 列に lex-leader の行が 170 本、オービトープが 1 つある。

### やること

1. mixoc で解き切れない原因を突き止める。
   - 観点: 最初の暫定解の質と時刻。木の進み方 (ノード数・下界の上がり方)。
   - 観点: `ENOMOTO_MIP_SYM_ORIG_CUT` の打ち切り値が効いた回数 (ログの "kept as a cutoff value")。
   - 観点: 根の時間の配分 (FJ のスレッドとカット)。
   - 既定と mixoc を同じ機械で 3 回ずつ並べる。
2. HiGHS が 60 秒で証明できるのに、ENOMOTO が既定でも余裕がない理由を見る (根の下界・ノード数を HiGHS のログと比べる)。
3. 手当てを切り替えで入れ、mixoc で既定と同じ程度に解けるようにする。2 回ずつ確かめる。

---

## 依頼 C: bppc4-08 の後退 (`<話題>` = bppc408)

### 事実

- 最終の値は、既定が 4 回の計測でずっと 54 (最適は 53)。
- 混ぜる形は 55 の回が多い。
  - mix3・mix4・mix5・mixoc6 は 2 組とも 55。
  - mix6 は 54 / 55。
- HiGHS も 60 秒では証明できない。

### やること

1. 既定で 54 を出しているのはどのヒューリスティクスか、ログの "new incumbent" の出どころで調べる。混ぜる形でそれが出なくなる理由も調べる。
   - 疑い: 最初の暫定解が早く出ることで、暫定解なし用のヒューリスティクスや Crossover の相手が変わる。
   - 疑い: FJ の解が Local-MIP に吸われる。
2. 原因に合う手当てを切り替えで入れ、mixoc で 54 (できれば 53) が 2 回とも出るかを確かめる。
3. ヒューリスティクス役のスレッド (ALNS) が bppc4-08 でどう働いているかも見る。この問題はヒューリスティクス役の効果が出た問題として記録がある (`solver.rs` の `heur_role_count` の注記)。

---

## 依頼 D: physiciansched6-2 の HiGHS との差 (`<話題>` = physiciansched)

### 事実

- HiGHS は 10.8 秒で最適 49324。
- ENOMOTO の既定は、この機械では解なしのことが多い。
- mixoc6 は 50024 / 49824 (1.0〜1.4%)、mix6 は解なしか 51125。
- ENOMOTO の根の下界は 48587。HiGHS は前処理とカットで根の下界を最適値 49324 まで上げ、根のサブ MIP で解を出す (`analysis/physiciansched6-2_20261020.md`)。
- 対称性: 11 列の 1 軌道に lex-leader の行が 10 本。

### やること

1. 根の下界の差 (48587 と 49324) の出どころを調べる。
   - HiGHS の前処理後の問題とカットの種類を比べる。
   - ENOMOTO で効いていないカット (種類・本数・下界への寄与) を特定する。
   - 前の調査 `analysis/physiciansched6-2_20261020.md` を先に読む。
2. 解が出ない理由を調べる。FJ が種しだいで、Local-MIP は違反 1 行で止まる回がある。
   - 違反行の形を調べる。手がかりは `ENOMOTO_MIP_FJ_DUMP` と `analysis/heur_ls_20261023/viol_rows.py`・`nearest.py`。
3. 下界か解のどちらかで HiGHS に近づく手当てを 1 つ実装する。既定と mixoc で 2 回ずつ確かめる。
4. 時間の上限の超過 (依頼 A) はこの依頼では直さなくてよい。

---

## 依頼 E: neos-960392 の時間切れ (`<話題>` = neos960392)

### 事実

- 既定は多くの回で 28 秒前後に最適 −238 を出す。ただし既定でも −232・−237 で時間切れの回がある (ノード 1)。
- mix・mixoc は時間切れの回が多い (ba7f18b では 4 回とも)。
- 最適を出しているのは、FJ の質の悪い解のときに回る「暫定解なし用の RENS」(`fj_poor_bg_rens`)。
- 最初に −237 のような良い解が出ると「質の悪い解」と判定されず、この RENS が回らないとみている (推測)。
- オービトープ (493×7) がある。

### やること

1. 時間切れの回を再現し、根で何に時間を使い、なぜ −238 に届かないかをログで示す。
   - 観点: RENS が回ったか。find thread が回ったか。根の LP・カットの時間。ノード 1 のまま止まる理由。
2. 「暫定解が FJ のスレッドの解だけのうちは、質に関係なく裏の RENS を回す」などの手当てを切り替えで試す。
   - 既定と mixoc で 3 回ずつ、時間切れの回数を比べる。
3. HiGHS は 13.9 秒で最適。HiGHS の解き方 (ログ) と比べる。

---

## 依頼 F: mzzv42z の HiGHS との差 (`<話題>` = mzzv42z)

### 事実

- HiGHS は 50.1 秒で最適 −20540。
- ENOMOTO は既定・mix・mixoc とも時間切れで、最終の値は −19500〜−20490 (0.2〜5% の差)。
- 下界は −20730〜−20810。
- ノードはほとんど 0 (根で 60 秒を使う)。再スタートがある。
- 対称性: 8 列に lex-leader の行が 4 本。

### やること

1. 根で 60 秒を何に使っているか (LP・カット・ヒューリスティクス・再スタート) を時刻つきで示し、HiGHS の進み方と比べる。
2. 下界と解のどちらが足りないかを分けて示す。
   - 下界: HiGHS は根のカット後にどこまで上がるか。
   - 解: HiGHS は −20540 をいつ何で見つけるか。
3. 一番効きそうな手当てを 1 つ実装し、既定と mixoc で 2 回ずつ確かめる。

---

## 依頼 G: roll3000 の証明の差 (`<話題>` = roll3000)

### 事実

- 最適値 12890 には、既定・mix・mixoc とも届くことが多い。
- 60 秒で最適を証明できない回が多い。下界は 12570〜12860。
- HiGHS は 46.3 秒で証明。
- 対称性: 56 列の 1 軌道に lex-leader の行が 55 本。

### やること

1. 下界が上がらない理由を調べる。
   - HiGHS と比べる: 根の下界、カット、木のノード数・処理速度。
   - `ENOMOTO_MIP_NO_SYMMETRY=1` で lex-leader の行を外したときの違いも見る。
2. 手当てを 1 つ実装し、既定と mixoc で 2 回ずつ、60 秒での証明の回数と下界を比べる。

---

## 依頼 H: 証明の揺れ (h80x6320d・neos-2978193-inde・beasleyC3) (`<話題>` = proofvar)

### 事実

- 3 問とも、既定と mixoc で、60 秒以内に最適を証明できる回と、最適値に届いたまま時間切れの回が混じる。
  - h80x6320d: mixoc6 は 6386.1 で時間切れと 53 秒、既定は 54 秒と時間切れ。
  - neos-2978193-inde: 37 秒と最適値で時間切れ。
  - beasleyC3: 18〜57 秒と 754 で時間切れ。
- HiGHS はそれぞれ 17.4・23.9・25.8 秒で証明。
- h80x6320d では、mix で一度だけ誤った最適 (6387.76、正しくは 6382.1) が出た。最適と報告する直前の守りを入れた (`analysis/heur_ls_20261023.md` 追記 5)。

### やること

1. 3 問それぞれ、既定で 5 回ずつ回して時間の分布を出す。遅い回と速い回の違いを示す。
   - 観点: 最適解を見つけた時刻と出どころ。根の下界。ノード数。木のスレッドの働き方。
2. 証明が遅い回に共通する原因を突き止め、手当てを 1 つ実装する。5 回ずつで分布が良くなるかを確かめる。
3. h80x6320d の誤った最適の再発がないかを見る (ログの "MIP: WARNING" と目的値)。
