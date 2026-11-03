# スレッド数を変えて 58 問を解く (Mittelmann に出す前の確かめ、並列 6 セッションのうちの 5、2026-11-03)

## 0. 要点

- **crash・誤答 (最適値の食い違い) はなし**。1・2・4・8・16 スレッド × 58 問 (+ 4 スレッドの bench20 の 2 回・HiGHS 1.15.1) で、optimal 同士・HiGHS の
  最適値との差は相対 1e-6 以内。時間切れの暫定解・下界が HiGHS の最適値を越える回もなし。
- **時間の上限の超過が 2 件**:
  1. **ns1116954 の 1 スレッドで 70〜71 秒 (10〜11 秒の超過)**。原因は根のヒューリスティクス interior rounding の内点法の**準備**
     (KKT の AMD・METIS の順序付け → 多前線法の記号分解・最初の分解)。止めの合図を反復の頭でしか見ないので、予算 0.85 秒に対し 11〜16 秒かかる。
     4 スレッド以上では AMD と METIS が `rayon::join` で並ぶので短く、根に入る前に早めの FJ が解を見つけるので表に出にくい。
     試作 **`ENOMOTO_MIP_IPM_HEUR_LPGATE`** (根の LP が予算より長い問題では内点法を始めない) を入れた (既定では働かない)。
  2. blp-ar98 の 16 スレッドで 65.6 秒 (1 回)。ログ付きの解き直しでは 60.36 秒で、再現しなかった (4 CPU に 45 本が載る状態での揺らぎと見る)。
- **割り振りは想定どおり**: 早めの FJ : Local-MIP は 4・8・16 スレッドで 3:1 (3+1・6+2・12+4)、2 スレッドは 1+1、1 スレッドは交互。
  ヒューリスティクス役の木のスレッドは 4/8/16 で 1/2/4 本、2 スレッドでは 0 本。1 スレッドでは裏のカット・並列のサブ MIP・木の並列・根の LP の競争は
  動かない (ログに出ない)。停止の合図の後の木のスレッドの合流は 16 スレッドでも 0.10〜0.24 秒 (待ちすぎ・デッドロックはなし)。
- **このコンテナは CPU 4 本** (`nproc` = 4、メモリ 15 GB) なので、8・16 スレッドは 4 CPU を 24〜46 本のスレッドが取り合い、4 スレッドより遅い
  (SGM 21.1 秒 対 4 スレッド 16.8〜17.3 秒)。速さの伸びはここでは測れない。
- メモリはスレッド数に応じて増える (最大 RSS の中央値 1 スレッド 155 MB → 16 スレッド 384 MB、最大は blp-ar98 の 16 スレッドで 4.3 GB)。
- 4 CPU の機械で `--threads 8` を渡すと、木のスレッドは CPU の数 (4) で頭打ち (`src/model.rs` の clamp) なのに、台本が渡す `ENOMOTO_MIP_THREADS=8` は
  頭打ちされず、サブ MIP の同時の本数などは 8 の扱いになる (食い違い。先生の機械は CPU が足りるので関係しないが、記録しておく)。

## 1. 条件

- 機械: クラウドのコンテナ、`nproc` = 4 (`os.cpu_count()` も 4)、メモリ 15 GB。ソルバーは同時に 1 つだけ動かした。highspy 1.15.1。
- 土台: ブランチ `ccr-1beed81b-ex0uhq` の `925975ae` (src の最後の変更は `e0f825d8`。今の既定 = canddet + p)。1・2・8・16 スレッドと
  ログ付きの解き直しはこの wheel。4 スレッド (t4・t4g) は `c9d56d67` (ほかのセッションの試作と `ENOMOTO_MIP_IPM_HEUR_LPGATE` を含むが、どれも既定では働かない)。
- 58 問 (`analysis/cbcimpl_20261029/probs.py`)、60 秒、1 回ずつ。`run_miplib_benchmark.py --worker-ours` の `time` (`m.solve` だけ) で数える。
  解けた = optimal かつ 60 秒以内。SGM はずらし 10 秒、解けない回は 60 秒 (`halve_submip_20261030/summ8.py` と同じ)。
- **8・16 スレッドは 16 CPU の機械を真似た**: このソルバーは `Model.solve(threads=N)` の N を CPU の数で頭打ちにする (`src/model.rs:163`、
  `mip::max_threads()`) ので、4 CPU の機械で 8 を渡しても木は 4 本になる。そこで診断用の `ENOMOTO_MIP_DIAG_HW_THREADS=16` (`src/mip/mod.rs:315`) で
  CPU の数を 16 とみなさせた (t8・t16)。CPU の割り当ては実際には 4 本のまま。
