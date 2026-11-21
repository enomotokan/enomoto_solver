# 引き継ぎの鎖を共有の待ち行列でも共有する直し (`ENOMOTO_MIP_INHPOOL`) の 4 スレッドの時間への影響 (2026-11-21、親の依頼)

analysis/biglp_20261121 の 8 節で既定に入れた直し (n3div36 の 4 スレッドの RSS 5553 → 1347 MB) について、4 スレッドで 1 回ずつ見えた遅れ
(neos5 35 → 43 秒、noswot 19 → 27 秒、unitcal_7 215 → 242 秒) が揺れかどうかを、P の広い計測で確かめる。

- 設定 (cfg.py): def = 既定、old = `ENOMOTO_MIP_INHPOOL=0`。どれも 4 スレッド・60 秒。
- 分担表 (plan.py → shards_P.json): MIPLIB の 58 問 × 種 1〜3 × {old, def} = 348 回を 6 分担 (見積りは 1 分担 0.47 時間)。
- 回し方 (リポジトリの根から、分担 K = 0〜5): `cd analysis/inhpool_20261121 && python run.py P K` → res_P_sK.jsonl (途中から再開できる)。
  ビルドは直しの入った wheel (a781007d4 以降、`build.sh`)。
- 集計: `python ci.py 60 old def res_P_s*.jsonl` (ずらし幾何平均の比の 95% 区間、解けた数、全回で 1.25 倍超の遅れ)。

## 結果

(計測中)
