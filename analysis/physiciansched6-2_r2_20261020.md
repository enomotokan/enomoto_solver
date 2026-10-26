# physiciansched6-2 (2 回目): HiGHS の解き方と、P3 から uccase12 を悪くする部分を除く (2026-10-20)

機械は 4 CPU (`nproc` = 4)。基準は develop `0542795` (クリーク行の拡張・行をまとめた後の前処理のかけ直し・同値な 0-1 列の
代入・P1・P2 が既定)。試作はブランチ `inv/physiciansched-r2-20261020`。試作はすべて環境変数で有効にし、既定は変えていない。
**計測はすべて 4 スレッド・60 秒** (ENOMOTO は `ENOMOTO_MIP_THREADS=4`・`m.solve(None, False, 60, None, None, 4, log=True)`、
HiGHS 1.15.1 は `threads=4`・`mip_rel_gap=0`)。計測の間は他の重い処理を動かしていない (HiGHS の計測用ビルドの間に走らせた
`ENOMOTO_MIP_LOG` の 30 秒の 1 回だけは例外で、ログの中身を見るのにだけ使った)。

スクリプトとログは `analysis/physiciansched6-2_r2_20261020/`: `eno_run.py <mps> <秒>`、`highs_run.py <mps> <秒> [option=値 ...]`、
`run_cfg.sh <問題> <秒> <名前> [環境変数=値 ...]` (ログは `logs/<問題>_<名前>_<回>.log`)、`summ.py [問題 ...]` (ログの最後の
JSON を並べる)、`batch1.sh`・`batch2.sh` (下の表の計測)。HiGHS の計測用ビルドのログは `logs/ps62_highs_instr.{log,err}`
(`scripts/highs_instrument.patch` を当てた `highs`)。

設定の略号:

- `P3` = `ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1` (前の計測の P3)
- `P5` = P3 + `ENOMOTO_MIP_INC_LP_CAP=1 ENOMOTO_MIP_RINS_FJ_BOOST=1` (この調査の提案)
- `CUT` = `ENOMOTO_MIP_FACE_SEP=off ENOMOTO_T_MIP_CUT_TIME_FRAC=0.3 ENOMOTO_T_MIP_CUT_LP_TIME_FRAC=0.1` (根のカットのループの診断)

## 1. 結論

事実:

1. **HiGHS は双対側 (根のカット) で下界を最適値まで閉じ、その後のサブ MIP で解を拾う**。前処理後 8861 × 4608 (2.0 秒)、
   根の LP 47410.76 (4.2 秒)、カットの 5 ラウンドで 49324 (= 最適値、9.5 秒)、根のサブ MIP (`L`) が 49324 を見つけて 1 ノードで
   終わる (12.9 秒)。計測用ビルドの分離器ごとの数 (本体の 13 ラウンドの合計) は tableau 2690 本・経路集約 429 本・mod-k 52 本で、
   下界を上げているのは tableau の分離 (ラウンド 1: tableau 256・経路 230 で 47410.8 → 48107.0、以後 tableau が大半)。
   FJ (`J`) は使っていない。
2. **ENOMOTO の根のカットは 48583.1 → 48587.4 しか上げない**。1 ラウンド目に 50 本 (平均 2 非零) を足して目的値が動かず、
   上限を 1000 本に広げた 2 ラウンド目の LP が 1 回の LP の時間の上限 (全体の 2% = 1.2 秒) で打ち切られてカットを外し、
   根のカットの時間の上限 (全体の 10%) に当たって終わる。面の頂点からの分離 (既定の `int2`) は面の LP を 2 回解くので、
   この上限の多くを使う。`CUT` (面の分離を切り、時間の上限を 0.3・0.1 に) にすると 10 ラウンド回り 48984.8 まで上がる
   (HiGHS の 49324 には届かない)。カットで上がった分は、この問題では解を出すのに効かなかった (§2.4)。
3. **P3 で uccase12 を悪くするのは `DUAL_UB` と `CS_OBJ_PIVOT` の組**。P3 から 1 つずつ外すと、`DUAL_UB` か `CS_OBJ_PIVOT` を
   外したときだけ 11507.41 に戻る (`INEQ_SINGLETON`・`FJ_LP_REPAIR` を外しても 798739)。原因は FJ の解の後の連続列の解き直しの
   LP: 両方があると FJ が根の LP の直後 (8.4 秒) に 865106 の解を出し、その解の整数列を固定して連続列を解き直す LP
   (`resolve_continuous`、時間の上限は全体の期限だけ) が 56 秒まで終わらず、その間、根のカット・ヒューリスティクスが
   1 つも回らない (60 秒の暫定解 798739)。