- 外から 0.2 秒ごとに `/proc/<pid>/status` の Threads・VmRSS を読み、最大のスレッド数・最大 RSS とした (Python の 1 本 + highspy の読み込みが作る 3 本を含む)。
- 4 スレッドは bench20 (`analysis/halve_submip_20261030/bench20_s*.jsonl`) の設定 p の回 34・35 (別のコンテナ) と、このコンテナの t4 の 3 つ。
- 先生の計測の条件: 公開ページ (plato.asu.edu/ftp/milp.html) は 503 で読めなかった。二次資料 (MIPLIB 2017 の論文、MindOpt の論文が 2023-12 の先生のページを引く) では
  **12 スレッド・2 時間**、Quad Intel Xeon E5-4657L。リポジトリの以前の記録 (`analysis/app1-1_nu25-pr12_20261019.md`) は「16 スレッド」。今のページでは確かめていない。

## 2. 結果 (58 問)

(表は `threads/summ.txt`、`python3 threads/summ.py bench,gate`)

TABLE_PLACEHOLDER

- crash (optimal・time_limit 以外の状態) は 0。61 秒を超えたのは blp-ar98 の t16 (65.64 秒) だけ (ログ付きの t1 の ns1116954 は別の解き直しで、§3)。
- 1 スレッドで 4 スレッドより大きく遅いのは並列の効き目 (木・裏のカット・並列のサブ MIP・FJ) がなくなる分で、想定どおり。
- 8・16 スレッドは 4 CPU に 24〜46 本が載るので、4 スレッドより遅い問題が多い (neos-860300 19〜26 → 52〜57 秒、rout 5 → 10〜12 秒、
  roll3000・neos5・h80x6320d・swath1 は解けなくなる)。速いのは misc07 (19 → 14 秒) くらい。**CPU が 8・16 本ある機械での速さは、ここからは分からない**。
- ns1116954 (実行可能性問題) は、前処理の中の早めの FJ/Local-MIP (上限 6 秒) が解を見つけるかで決まる: 4 スレッドは FJ 9 回の試行で 4.5 秒で見つかるが、
  1・2 スレッドは 6 秒で FJ 4 回しか回らず見つからず、13 万行の根の LP (1 スレッドで 14 秒) に進んで時間切れ。8 スレッドの本計測は 4 CPU を 6 本の FJ が
  取り合って 6 秒に間に合わず (ログ付きの解き直しでは 6.01 秒で見つかった)。CPU の足りる機械では起きない。

## 3. ns1116954 の 1 スレッドの時間の上限の超過 (70〜71 秒)

- ログ付きで 2 回 (70.06・71.07 秒)。ログなしの 2 回 (60.23・60.19 秒) は超えない (根のヒューリスティクスに入る前に期限が来たかどうかの差)。
- ログ (`threads/logs/ns1116954_t1.log.gz`): 前処理 (早めの FJ 6 秒 + LP による固定の根の LP 13.6 秒など) に約 24 秒。内側の時計で
  カットのループが 34.0 秒 (= 外の約 58 秒) に終わり、そこから interior rounding が **11.5 秒** (`IPM (cost) failed`) かかって、外の 70 秒で返る。
- interior rounding の予算は min(5% × 上限 = 3 秒, 残りの半分) = 約 0.85 秒で、期限を見張るスレッドが 0.85 秒で止めの合図を立てるが、内点法は
  合図を反復の頭 (`src/interior_point/boxed.rs:672`) でしか見ない。gdb で 10 回スタックを取ると (`threads/ipm_sample.sh`、出力 `threads/ipm_sample_ns1116954_t1.bt`、上限 60 秒・1 スレッド、この回は 71.07 秒)、
  1〜4 回目が `faer::sparse::linalg::amd::order_maybe_unsorted` (`kkt::symbolic_with_ordering`)、5 回目が `kkt::metis_ordering`、6〜8 回目が
  `Multifrontal::new`・`prepare`、9〜10 回目が `Multifrontal::bwd_node` (分解)。最初の反復に入る前の準備だけで予算を 10 秒以上越える。
  `symbolic_with_ordering` は AMD と METIS を `rayon::join` で並べるので、rayon の pool が 1 本の 1 スレッドで最も長い。
- 試作 **`ENOMOTO_MIP_IPM_HEUR_LPGATE`** (`src/mip/heur_ipm.rs`): 根の LP (単体法、カットの前) にかかった秒 (`root_lp_secs`) が
  `ENOMOTO_T_MIP_IPM_HEUR_LPGATE_MULT` (既定 1) × 予算を超えたら内点法を始めない。ns1116954 の 1 スレッド・上限 120 秒で
  `interior rounding: skipped (root LP 16.37s > budget 4.25s)` となり、120.04 秒で返る (超過なし)。
- 4 スレッドの 58 問 (t4g) で、門なし (t4) との比べ: GATE_PLACEHOLDER
- 直し方の候補 (試していない): 内点法の準備 (順序付け・記号分解) の中で合図を見る、`interior_point` を別スレッドで走らせて期限で見捨てる、
  KKT の大きさ (行・非零) から準備の時間を見積もって予算に入らなければ始めない。ほかに止めの合図を見ない長い処理は `overrun.md` (別のセッション) が扱っている。

## 4. スレッドの割り振り (`ENOMOTO_MIP_LOG=1`、`threads/logs/`)

