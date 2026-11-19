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