4. **この LP に時間の上限をつける (`ENOMOTO_MIP_INC_LP_CAP`) と uccase12 は治る**: P3 + CAP で 3 回とも 11507.41
   (最適値 11507.41)。LP は 1.2 秒・993 反復で打ち切られ、FJ の解はそのまま暫定解になり、その後ランダム丸め・RINS・FP で 11507.4 に
   なる。ただし最適の証明は 60 秒で終わらない (既定は 3 回中 2 回 49 秒で最適を示す。P3 + CAP・P5 は 5 回とも時間切れ)。
5. **physiciansched6-2 の暫定解を良くするのは根の RINS の時間の予算**。P3 では FJ の解 (50926 か 51429、スレッドのタイミングで
   変わる) からの RINS (自由な列 約 3000) が残り時間の 7% (約 3 秒) で根も終わらずに止まる回が多い。RINS が通った回は
   49624。根で暫定解が FJ のものだけのときに RINS の予算を残りの 25% (最大 15 秒) にする (`ENOMOTO_MIP_RINS_FJ_BOOST`) と、
   P3 + CAP + BOOST の 7 回は 49624・49624・51429・49624・49624・49524・**49324** (最後は最適値。P5 の 2 回は 49524 と 49324)。
   どの回も最適の証明はない (下界は 48587〜48724)。

推測:

- RINS のサブ MIP の中では、カットで下界が 0.55 秒で 49324 に届く (`ENOMOTO_MIP_SUBMIP_VERBOSE`、§2.5)。HiGHS の根と同じく
  「下界が最適値 → ほぼ整数の LP 解からヒューリスティクスで当てる」形がサブ MIP の中で起きている。サブ MIP の中で解を出しているのは
  ベクトル長ダイビング (49624) と、その後のノード (49524・49324)。外側の根の下界が上がらないのは、元の問題ではカットが弱い
  (§1 の 2) ため。
- uccase12 の解き直しの LP が遅いのは、`DUAL_UB` で 18090 本の連続列の境界を締めた上で `CS_OBJ_PIVOT` が費用のある連続列を
  小さいピボットで代入した問題で、整数列を全部固定すると根の基底から遠いからだと見ている (反復の数・条件数は見ていない)。

## 2. 調べたこと

### 2.1 両ソルバーの並び (60 秒)

| | HiGHS | ENOMOTO 既定 (0542795) | ENOMOTO P3 | ENOMOTO P5 |
|---|---|---|---|---|
| 前処理後 | 8861 × 4608 (2.0 秒) | 12119 × 6003 (2.1 秒) | 12086 × 5970 (3.4 秒) | 同 P3 |
| 根の LP | 47410.76 (4.2 秒) | 48583.1 (クリーク行の拡張の後、4.6〜7.2 秒) | 48583.1 | 48583.1 |
| カット後の根 | 49324 (9.5 秒、カット 2177 本・LP に 400 本) | 48587.4 (49〜50 本) | 48587.4 | 48587.4 |
| 最初の解 | 49324 (12.9 秒、`L`) | なし (3 回) | 50926 / 51429 (8.5〜9.6 秒、FJ) | 同 |
| 60 秒の暫定解 | 最適 (12.9 秒、1 ノード) | なし | 51429 (3 回) | 49524 / 49324 (RINS) |

ENOMOTO の下界は再スタートの後に根が終わらないと -inf と返る (P5 の 2 回とも。RINS の解の後、根の被約費用で 5 列が新しく固定されて
再スタートし、再スタートの根 (48724) の途中で時間切れ)。最良の下界はそれまでの根の 48587〜48724。

### 2.2 HiGHS の根 (計測用ビルド、`HSEP`・`HADD`)

本体の分離のラウンド (目的値は前処理の定数 36109.5 を引いた値で出るので足して書く):

| ラウンド | tableau | 経路集約 | mod-k | ラウンド後の LP |
|---|---|---|---|---|
| 1 | 256 | 230 | 1 | 48107.0 |
| 2 | 314 | 59 | 3 | 48678.3 |
| 3 | 288 | 30 | 2 | 49033.2 |
| 4 | 322 | 23 | 8 | 49123.8 |
| 5 | 463 | 36 | 5 | 49324.0 |
| 6〜13 | 1047 | 51 | 33 | 49324.0 |

