# neos-827175 の遅れの手当ての確かめ (tkfix)

設定は `cfg.py` の 4 通り (どれも環境変数だけで、ソースの既定は変えない):
- cand1117
- c17_mr8 = + `ENOMOTO_T_MIP_CUT_MIN_ROUNDS=8` (外側の根のカットのループを 8 ラウンドまでは時間の上限で止めない、既定 0 の試作の鍵)
- c17_w = + 当てはめ直した手間の重み (analysis/timedep_20261114.md の 12.9 節、`ENOMOTO_T_TICKS_W_<部分>`)
- c17_mr8w = 両方

neos-827175・neos-960392・10teams・neos-3083819-nubu × 種 1〜8、4 スレッド・60 秒、2 分担 (`python plan.py`、各 64 回、約 20 分の見込み)。
neos-827175 は `ENOMOTO_MIP_LOG=1` のログを `logs/` に残す。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/tkfix/run.py K      # K = 0・1
```

終わったら `res_sK.jsonl` と `logs/` をコミットして push する。まとめは `python analysis/timedep_20261114/tkfix/summ.py > summ.txt`。
