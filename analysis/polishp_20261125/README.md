# POLISH_STRICT の P での比べ (2026-11-25、台本と分担表)

## 目的

analysis/nattai_20261125 の試作 `ENOMOTO_MIP_POLISH_STRICT` の =1・=2 を、既定と P で比べる。
- P は MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py)。
- 種 1〜3、60 秒、1・4 スレッド。
- 確かめること: 速さが変わらないこと、食い違いが 0 であること。

## 台本

台本は analysis/protoscreen_20261124 の写し。`run.py`・`summ.py` は同じ。

- `cfg.py`: 次の 6 設定。
  - `def_t1`・`def_t4` (既定)
  - `s1_t1`・`s1_t4` (STRICT=1)
  - `s2_t1`・`s2_t4` (STRICT=2)
- `plan.py [分担の数]` → `shards.json`。
  - 同じ (問題, 種) の 6 設定は、同じ分担で続けて解く。
  - 規模: 58 問 × 種 3 × 6 設定 = 1044 回。4 分担なら見積りは各 2.4 時間、計 9.6 時間。
  - 見積りの取り方: analysis/final_20261118 の P の既定の時間 (解けなければ 60 秒) に、待ち 8 秒を足した。

## 回し方

分担 K を回すとき:

```
git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD
pip install -e .
python analysis/polishp_20261125/run.py K        # → res_sK.jsonl (途中から再開できる)
python analysis/polishp_20261125/summ.py         # 集計
```
