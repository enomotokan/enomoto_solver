# 手間の数え違いの直し 2 つ (2026-11-25、親の依頼・利用者の承認)

どちらも既定オフの試作。新しい閾値は作っていない。

1. サブ MIP の手間の数え違いと、1 スレッドで回ごとに結果が変わる件 (neos-1582420、comboharm README 10.1 節・lmipticks README 4 節)。
   → 試作 `ENOMOTO_MIP_SUBMIP_TICKS` (文字 `f`・`c`・`d`、`1` は全部)。
2. Local-MIP の仕事の上限 (`work_cap`) と `subheur_local_mip` の手数の上限 (lmipticks README 3 節)。
   → 試作 `ENOMOTO_MIP_LMIP_SCORE_TICKS` の中に入れた (列の走査で実際に見た非零 `col_work` で判定する)。

診断 `ENOMOTO_MIP_WORKCOUNT_DIAG=1` (解き方は変えない) を足した。
- 求解の区切り (前処理・根の LP・根のカットのループ・木に入るまで・木) ごとに、前の区切りからの本当の CPU 時間 (`clock_gettime` のスレッド CPU) と
  手間 (重み付き、秒に直したもの) の増えを `WCDIAG` の行に出す。
- サブ MIP の始め (手間の予算・時間の上限・手間の残り・壁時計の残り)、根のカットのループの止まった理由 (サブ MIP の中も) も出す。
- 注意: サブ MIP の行の `TICKS cpu` は、既定 (`ENOMOTO_MIP_TICKS` の `w`) では本当の CPU 時間ではなく、手間の秒 (`cap_clock`) を出している。

## 1. neos-1582420: 何が数えられていないか、なぜ回ごとに変わるか

### 1.1 揺れの出どころ (`logs_1582420_combo1k_r{1,2}.err`、comboharm の combo + `HEUR_ADAPT=1k`、1 スレッド・ログつき・2 本並べ)

- 2 回とも全体のノード数は同じでも、木の RINS のサブ MIP (`SUBFIX tree RINS try 1`) は 240・241・242 ノードと、回ごとに違う所で `TimeLimit` になった。
- 2 回の `WCDIAG` の行を順に比べると、最初のずれは根のヒューリスティクスの、入れ子のサブ MIP の中にある (木よりずっと前)。
  - 根の RENS (サブ MIP、手間の予算 6 秒) の中で、FP の第 3 段 (`fp_stage3`) が孫のサブ MIP を解く。
  - 第 3 段の上限は壁時計で決まる。上限 = 残り時間の 0.1 倍・最大 10 秒、局所探索 (`FP_STAGE3_LS=b`) に使った秒を引く。
    孫のサブ MIP には手間の期限 (`push_budget`) もかからない。
  - 孫の根のカットのループの後の壁時計の残りは 0.16 秒と 0.21 秒で、続く解き直しの止まる所 (LP の反復 97 と 257) がここで分かれた。
- そのずれが手間の合計に残る。木の大近傍探索の予算 `lns_avail_tk` は手間の合計から決まるので、木の RINS の予算も回ごとに違った
  (同じノード 101 で、手間の残り 3.230 と 3.245)。
- c2c (`HEUR_ADAPT=1mk`) では、第 3 段をサブ MIP の中で回さない (`m`) ので、この道に入らない。
  今の先端の c2c は、ログつき・ログなし・2 本並べのどれでも 327 ノードで同じだった (`rep_1582420.txt`)。

### 1.2 数えていない手間 (`logs_1582420_c2c_diag.err`、`logs_1582420_def_sepprof.err`)

区切りごとの CPU / 手間の比 (c2c、1 スレッド。木は 1.0〜1.4 で、手間の重みの当てはめの範囲):

| 区切り | 比 (CPU / 手間) | 中身 |
|---|---|---|
| 外側の根のカットのループ | 2.07〜2.46 | 下の表 |
| サブ MIP の根のカットのループ | 2.0〜2.7 | 同じ |
| 前処理 (外側・サブ MIP) | 1.2〜3.6 | 量は小さい (サブ MIP で 0.005〜0.15 秒) |
| 根のヒューリスティクス〜木に入るまで | 1.3〜2.3 | |
| 木 | 1.0〜1.4 | |

