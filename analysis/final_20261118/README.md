# 最終計測: 今の既定を 0.1.9・SCIP・HiGHS と比べる (2026-11-18、利用者の依頼)

依頼: 「mittelmann 全題と miplib で最終計測をやり、SCIP と HiGHS 比で遅い問題を調査、改善案を並列で実装、計測」。
候補 (can・clqprb・seq・rfs30・drevmf・sw2・sparsify A・s3g・rin) を既定にした後の版を、同じ機械で PyPI の 0.1.9 と対にして測る。
SCIP 10.0.2 と HiGHS 1.15.1 は 11-17 の行 (analysis/vsscip_20261117・analysis/vshighs_20261117、同じ種・同じ問題・同じ形の機械) を使う。

## 設定 (cfg.py)

- def: 今の既定、4 スレッド。def_t1: 今の既定、1 スレッド (SCIP は既定で逐次なので、公平な比べは def_t1 / scip)。
- v019: PyPI の enomoto-solver 0.1.9 の既定、4 スレッド。別の venv `<repo>/.v019env` (`sh setup_v019.sh` で作る) の python で同じ worker
  (`scripts/run_miplib_benchmark.py --worker-ours`) を動かす。

## 問題と分担 (plan.py)

- M: Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) × 種 1 × {def, def_t1, v019}、600 秒、44 分担 (見積り各 1.4〜1.57 時間): `python run.py M K` (K = 0〜43)
- P: MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {def, def_t1, v019}、60 秒、3 分担 (各 1.54 時間): `python run.py P K` (K = 0〜2)
- 同じ (問題, 種) の 3 回は同じ分担で続けて解き、順は問題ごとに回す。作り直しは `python plan.py 44 3`。

## 準備

```
pip install maturin highspy==1.15.1 && pip install -e .     # 今の版 (この台本のコミットでビルド)
sh analysis/final_20261118/setup_v019.sh                     # 0.1.9 を .v019env に
```

## 集計 (summ.py)

`python summ.py M > summ_M.txt`、`python summ.py P > summ_P.txt`。def / v019 (対)、def_t1 / scip、def / scip、def / highs、
揺れの目安 (v019 / 11-17 の 0.1.9、11-17 の 0.1.9 同士 (vsscip / vshighs))。遅い順の表 (比 > 1.25) が調べる問題の候補。

- 揺れの目安 (先に計算): 11-17 の 0.1.9 の def 同士 (vsscip と vshighs、M 186 問・種 1) は **−3.9% [−13.6%, +5.6%]**、解けた 91 / 88。
  1 問ずつでは supportcase33 3.4 倍・neos859080 2.1 倍・widden 2.1 倍など、同じ版でも 1.5〜3 倍の差が出る。別の日の行との比べは揺れが大きい。

## 結果 (summ_M.txt・summ_P.txt、47 分担そろい、2026-11-18)

- 誤答 0・crash 0 (M 186 問 × 種 1 × 3 設定、P 58 問 × 種 1〜3 × 3 設定)。
- **今 def / 0.1.9 (同じ機械の対)**: M −2.9% [−19.1%, +13.6%]、解けた 88 / 90。P −0.4% [−6.8%, +6.1%]、解けた 136 / 135。
  - M の内訳: 候補を選ぶのに使った 48 問は −18.4% [−44.8%, +9.2%] (34 / 33)、残りの 138 問は +8.1% [−9.1%, +29.6%] (54 / 57)。
    48 問に合わせすぎている疑い (ただし同じ 0.1.9 の別の日 (v019 / o019) でも +8.2% の揺れ)。
  - 全種で遅い (P): noswot 1.87・physiciansched6-2 1.87・neos5 1.54 (M でも neos5 1.66・physiciansched6-2 1.58)。
  - M 種 1 で遅い: neos859080 (時間切れ / 27.6 秒)・ns1208400 (時間切れ / 92.7、def_t1 は 51 秒)・n2seq36q 4.0・wachplan 2.5・
    supportcase33 2.6・fastxgemm 2.0・swath3 1.8・piperout-27 1.6 (0.1.9 は根で解く)・qap10 1.6 など (種 1 だけなので揺れを含む)。
- **SCIP**: def_t1 / scip は M −2.5% [−31.5%, +37.3%] (69 / 70)、P −24.4% (118 / 102)。def / scip は M −46.9%・P −50.4%。
  1 スレッドで SCIP より大きく遅い: fhnw-binpack4-4 (時間切れ / 12 秒)・neos859080 (128 / 2.3)・neos-3381206-awhea (33 / 1.2)・
  neos-1171448 (178 / 12)・neos-1445765 (414 / 45)・graph20-20-1rand (49 / 4.6、P でも 6 倍)・CMS750_4・cod105・swath1・istanbul・bnatt400 など。
- **HiGHS**: def / highs は M −43.9% [−58.7%, −25.5%] (88 / 84)。HiGHS だけ解ける: csched008・neos-873061・neos-933966・neos859080・net12・
  ns1208400・satellites2-40・satellites2-60-fs・sct2・tbfp-network・uccase12。HiGHS より遅い: physiciansched6-2 3.3・p200x1188c 3.1・pg 2.6・n2seq36q 2.6 など。