| スレッド数 | 早めの FJ / Local-MIP (ns1116954) | 木のスレッド (うちヒューリスティクス役) | 並列のサブ MIP の開始 (blp-ar98) | 裏のカットのログの行 (blp-ar98) | 最大スレッド数 (blp-ar98) |
|---|---|---|---|---|---|
| 1 | 1 本で交互 (FJ 4 回・Local-MIP 432 歩、見つからず) | 1 (並列なし、`tree search with` の行なし) | 0 | 0 | 11 |
| 2 | FJ 1 + Local-MIP 1 (FJ 4 回、見つからず) | 2 (0) | 6 | 16 | 13 |
| 4 | FJ 3 + Local-MIP 1 (FJ 9 回、4.45 秒で発見) | 4 (1) | 6 | 16 | 21 |
| 8 | FJ 6 + Local-MIP 2 (FJ 12 回、6.01 秒で発見) | 8 (2) | 6 | 16 | 29 |
| 16 | FJ 12 + Local-MIP 4 (FJ 12 回、5.50 秒で発見) | 16 (4) | 6 | 16 | 46 |

- FJ と Local-MIP の比は 4 以上で 3:1 (`ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS` の既定 = スレッド数 / 4、最低 1)。2 スレッドで 1:1 なのは設計どおり。
- ヒューリスティクス役は スレッド数 × 0.25 の切り捨て、4 スレッド未満では 0 (`heur_role_count`)。ログの `k of N tree threads take the heuristic role` と一致。
- 1 スレッドで並列の部分は切れている: 裏のカット (`params.threads > 1` が条件)・並列のサブ MIP (`mip_threads() > 1`)・木の並列のどれもログに出ない。
  rayon の pool は `Model.solve` が `threads` 本で作る (`src/model.rs:169`) ので 1 本。1 スレッドで見える 6〜11 本は、Python の 1 本・highspy の読み込みが作る 3 本・
  rayon の 1 本・期限の見張り (内点法の `interior_point` などが短く作る、sleep するだけのスレッド) と、短く作られる FJ/Local-MIP の scope のスレッド。
- 停止の合図の後の合流: blp-ar98 の 16 スレッドで木の 16 本が 0.100〜0.237 秒で合流 (`tree thread k joined ... after the stop signal`)。
- 並列のサブ MIP の同時の本数の上限は `mip_threads() - 1` (`submip_par_limit`、`ENOMOTO_MIP_THREAD_BUDGET` なしの既定)。16 スレッドなら 15 本まで並ぶ。
  blp-ar98 では根で 6 本始まるだけで、16 スレッドでも上限には届かない。
- メモリ: 最大 RSS はスレッド数とともに増える (中央値 155 → 167 → 246 → 303 → 384 MB)。blp-ar98 は 1 スレッド 674 MB、4 スレッド 2.1〜3.1 GB、
  16 スレッド 3.6〜4.3 GB (スレッドごとの問題の写し・LP を持つためと見られる。中身は調べていない)。16 スレッドで 4 GB を越える問題がある (先生の機械のメモリは確かめていない)。

## 5. 4 CPU で CPU の数より多いスレッドを渡したとき (記録だけ)

- `run_miplib_benchmark.py` は `--threads N` を `Model.solve` と環境変数 `ENOMOTO_MIP_THREADS=N` の両方に渡す。`Model.solve` は N を CPU の数で頭打ちにするが、
  `mip_threads()` (`src/mip/solver.rs:2059`) は環境変数をそのまま使う。4 CPU で N = 8 なら、木・裏のカット・根の LP の競争は 4 の扱い、
  並列のサブ MIP の上限・早めの探索などは 8 の扱いになる。先生の機械では CPU が足りるので関係しないが、`enomoto_mip_cli.py` (handoff.md) を
  使う経路でも同じ環境変数を立てるなら、`mip_threads()` も `max_threads()` で頭打ちにするのが筋 (直していない)。

## 6. 再現

```
pip install -q highspy==1.15.1 maturin
maturin build --release -o /tmp/whl && pip install -q --force-reinstall --no-deps /tmp/whl/*.whl
cd analysis/mittelmann_ready_20261103/threads
python3 run.py t1,t2,t8,t16            # bench.jsonl (58 問、問題ごとに設定の順を逆にする)
python3 run.py t4 "" bench             # 同じコンテナの 4 スレッド
python3 run.py t4g "" gate             # ENOMOTO_MIP_IPM_HEUR_LPGATE=1 の 4 スレッド (gate.jsonl)
python3 run.py t1,t2,t4,t8,t16 ns1116954,blp-ar98 logs log   # ENOMOTO_MIP_LOG=1、logs/*.log.gz
python3 summ.py bench,gate > summ.txt
```

- 生データ: `threads/bench.jsonl` (行ごとに結果・壁時計・最大スレッド数・最大 RSS)、`gate.jsonl`、`logs.jsonl`・`logs/`、`rep2.jsonl` (ns1116954 の t1 のもう 1 回)。
