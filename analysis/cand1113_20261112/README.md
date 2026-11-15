# cand1113 の確かめ (2026-11-12)

束 cand1113 = cfg.py の `CAND1113` (今日までに合格した 11 項目 + cand1112 の 7 項目 + OBJINT=hfc + CLQPRB=1d)。WRONGANS の seed14 の直しは既定に入っているので両側に入る。
入れないもの: FIXMORE (保留)・TREEEFF の D (保留)・CLQACT・TUBR・クリーク表の持ち方 (次の束で判断)・CUTRND・CLIQFB の各案・CLQEXT_KNAP (見送り)。
既知の注意: RCSTRONG は rmatr100-p10、GAP1=u は pk1 を遅くする (WINWHY、120 秒・種 2 つ)。physiciansched6-2 は揺れが大きいので種 5〜12 も回す。

## 手順 (分担 1 つ = セッション 1 つ)

固定するコミット: このファイルを足したコミット (分担の依頼に書く)。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach <固定するコミット>
pip install maturin highspy==1.15.1 && pip install -e .      # リリースのビルド (20〜25 分)
python analysis/cand1113_20261112/run.py {M|P} K              # 途中から再開できる。ソルバーは同時に 1 つ (4 スレッド)
```

- M (Mittelmann 186 問 × 種 1・2 × {def, cand1113} 600 秒 + 種 1 の HiGHS): 28 分担 (見積り各 約 4 時間)。同じ問題の回は同じ分担。
- P (58 問 × 種 1〜4 × {def, cand1113} 60 秒 + physiciansched6-2 の種 5〜12): 6 分担 (見積り各 約 1 時間)。
- 終わったら res_{M|P}_sK.jsonl だけを名指しで add してコミットし push する (`git add -A` は使わない)。

## 結果

### P (58 問、60 秒、種 1〜4) — summP.txt
SGM の対の比 −3.2% [−10.1%, +3.6%]、解けた 172 → 176、crash 0、誤答 0。

### M (Mittelmann 186 問、600 秒、種 1・2) — summM.txt
- 対 372 回。crash 1 (HiGHS の germanrr が -9 で落ちた。ENOMOTO は 0)。
- **誤答 2: unitcal_7 の cand1113 (種 1・2) が約 8 秒で実行不能と返した** (正しくは最適値 19635558)。原因は既定の TREE2_FORCEQ の許容誤差で、DUALEQ の作る等式行で表に出た。直し 40e80904 (REVIEW2、analysis/review2_20261113.md の「誤答」の節)。集計ではこの 2 回を解けていない扱いにした。
- 解けた: def 149 / cand1113 170 (372 回中)。SGM (shift 10): 215.9 → 202.0。対の比 −12.7% [−20.9%, −4.5%] (どちらかが解けた 92 問)。
- 2 種とも遅い: enlight_hard (HNFPRE=0)・neos-1122047 (IPMRC=utw)・cbs-cta (CLQPRB)・roll3000 (CLQPRB)・neos-960392 (SUBFIX=0)。括弧は loo.md・loo2.md で突き止めた原因の項目。
- 最大 RSS: def 6.8 GB / cand1113 7.6 GB (5 GB 超は 8 回 → 4 回)。

### HiGHS 1.15.1 との比べ (種 1、同じ分担・同じ機械、186 問)
| | HiGHS | def | cand1113 |
|---|---|---|---|
| 60 秒までに解けた | 37 | 49 | 50 |
| 600 秒までに解けた | 84 | 77 | 82 |
| 時間の比 (対 HiGHS、60 秒) | 1 | 0.73 | 0.74 |
| 時間の比 (対 HiGHS、600 秒) | 1 | 0.81 | 0.73 |

- HiGHS だけが解く (13): csched008・mcsched・mzzv11・neos-3216931-puriri・neos-873061・neos-933966・net12・rococoC10-001000・satellites2-60-fs・sct2・traininstance6・uccase12・unitcal_7 (誤答)。csched008・mcsched・traininstance6 は cand1113 も種 2 では解ける。
- cand1113 だけが解く (11): CMS750_4・comp07-2idx・fastxgemm-n2r6s0t2・fhnw-binpack4-48・istanbul-no-cutoff・mas74・neos-4738912-atrato・neos-787933・ran14x18-disj-8・supportcase40・wachplan。
- 両方解けて cand1113 が 2 倍以上遅い: hypothyroid-k1 (154 / 23 秒)・mzzv42z (199 / 50)・neos-3627168-kasai (242 / 113)。

### 既定化に向けて
束は Mittelmann・58 問とも速く、解ける数も増える。既定にする前に、次を決めてから束を組み直し、もう一度確かめる。
- 誤答の直し 40e80904 を入れる (unitcal_7)。
- RCSTRONG は irk でなく k にする (RCSTRONG2)。
- 遅い問題の原因になっている項目: HNFPRE=0 (外す方向)・IPMRC=utw・SUBFIX=0・CLQPRB・DUALEQ (binkar10_1)。
- FOLD は今の位置 (前で畳む) のまま (analysis/foldpre_20261112.md)。
- UNB_FEAS の直し (dc1f6842) は既定で入っている。
