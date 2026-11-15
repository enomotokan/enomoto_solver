# fiball などの切り分けの手順 (2026-11-13)

ビルドは 54a057b8 (cand1114 の P・M と同じ)。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach 54a057b8
git checkout origin/ccr-1beed81b-ex0uhq -- analysis/cand1114_20261113/fiball   # 台本だけ新しくする
pip install maturin highspy==1.15.1 && pip install -e .
python analysis/cand1114_20261113/fiball/run.py L K      # K = 0〜5 (親に依頼)。6 と F は手元
```

- 分担 0・1: fiball 種 1・2、2・3: ran14x18-disj-8 種 1・2、4・5: csched008 種 1・2。各分担は 8 形 × 600 秒 (最大 80 分)。
  形は cand1114・cand1113・cand1114 に cand1113 の項目を 1 つ戻した 6 形 (cfg.py)。
- 分担 6 (neos-2978193-inde 種 1・2) と F (衝突の直しの影響: 54a057b8 + conflict_pool.rs・domain.rs の直しのビルドで fiball・neos-2978193-inde の
  cand1114・cand1113、shards_F.json) は手元で回す。
- 終わったら res_L_sK.jsonl を名指しで add してコミット・push。