既定の外側の根のカットのループ (`ENOMOTO_MIP_DEBUG_SEPPROF`、17 ラウンド): 1 ラウンド 395 ms のうち、表 (tableau) の行のカットが 348 ms。
その中でカットの仕上げ `finish_cut` が 222 ms (候補 7190 本、平均 200 非零)。
- 分離の手間 (`SEP`) を足しているのは、カットの生成 (`cuts.rs`、項の数) と分離の準備 (LP の行の非零) だけ。
- `finish_cut_impl` (行の活動量の変数を LP の行で置き換えて構造変数の式にする) は数えていなかった。
- 手間は 3.39 秒、CPU は 7.32 秒 (比 2.16)。
- 診断用にしか数えない分離器 (zero-half・clique・implied bound の `diag_add`) は、この問題ではほぼ 0 だった (足しても比は 2.28 のまま)。

親の材料の「手間の和が CPU の約 1/6」は、サブ MIP の行の `TICKS cpu 3.25` を CPU と読んだ比べと見られる。この値は手間の秒で、壁時計は 5.9 秒。
本当の CPU と手間の比は、このサブ MIP 全体でおよそ 1.3〜2.3。

### 1.3 直し `ENOMOTO_MIP_SUBMIP_TICKS` (src/ticks.rs `submip_ticks`、src/mip/heuristics.rs `fp_stage3`、src/mip/separation.rs `finish_cut_impl`)

- `f`: FP の第 3 段の上限を手間で決める。
  - 残りは、手間の期限があればその残り (`ticks::left`)、なければ元の時間の上限 − 手間の経過 (`ENOMOTO_MIP_TICKS` の `s`・`u` と同じ)。
  - サブ MIP に手間の期限 (`push_budget`) をかけ、時間の上限は手間の上限の 10 倍 (壁時計の残りで頭打ち) の安全網にする。
  - 局所探索に使った分も手間で引く。
- `c`: `finish_cut_impl` で触った非零 (生のカットの項・置き換えた LP の行の長さ・触った列) を分離の手間に足す。
  - 外側の根のカットのループの比が 2.16 → 0.99 になった (`logs_1582420_c_sepprof.err`)。
  - 根のカットのループの上限も手間で数えるので、ラウンドの数は変わる (17 → 10)。
- `d`: 診断用の手間 (`diag_add`、重み `ENOMOTO_T_TICKS_W_DIAG` = 2) も合計に入れる。この問題では量が小さく、効きは見えない。
- 新しい閾値はない (10 倍の安全網と重みは既存のもの)。

### 1.4 確かめ (neos-1582420、1 スレッド・種 1・120 秒)

壁時計の進みを変えて、結果が変わるかを見た (`logcheck.sh` → `logcheck_1582420.txt`)。
- ログ (`ENOMOTO_MIP_LOG`) の出力は壁時計の進みを変える。4 コアに 6 本を並べて、負荷も高くした。

| 形 | ログあり | ログなし |
|---|---|---|
| combo1k (直しなし) | 1170 ノード | **1125 ノード** |
| combo1k + `SUBMIP_TICKS=f` | 1170 | 1170 |
| combo1k + `SUBMIP_TICKS=1` | 1153 | 1153 |

- 直しなしは、ログの有無 (壁時計) で道筋が変わる。直しをつけると変わらない。
  - 前の回 (ログつき・2 本並べ、`WORKCOUNT_DIAG` つき) では、直しなしは 3259 ノード (木の RINS が 240〜242 ノードで止まる)。
    `f`・`1` は 1170 ノードで、2 回とも同じだった。
- c2c (今の組み合わせ) は直しなしでも揺れない (1.1 節)。`SUBMIP_TICKS=1` をつけても 327 ノードで同じ (`rep_1582420.txt`)。
- `c` は根のカットのループの長さを変えるので、既定にするなら P で得失を見る必要がある。`f` は FP の第 3 段を回す形 (HEUR_ADAPT の `m` なし・
  `FP_STAGE3`) にだけ効く。

## 2. Local-MIP の仕事の上限 (src/mip/localmip.rs `LocalMip::run`・`real_work`、src/mip/heuristics.rs `subheur_local_mip`)

