# 前処理の伝播の締めすぎ (当方が実行不能と誤る) の直し (2026-11-17)

sat_rens の形の乱数問題 (`analysis/dualfree_20261117/gen.py`) で、HiGHS が最適なのに当方が実行不能とする 3 問
(既定の 12422・15631、試作 `ENOMOTO_AGG_DUALFREE_INEQ` の 11185)。analysis/pcd_20261117 の 1 節で同じ型と分かったもの。

## 1. 原因 (最小の再現: seed 15631、連続 4 列・整数 3 列・12 行)

- 前処理の外側のラウンドごとに、不等式の伝播 (`propagate_split`) が連続列 x2 の上限を少しずつ下げる:
  −27096.510745019765 (ラウンド 0) → −27096.5107450241 (1) → −27096.5107453345 (2) → …
  x2・x3・x1 が行で循環につながっていて、伝播が循環を回るたびに丸めの誤差のぶん (1 回 1e-8〜3e-7、値の大きさ 2.7e4 に対し相対 1e-11)
  境界を締めすぎ、それがたまる。締めの判定は絶対 `PROPAGATE_EPS` (1e-9) だけなので止まらない。
- 最後に 2 列が 1 点に固まり、残った行 `0.5 x2 − x3 ≤ −108780.85` が 1.2e-4 破れて実行不能と判定する
  (診断 `ENOMOTO_DEBUG_PRESOLVE_INFEAS`)。12422・11185 では前処理の後の問題 (`ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED`) が HiGHS でも実行不能
  (連続列の幅が 1e-4〜5e-3 まで締められ、HiGHS の最適解が相対 4e-10 ほどはみ出す)。
- 境界が行から導ける値ちょうど付近の、退化に近い問題で起きる (生成器が時刻の列の境界を導ける値ちょうど・±1e-10〜1e-6 にする)。

## 2. 直し: 既存の相対改善閾値 `ENOMOTO_T_PROP_RELTOL` を 1e-10 にする

- 不等式の伝播で、有限の境界は締めの幅が `reltol × (1 + |境界|)` を超えるときだけ締める (既存の仕組み。HiGHS 流に、循環で境界が
  削られ続けるのを止めるために入れたが、測っておらず既定 0 のまま。docs/improvement_history.md の `propagate_split`)。
- 1e-10 で 3 問とも直る (1e-9・1e-8 でも同じ)。等式の伝播の閾値 `ENOMOTO_T_EQPROP_RELTOL` だけでは直らない。
- コードの変更は要らない (既定の値 `PROP_RELTOL` を変えるかどうか)。既定は 0 のまま (計測の後で判断)。

## 3. 確かめ (`fuzz.sh 1e-10` → `fuzz_res.txt`)

- release019 の 7 本: すべて bad 0。cargo test 419 件通過。
- sat_rens の形の乱数問題 (種 0〜19999): 12422・15631 (既定)・11185 (pcd) が直り、新しい食い違いはない。
  既定: 種 0〜9999 は 15 件で直す前と同じ、10000〜19999 は 12 → 10 件。pcd: 0〜9999 は 12 件で同じ、10000〜19999 は 10 → 9 件。

## 4. 手元の P (58 問 × 種 1、60 秒、4 スレッド。`summ_P.txt`・`summ_P2.txt`)

| 設定 | SGM 設定/def [95% CI] | 解けた数 def → 設定 | 目立って遅い | 食い違い |
|---|---|---|---|---|
| rt (`ENOMOTO_T_PROP_RELTOL=1e-10`、全列) | +1.5% [−3.5%, +9.0%] | 45 → 44 | noswot 3.09 (時間切れ)・neos-911970 1.37 | 0 |
| rtc (`ENOMOTO_T_PROP_RELTOL_CONT=1e-10`、連続列だけ) | +1.1% [−2.6%, +6.2%] | 45 → 45 | noswot 2.25・rout 1.21 | 0 |

- rt の noswot を種 2・3 でも解くと既定 13.8・14.2 秒、rt 32.2・30.4 秒 (`res_Q_s0.jsonl`)。整数列の小さな締めを失うためと見て、連続列だけにかける
  rtc を足した (`ENOMOTO_T_PROP_RELTOL_CONT`、既定 0)。
- rtc で noswot・neos-911970 を種 1〜3 (`res_R_s0.jsonl`): noswot は既定 44.2・57.0・時間切れ / rtc 37.1・7.9・17.4 秒、
  neos-911970 は既定 18.9・18.4・15.4 / rtc 24.6・19.4・11.4 秒。noswot は既定の側も大きく揺れる。
- rtc の fuzz (`fuzz_res_cont.txt`): rt と同じ (7 本 bad 0、cargo test 通過、3 問が直り新しい食い違いなし)。

→ 案: 既定にするなら rtc (`PROP_RELTOL_CONT = 1e-10`)。整数最適化の前処理の連続列だけに効き、整数列の締めは変えない
(LP の前処理では整数の印がないので全列にかかる点に注意。LP の計測は別に要る)。既定に入れるかは P/M の計測の後で判断。

## 5. P/M の計測 (def / rtc、`plan_pm.py 3 12`)

同じ (問題, 種) の 2 設定は同じ分担で続けて解く (順は種で入れ替える)。4 スレッド。`cfg.py` の rtc = `ENOMOTO_T_PROP_RELTOL_CONT=1e-10`。
- P: 58 問 × 種 1〜3、60 秒、3 分担 (計 348 回、見積り各約 0.92 時間): `python analysis/proptol_20261117/run.py PX K` (K = 0〜2)
- M: analysis/pcd_20261117 の M と同じ 26 問 × 種 1・2、600 秒、12 分担 (計 104 回、見積り各 0.93〜0.99 時間):
  `python analysis/proptol_20261117/run.py MX K` (K = 0〜11)
- 準備は analysis/pcd_20261117/README.md と同じ (venv・maturin develop・probs.py)。終わったら `res_PX_sK.jsonl`・`res_MX_sK.jsonl` をコミットして push。
- 集計: `python summ.py PX > summ_PX.txt`・`python summ.py MX > summ_MX.txt`・`python summ.py PXMX > summ_PXMX.txt`。

## 6. LP の計測 (LP の前処理には整数の印がないので、rtc の閾値が全列にかかる)

既存の A/B の台本 `scripts/ab_bench.py` (NETLIB の問題を 2 つの腕で交互に解き、時間の中央値と状態・目的値の違いを出す) を、同じ python で
片方にだけ環境変数を付けて回す。NETLIB は `python scripts/setup_netlib_data.py` で一度取得する (docs/netlib-data.md)。

```bash
python scripts/setup_netlib_data.py
python scripts/ab_bench.py --base .venv/bin/python --new .venv/bin/python --new-env ENOMOTO_T_PROP_RELTOL_CONT=1e-10 \
    --rounds 3 --out analysis/proptol_20261117/ab_lp.json > analysis/proptol_20261117/ab_lp.txt
```
1 分担。状態・目的値の違い (台本が出す) が 0 で、時間の中央値の比に目立つ悪化がないことを見る。
