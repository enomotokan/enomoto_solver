# TICKS の m の手当ての確かめ (tkm)

設定は `cfg.py` の 5 通り。基準は c17_mr8 = cand1117 + `ENOMOTO_T_MIP_CUT_MIN_ROUNDS=8`:
- c17_mr8: 今の `m` (MPSEP の最適面の移動を入れ替え 16 回で止める)
- c17_mr8_p8・c17_mr8_p4: 入れ替えを 8・4 回で止める (`ENOMOTO_T_MIP_TIMEDEP_MPSEP_P0`)
- c17_mr8_mtime: TICKS から `m` を外す (時間版)
- c17_mr8_mtk: 時間版と同じ式の手間の期限 (`ENOMOTO_T_MIP_TICKS_MPSEP_MOVE_FRAC=0.25`、このコミットで足した試作の鍵)

neos-827175・neos-960392・10teams・neos-3083819-nubu × 種 1〜8、4 スレッド・60 秒、2 分担 (`python plan.py`、各 80 回、約 25 分)。
同じ (問題, 種) の全設定は同じ分担に入れ、種は 1・3・5・7 と 2・4・6・8 に分けた。neos-827175 はログを `logs/` に残す。

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout <依頼に書いたコミット>
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py
python analysis/timedep_20261114/tkm/run.py K      # K = 0・1
```

終わったら `res_sK.jsonl` と `logs/` をコミットして push する。まとめは `python analysis/timedep_20261114/tkm/summ.py > summ.txt` (比は c17_mr8 に対して)。
