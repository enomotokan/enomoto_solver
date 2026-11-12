# 分担表 (analysis/foldpre_20261112.md)

設定 6 つ、600 秒・4 スレッド:

- nofold: `ENOMOTO_MIP_FOLD=0` (畳まない)
- def: 今の既定 `riq` (元を畳む → 前処理 → 解く)
- fP: `riPq` (前処理 → 畳む → 前処理なしで解く)
- fQ: `riQq` (前処理 → 畳む → 前処理 → 解く)
- fB: `riBq` (畳む → 前処理 → 畳む → 前処理なしで解く。前で畳めなければ fP と同じ)
- fBQ: `riBQq` (畳む → 前処理 → 畳む → 前処理 → 解く。前で畳めなければ fQ と同じ)

1 行 = 問題・種の 1 組 (6 設定を同じ機械で続けて解くので対になる、1 組約 1 時間)。L0〜L2 はこのセッション、R0〜R3 は依頼。

wheel はコミット 2012d0d3 以降から作る: `maturin build --release -o W && pip install --no-deps --target PKG W/*.whl`。

```
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L0.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 physiciansched3-3   # L0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L0.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 neos-4722843-widden   # L0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L0.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 brazil3   # L0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L0.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 bnatt500   # L0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L1.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 supportcase12   # L1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L1.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 atlanta-ip   # L1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L1.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 cryptanalysiskb128n5obj14   # L1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L2.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 neos-5093327-huahum   # L2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L2.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 bnatt400   # L2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L2.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 cryptanalysiskb128n5obj16   # L2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R0.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 brazil3   # R0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R0.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 bnatt500   # R0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R0.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 supportcase33   # R0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R1.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 cryptanalysiskb128n5obj14   # R1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R1.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 physiciansched3-3   # R1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R1.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 neos-4722843-widden   # R1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R2.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 cryptanalysiskb128n5obj16   # R2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R2.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 supportcase12   # R2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R2.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 atlanta-ip   # R2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R3.jsonl nofold,def,fP,fQ,fB,fBQ 1 600 supportcase33   # R3
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R3.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 neos-5093327-huahum   # R3
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R3.jsonl nofold,def,fP,fQ,fB,fBQ 2 600 bnatt400   # R3
```

結果は analysis/foldpre_20261112/heavy_<分担>.jsonl。途中から再開できる (同じ行をもう一度実行)。
