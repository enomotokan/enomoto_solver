# timedep の 58 問 × 種 1〜4 (5 回目、4 スレッド・60 秒)

利用者の方針「時間依存の部分をなるべく減らしたい。損得のないぐらいにできれば上出来」により、損の出ない値でできるだけ多くの文字を入れた組を
base と対で確かめる。設定は `cfg.py` (base = 束 cand1114、C = cand1114 + `ENOMOTO_MIP_TIMEDEP=mplwnNebtyc` + `SEP_COST=5` + `LNS_MIN=10`、
Cr = C + r)。分担表は `shards.json` (`python plan.py 6` で作った 6 分担。同じ (問題, 種) の 3 設定は同じ分担。見積り 1 分担 約 0.8 時間)。

## 1 分担の回し方 (4 CPU の機械で、ほかの計測と重ねない)

```bash
git fetch origin ccr-1beed81b-ex0uhq && git checkout ccr-1beed81b-ex0uhq   # 試作を含む版 (この README を足したコミット以降)
python -m venv .venv && .venv/bin/pip install maturin highspy
source .venv/bin/activate && maturin develop --release
python analysis/cbcimpl_20261029/probs.py          # 58 問を .miplib_cache に取得
python analysis/timedep_20261114/b58e/run.py K      # K = 0〜5。res_sK.jsonl に足す (途中から再開できる)
```

終わったら `analysis/timedep_20261114/b58e/res_sK.jsonl` だけをコミットして push する。計測中に `git pull --autostash` をしない
(書き込み中のファイルが置き換わる)。まとめは `python analysis/timedep_20261114/b58e/summ.py > summ.txt`。

## 1 文字ずつ外した形 (必要なときだけ)

C で全種そろって遅い問題が出たら、`python plan.py N --loo m,p,... --probs 問題,...` で base・C・C-文字 の分担表 `shards_loo.json` を作り、
`python run.py K loo` (結果は `res_loo_sK.jsonl`)、まとめは `python summ.py loo`。
