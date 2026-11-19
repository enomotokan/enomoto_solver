# P2-INF: 実行不能を示す問題が遅い (neos859080・fhnw-binpack4-4・bnatt400)

親セッションからの依頼 (2026-11-19)。当方は neos859080 を 4 スレッドで時間切れ・1 スレッドで 128 秒 (SCIP 2.3 秒、HiGHS 1.6 秒、
0.1.9 は 27.6 秒)、fhnw-binpack4-4 を時間切れ (SCIP 12 秒)、bnatt400 を時間切れ (SCIP 142 秒)。neos859080 は 0.1.9 から悪くなったので、
まず 0.1.9 と今の既定の差を確かめる。

## 1. neos859080: 0.1.9 の後に既定にした機能を 1 つずつ切る (ab.py)

4 スレッド・120 秒・種 1〜3 (`python ab.py neos859080 120 4 1,2,3`、res_ab_neos859080.jsonl)。

| 設定 | 種 1 | 種 2 | 種 3 |
|---|---|---|---|
| v019 (PyPI 0.1.9) | 実行不能 25.6 秒 | 12.3 秒 | 24.8 秒 |
| def (今の既定) | 時間切れ | 時間切れ | 時間切れ |
| no_can・no_clqprb・no_seq・no_rfs30・no_drevmf・no_sw2・no_spl・no_s3g・no_rin・no_vubagg・no_orbpp | 33 回すべて時間切れ | | |

既定にした 11 の機能のどれを切っても解けない。悪くなったのは既定化した機能ではない。

## 2. commit の二分 (bis.sh・bisrun.sh) と原因

0.1.9 の頃の 2f9919194 (自分でビルドして 21.6・51.6 秒で実行不能) から 4e0dc7b34 (今の既定) まで、`src`・`Cargo.*` を変えた
commit を `git bisect run` で二分した (4 スレッド 90 秒、種 1・2、割れたら種 3。res_bis.jsonl、logs/bisect_log.txt)。

**最初の悪い commit は 558d7a4ce「ノードの LP の際どい実行不能を信用しない (widden の誤答の直し)」** (直前の f7fb51f84 は 20・29 秒で
実行不能、558d7a4ce は 2 種とも時間切れ、558d7a4ce に `ENOMOTO_MIP_NO_LPINF_CHECK=1` で 64・41 秒)。今の版でも
`ENOMOTO_MIP_NO_LPINF_CHECK=1` で 43・26・35 秒 (種 1〜3)。`ENOMOTO_MIP_LPINF_LOOSE=1` (ノードの解き直しの結果をそのまま使う) では 3 種とも時間切れ。

統計の行 (今の既定、種 2、60 秒) では、際どさの判定 `lp_farkas_ratio` が見た約 1000 本の証明のうち 767 本が「NaN・無限・負」の欄に入り、
ノードの LP の解き直し 663 回・強分枝の子を不明にした 104 回。一時の診断で見ると、射線は返っていて、標準形での確かめ
(`std_farkas_ok`、`ENOMOTO_MIP_DEBUG_LPINF`) はすべて成り立つ (Some(true)) のに、元の行の向きで測ると隙間が -∞ だった。

理由: `std_farkas_ok` は |z| が 1e-9 (1 + max|y|) 以下の項の無限の境界を無視するが、`lp_farkas_ratio` は行の重み y_i が 0 でなければ
(1e-17 程度の丸めの誤差でも) その行の境界を数える。片側が無限の行 (neos859080 の `y ≤ 10 b` など) に逆向きの微小な重みが載ると
行の区間が (-∞, ∞) になり、正しい証明を「際どい」とみなしていた。その結果、ノードを刈らずに解き直し、強分枝の子を不明にして
変数を固定せず、木が大きく育つ。

## 3. 試作 `ENOMOTO_MIP_LPINF_ROWTINY` (既定では使わない)

`lp_farkas_ratio` で |y_i| ≤ 1e-9 (1 + max|y|) の行を射線から落とし、z = yᵀA は残した行だけから作る。落とした後の射線そのものを
測るので、隙間が正ならそのまま厳密な証明 (緩めたのではない)。widden の 1e-8 程度の際どい実行不能は、落としても隙間が小さいので
今までどおり信用しない。

neos859080 (4 スレッド 90 秒): 種 1 46.5 秒・種 2 16.2 秒・種 3 51.0 秒で実行不能 (logs/neos859080_rowtiny_s*.log)。際どいとされた
証明は 0 本、全部「≥ 1e-4」の欄。cargo test 409 通過。

計測: `python plan.py 12 4 def rowtiny` → shards_MROWTINY.json (Mittelmann 48 問 × 種 1・2 × {def, rowtiny}、600 秒、12 分担・各 1.4 時間)・
shards_PROWTINY.json (MIPLIB 58 問 × 種 1〜3、60 秒、4 分担・各 0.74 時間)。`python run.py MROWTINY K` / `python run.py PROWTINY K`、
集計は `python summ.py MROWTINY def rowtiny`。判定の変更は LP の実行不能の扱い全部 (ノード・根・強分枝・LP のプロービング) に
効くので、M は似た問題に絞らず 48 問全部で見る。