(数は分離した本数で、LP に入った数ではない。LP の行は 8861 → 9351。) 根のヒューリスティクスのサブ MIP の時間は 0.11 秒。
`HIGHS_SKIP_SEP` で tableau を止めた比較はしていない。

### 2.3 ENOMOTO の根のカットのループ (`ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_SEP=1`)

既定 (`logs/physiciansched6-2_dbgsep_1.log`):

```
MIP: cut round 0: 50 cuts did not move the objective; up to 1000 cuts per round from now
MIP: cut round 0: 50 cuts (of 4120, sep 2.52s, LP 603 iters), LP rows 10754 (cut rows mean nnz 2), obj 4.8587391667e4 (10.08s)
MIP: cut round 1: LP status TimeLimit, removed the cuts (Optimal)
MIP: clique fallback: 100 cuts, LP 4.8587391667e4 -> 4.8587391667e4 ... not used (2.68s)
Root cuts: objective 4.8583100000e4 -> 4.8587391667e4, 50 cuts in the LP (16.51s)
```

1 ラウンドの分離 (今の頂点 + 面の頂点 2 つ) のうち、`separate` 自体は 1 回 0.3〜0.7 秒 (tableau が大半、候補 3000〜4600 本、
平均 約 530 非零)。ラウンドの分離の時間 2.52 秒のうち `separate` 3 回分を除く残りは面の頂点を求める LP 2 回とみている
(LP ごとの時間は測っていない。1 回の上限は 1.2 秒)。上限を広げた回 (`captime`:
`CUT_TIME_FRAC=0.4`・`CUT_LP_TIME_FRAC=0.15`) は面の LP が長くなり 1 ラウンドの分離が 17 秒、2 ラウンドで 48717.0。

| 設定 (各 1 回) | ラウンド | カット後の根 |
|---|---|---|
| 既定 | 1 (2 ラウンド目は LP の時間切れで外す) | 48587.4 |
| `CUT_TIME_FRAC=0.4 CUT_LP_TIME_FRAC=0.15` | 2 | 48717.0 |
| `CUT` (`FACE_SEP=off` + 0.3・0.1) | 11 (時間の上限) | 48984.8 |
| `CUT` + `ENOMOTO_MIP_CUTSEL_HIGHS=1` | 11 | 48957.0 |
| P3 + CAP + `CUT` (3 回) | | 48884〜48912 (再スタート後の根 49215.7 の回あり) |

`CUT` は解を出すのには効かない: P3 + CAP + `CUT` は 3 回とも 51429 (RINS がサブ MIP の根で止まる)。

### 2.4 P3 の切り分け (uccase12、60 秒)

| 設定 | 回数 | 60 秒の暫定解 | 最初の解 | 最適を示したか |
|---|---|---|---|---|
| 既定 | 3 | 11507.41 (3 回) | 25.9〜27.5 秒 | 2 回 (49.3・49.6 秒) |
| P3 | 3 | 798739 (3 回) | 60.0 秒 (865106) | なし |
| P3 − `INEQ_SINGLETON` | 1 | 798739 | 60.0 秒 | なし |
| P3 − `DUAL_UB` | 1 | 11507.41 | 24.6 秒 | なし |
| P3 − `CS_OBJ_PIVOT` | 1 | 11507.41 | 26.3 秒 | なし |
| P3 − `FJ_LP_REPAIR` | 1 | 798739 | 60.0 秒 | なし |
| P3 + `INC_LP_CAP` | 3 | 11507.41 (3 回) | 13.5〜15.8 秒 (865106) | なし |
| P5 | 2 | 11507.41 (2 回) | 12.5・13.0 秒 | なし |

P3 のログ (`logs/uccase12_p3log_1.log`):

```
MIP: root LP 1.1507291389e4 (12352 iters, 8.44s)
MIP: fixed-integer LP dual check: c x -7.2153274090e5, dual bound -1.5746114989e6 -> solve
MIP: new incumbent 8.6510606067e5 at node 0 (55.96s) [feasibility jump (parallel)]
Root cuts: objective 1.1507291389e4 -> 1.1507291389e4, 0 cuts in the LP (56.06s)
```

P3 + CAP (`logs/uccase12_p3cap_1.log`):

