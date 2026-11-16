# cand1115 + SUBMIP_HALT の上での TIMEDEP Et2 の 1 文字ずつ外し (4 スレッド)

設定は `cfg.py` (束 CAND1115 = TC=b、Et2 を含む。すべてに `ENOMOTO_MIP_SUBMIP_HALT=1` を足す)。full・-all (TIMEDEP なし = CAND1115C)・-x (文字 x を外す、-n は N も外す)・-sc (SEP_COST を既定 0.5 に)・-lns (LNS_MIN を既定 5 に) の 15 設定。
分担表 `shards.json` (`python plan.py 5`): eil33-2・neos17・neos-911970・physiciansched6-2・neos-827175 × 種 1〜4 の 60 秒、5 分担 (各 60 回、約 50 分)。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/timedep_20261114/td1115h/run.py K     # K = 0〜4。res_sK.jsonl に足す (途中から再開できる)
```

終わったら `res_sK.jsonl` だけをコミットして push する。まとめは `python analysis/timedep_20261114/td1115h/summ.py > summ.txt`。