### 3.1 正しさの確かめ (親の依頼、verify.sh)

ROWTINY は widden の誤答の直しの判定を変えるので、`ENOMOTO_MIP_LPINF_ROWTINY=1` で (1) neos-4722843-widden を 1・4 スレッド × 種 1〜3、
デバッグ解つき・900 秒で回し、最適 25009.662227 と比べる (wdn.py、res_dbgrun_wdn_t*.jsonl)、(2) release019 と同じ fuzz 7 本と
衝突解析の fuzz_cutconf 300 (ログは fuzz_logs/、コミットしない)。結果 (logs/verify.out、事実):

- widden: 1・4 スレッド × 種 1〜3 の 6 回すべて optimal 25009.6634 (正しい)。4 スレッド 46〜55 秒、1 スレッド 66〜83 秒。
  6 回とも際どい実行不能の判定は 0 回 (確かめに入らない)。デバッグ解の目的値は 26577.44 で今の前処理後の問題の最適と合わない (デバッグ解を作った版と
  前処理が変わった) ので、「MIP_DEBUG_SOL lost」の行 (3 回で計 19 行、刈り込みの上限 27631〜28518 の下) は判定の材料にならない。
  widden の誤答は直す前でも約 1/260 回と稀なので、6 回で言えるのは「6 回とも正しい」まで。ROWTINY が変えるのは丸めの誤差ほどの重みの行を
  落とすことだけで、落とした後の射線で隙間を測り直す (2・3 節) ので、widden の 1e-8 程度の際どい証明はこれまでどおり際どいと判定する。
- fuzz: fuzz_tiny 400 bad 0・mip_fuzz_vs_highs 300 bad 0・fuzz_objint 1000 bad 0・fuzz_slack 1000 食い違い 0・fuzz_tiny2 400 bad 0・
  fuzz_objint2 1000 bad 0・fuzz_slack2 1000 食い違い 0・fuzz_cutconf 300 bad 0。**誤答 0**。

## 4. fhnw-binpack4-4・bnatt400 (4 スレッド 120 秒・種 1、logs/<問題>_<設定>_s1.log)

| 問題 | def | rowtiny | NO_LPINF_CHECK | v019 |
|---|---|---|---|---|
| fhnw-binpack4-4 | 時間切れ (9.7 万ノード) | 時間切れ (10.0 万) | 時間切れ (10.1 万) | 時間切れ (7.0 万) |
| bnatt400 | 時間切れ (1.2 万) | 時間切れ (1.3 万) | 時間切れ (1.2 万) | 時間切れ (1.4 万) |

どちらも 0.1.9 から解けておらず、際どい実行不能の判定とは関係ない (neos859080 とは別の原因)。

### 4.1 fhnw-binpack4-4: 目的が全部 0 の問題に限って SST (試作 `ENOMOTO_MIP_SYM_SST_FEAS`、既定では使わない)

hang_20261117 の 7・15 節: SCIP は Schreier-Sims の表のカット (SST、45 行・9 段) で 12 秒で実行不能を示す。当方の SST
(`ENOMOTO_MIP_SYM_SST=1`) でも解けるが、orbital fixing の代わりに SST を使うと対称性のある最適化問題で P +20.8%・M +41.3% 遅い
ので既定にしなかった。SST と orbital fixing を同じ成分で混ぜると代表の選び方が食い違うおそれがあるので、使う条件を絞る:
**目的が全部 0 (実行可能性問題) のときだけ SST**。対称性で実行不能を示す必要があるのは主にこの型で、最適化問題の道筋は変わらない。
P・M (Mittelmann 48 問・MIPLIB 58 問) で目的が全部 0 なのは ns1116954 だけ (SST の行 19、def と sst で同じ道筋だった、hang 15.2 節)。

- fhnw-binpack4-4 (4 スレッド・種 1・300 秒、widden の確かめと並走で CPU を取り合った状態): **70.1 秒で実行不能** (7767 ノード、SST 45 行・9 段)。
  うち 30 秒は前処理の早めの FJ/Local-MIP (実行可能性問題の上限 30 秒をすべて使う)。cargo test 409 通過。
- 計測: shards_FSST.json (目的が全部 0 の 7 問 × 種 1〜3 × {def, sstfeas, def_t1, sstfeas_t1}、600 秒、6 分担 × 14 回) を `python run.py FSST K`、
  集計 `python summ.py FSST def sstfeas`・`python summ.py FSST def_t1 sstfeas_t1`。P は shards_PSSTFEAS.json (4 分担、各 0.74 時間、
  ns1116954 以外は道筋が同じはずなので揺れの確かめ)。M は効く問題がないので省く。