```
MIP: fixed-integer LP: TimeLimit (993 iters)
MIP: new incumbent 8.6510606067e5 at node 0 (9.46s) [feasibility jump (parallel)]
Root cuts: objective 1.1507291389e4 -> 1.1507351783e4, 14 cuts in the LP (16.17s)
MIP: new incumbent 1.1507906216e4 at node 0 (18.37s) [randomized rounding]
```

`INEQ_SINGLETON`・`FJ_LP_REPAIR` はこの悪化に関わらない。`DUAL_UB` + `CS_OBJ_PIVOT` の組で FJ が根の LP の直後に解を出し、
それが時間の上限のない LP を呼ぶ形。`INC_LP_CAP` は P3 のどれとも別の場所 (暫定解の受け取り) を
直すので、P3 の 4 つを残したまま足せる。

### 2.5 physiciansched6-2 の RINS (`ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_VERBOSE=1`)

P3 + CAP の RINS は FJ の解と LP 解で一致する整数列を固定し、自由な列 2947〜3165 本のサブ MIP を解く。既定の予算 (残りの 7%、
最大 6 秒) では 2.1〜3.4 秒で止まり、通る回 (49624) と根が終わらない回がある。`RINS_FJ_BOOST` のサブ MIP の中
(`logs/physiciansched6-2_p3capRv_1.log`):

```
MIP: presolve: rows 10672 -> 3905, free columns 3071 -> 2479 ...
MIP: root LP 4.9074000000e4 (1220 iters, 0.11s)
MIP: cut round 0: 52 cuts ... obj 4.9324000000e4 (0.23s)
MIP: cut loop done: 142 cuts added, 124 in LP, ... obj 4.9074000000e4 -> 4.9324000000e4
MIP: root heuristic feasibility pump: none (2.20s)
MIP: root heuristic fractional diving: none (2.37s)
MIP: new incumbent 4.9624000000e4 at node 0 (6.75s) [vector length diving]
```

| 設定 | 回数 | 60 秒の暫定解 |
|---|---|---|
| P3 | 3 | 51429・51429・51429 |
| P3 + CAP | 3 | 49624・51429・51429 |
| P3 + CAP + `RINS_FJ_BOOST` (ログあり 2 回を含む) | 5 | 49624・49624・51429・49624・49624 |
| P5 (= 上と同じ設定、ログなし) | 2 | 49524・49324 |

うまくいかなかった試作: サブ MIP の中で、外側の暫定解の打ち切り値があってもサブ MIP 自身に暫定解がなければ暫定解なし用の RENS を
回す (`ENOMOTO_MIP_SUBMIP_RENS=cutoff`、1 回)。深さ 2 の RENS が 3 回ずつ実行不能か時間切れになり、RINS の時間を使うだけで 51429。
コードは消した。

### 2.6 副作用 (60 秒)

| 問題 | 既定 | P5 |
|---|---|---|
| qiu | 最適 27.3・30.3 秒 | 最適 27.2・30.9 秒 |
| eil33-2 | 最適 37.6・39.1 秒 | 最適 44.9・36.8 秒 (P3: 43.1・43.0、P3 + CAP: 32.6・41.0) |
| uccase12 | 11507.41 (最適 2/3 回) | 11507.41 (最適 0/2 回) |
| neos-911970 | 54.83 / 下界 53.53 | 54.76 / 53.88 |
| mas74 | 11801.19 / 11191.3 | 11801.19 / 11194.5 |
| mas76 | 最適 (138855 ノード) | 最適 (131489 ノード) |
| bppc4-08 | 54 / 51.59 | 54 / 51.59 (最初の解 290 → 94) |
| pk1 | 最適 26.9 秒 | 最適 26.3 秒 |
| khb05250 | 最適 1.1 秒 | 最適 1.2 秒 |
| supportcase26 (3 回) | 1802.2・1759.8・1776.8 | 1811.2・1759.8・1806.5 |
| rout | 最適 10.6 秒 | 最適 9.2 秒 |
| binkar10_1 | 最適 13.0 秒 | 最適 12.7 秒 |
| fiber | 最適 1.4 秒 | 最適 1.1 秒 |
| timtab1 (3 回) | 774858・780064・785781 | 797561・785429・782531 |
| flugpl | 最適 0.1 秒 | 最適 0.1 秒 |