- `LMIP_SCORE_TICKS` のとき、`work_cap` の判定を「手数 × 1 手の見当 `step_work` + lift の非零」から、それに列の走査で実際に見た非零
  `col_work` を足した量にする。手間に足す量と同じ数え。
- `subheur_local_mip` (`SUBMIP_HEUR` の `l`) は、手数の上限 (予算 / 1 手の見当) の代わりに、`work_cap` = 予算として、仕事の量 (`real_work`) が
  予算に届いたら止める。
- `work_cap` を使うのは、ほかに `TPRE` の `w`・`g` (どれも既定では使わない) で、そこも同じ数えになる。

確かめ (`SUBMIP_HEUR=l`、1 スレッド・種 1・60 秒、`logs_lmip_*`・`out_lmip_*`、`ENOMOTO_MIP_DEBUG_SUBHEUR=1`):

| 問題 | LMIP_SCORE_TICKS | 1 回目の Local-MIP: 手数 / 予算 / 実際の仕事 / 秒 | 全部の回の秒 | 求解 |
|---|---|---|---|---|
| app1-1 | なし | 3392 手 / 2.15e6 / **1.19e8 (55 倍)** / 8.55 秒 | 11.56 (2 回) | 24.9 秒 (最適) |
| app1-1 | あり | 128 手 / 2.15e6 / 2.60e6 / 0.10 秒 | 0.16 | **12.9 秒** (最適) |
| traininstance6 | なし | 1088 手 / 1.59e5 / 8.91e5 (5.6 倍) / 0.05 秒 | 1.25 (5 回) | 時間切れ 29690 |
| traininstance6 | あり | 208 手 / 1.59e5 / 1.64e5 / 0.02 秒 | 0.20 (4 回) | 時間切れ 30160 |
| neos-4763324-toguru | どちらも | (60 秒でサブ MIP の Local-MIP に届かない) | | 時間切れ・同じ値 |

- 見当だけの判定では、密な列のある問題で予算の 5〜55 倍回っていた。直しの後は予算の 1.0〜1.2 倍で止まる (16 手ごとに見るので少し越える)。
- traininstance6 の得た値の差 (29690 / 30160) は、Local-MIP の回し方が変わったことによる道筋の違い (どちらも時間切れ)。

## 3. 正しさ・既定

- fuzz 7 本と cargo test: 直し・試作を全部つけて回した (`fuzz.sh all ENOMOTO_MIP_SUBMIP_TICKS=1 ENOMOTO_MIP_LMIP_SCORE_TICKS=1 ENOMOTO_MIP_SUBMIP_HEUR=l ENOMOTO_MIP_FP_STAGE3=1`、
  `fuzz_all.out`)。
  - 誤答 0: tiny 0/400・0/400、vs HiGHS 0/300、objint 0/1000・0/1000、slack 0/1000・0/1000。
  - cargo test は 424 通過・0 失敗・19 無視。
- 既定の道筋 (`check_default.py`、新しい基準 `ref_default.json`、`check_default.out`): 6 問とも一致 (021152f76 の取り込みの後のビルド)。
- 試作をつけないとき、新しい数えは何も足さない (`submip_ticks` は環境変数がなければ偽、`finish_cut` の数えも `c` のときだけ)。

## 4. 再現

```
E="..."  # analysis/combo2_20261125/cfg.py の C2C か、comboharm の combo + HEUR_ADAPT=1k
env $E ENOMOTO_MIP_WORKCOUNT_DIAG=1 ENOMOTO_MIP_DET_DIAG=1 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_T_MIP_SEED=1 \
  python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/neos-1582420.mps --time-limit 120 --threads 1
bash analysis/workcount_20261125/rep.sh <python> neos-1582420 120 3 "c2c=c2c" "c2c_st1=c2c ENOMOTO_MIP_SUBMIP_TICKS=1"
bash analysis/workcount_20261125/logcheck.sh <python> neos-1582420 120 "0 f 1" analysis/workcount_20261125/logcheck_1582420.txt
env ENOMOTO_MIP_DEBUG_SUBHEUR=1 ENOMOTO_MIP_SUBMIP_HEUR=l ENOMOTO_MIP_LMIP_SCORE_TICKS=1 ... app1-1 (60 秒)
```