問題は P3 の試作が働くと前の調査で分かっている問題 (§3.1 of `physiciansched6-2_fj_20261020.md`) と、前の計測で P3 が落とした
qiu・eil33-2。最適値はどれも一致。qiu・eil33-2 はこのコンテナでは P3 でも 60 秒に余裕がある (前の計測の時間切れは遅いコンテナで
60 秒の際)。timtab1 は P5 の平均がやや悪い (788.5k 対 780.2k) が回ごとの揺れ (前の調査で既定 781839 / 797170) と重なる。
`INC_LP_CAP` が打ち切った LP は uccase12 でしか見ていない (40 問のどれで打ち切りが起きるかは数えていない)。`RINS_FJ_BOOST` は
根で暫定解が FJ のものだけの問題で RINS の予算を 15 秒まで広げるので、そういう問題で根が延びうる。

## 3. 試作 (既定では使わない)

| 環境変数 | 変えた関数 | 内容 |
|---|---|---|
| `ENOMOTO_MIP_INC_LP_CAP` | `src/mip/solver.rs` `resolve_continuous` | 暫定解の整数列を固定して連続列を解き直す LP の時間の上限を、全体の `ENOMOTO_T_MIP_INC_LP_TIME_FRAC` (0.02、最低 1 秒) にする (既定は全体の期限だけ)。打ち切ったら解はそのまま。`ENOMOTO_MIP_LOG` で打ち切りを出す (`fixed-integer LP: TimeLimit`、この行は設定によらず出る) |
| `ENOMOTO_MIP_RINS_FJ_BOOST` | `src/mip/heuristics.rs` `rins` (中身は `rins_inner` に移した) | 根 (ノード 1 以下) で暫定解が FJ のものだけのとき (`inc_only_fj`)、RINS のサブ MIP の時間の上限を残りの `ENOMOTO_T_MIP_RINS_FJ_BOOST_FRAC` (0.25)、最大 `ENOMOTO_T_MIP_RINS_FJ_BOOST_MAX` (15 秒) にする (既定 0.07・6 秒) |

`cargo test --release --lib`: 308 件通過・17 件無視 (`RUSTFLAGS="-L <LIBDIR> -lpython3.13"`)。

## 4. 既定にする候補か

- **P5 (P3 + `INC_LP_CAP` + `RINS_FJ_BOOST`) を親の 40 問 + 12 問の対の計測にかける候補にする**。physiciansched6-2 は 2 回とも
  暫定解が出て 49524 / 49324 (P3 は 51429)、uccase12 の 798739 は出なくなる。残る心配は uccase12 で最適の証明が 60 秒に
  入らなくなること (既定 2/3 → 0/5。暫定解は同じ) と timtab1 の揺れ。
- `INC_LP_CAP` は単独でも既定の候補 (時間の上限のない LP を 1 つ減らすだけ)。ただし既定の組で打ち切りが起きる問題は見ていない。
- `CUT` の組は候補にしない (この問題で解に効かない。面の分離は既定の理由があって入っている)。カットの弱さ (48587 対 49324) は
  別の調査が要る。

計測にそのまま渡せる形:

```
P5: ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_INC_LP_CAP=1 ENOMOTO_MIP_RINS_FJ_BOOST=1
```

## 5. 確かでない点・次の手

- 根のカット: ENOMOTO は 1 ラウンド目に平均 2 非零のカットを 50 本選んで目的値が動かず、2 ラウンド目に 1000 本を入れた LP が 1.2 秒の
  上限で落ちる。HiGHS は tableau のカットで 5 ラウンドで最適値に届く。HiGHS の前処理後の問題でも ENOMOTO のカットは 48513
  (前の調査) なので、前処理の差ではなくカットの選び方・tableau の分離の差。`HIGHS_TAB_DUMP` で同じ LP の tableau のカットを比べるのが
  次の手 (`scripts/highs_instrument.md`)。
- 1 回の LP の時間の上限 (全体の 2%) で 2 ラウンド目のカットを全部外すのは、大きい問題では根のカットのループを止める。広げた
  上限 (0.1) の副作用は見ていない。
- 再スタートの後に根が終わらないと下界が -inf で返る (P5 の physiciansched6-2・uccase12)。最良の下界は再スタート前の根の値なので、
  返す値をそれと再スタート後の値の大きい方にするべき (直していない)。physiciansched6-2 の再スタートは 109346 列中 5 列の固定で
  起き、残り 40 秒を再スタートの根に使う。
- uccase12 の解き直しの LP が遅い理由 (§1 の推測) は確かめていない。
- `RINS_FJ_BOOST` の 7 回のうち 1 回は 15 秒の予算でもサブ MIP の根が終わらなかった (自由な列 3165)。
