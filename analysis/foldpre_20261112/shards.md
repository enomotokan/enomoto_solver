# 分担表 (analysis/foldpre_20261112.md)

設定 nofold (`ENOMOTO_MIP_FOLD=0`)・def (今の既定 riq = 前処理の前に畳む)・fP (riPq = 前処理の後)・fB (riBq = 前と後の両方)、600 秒・4 スレッド。
1 行 = 1 本の分担 (同じ機械で 4 設定を続けて解くので対になる)。L0〜L2 はこのセッション、R0〜R2 は依頼。

wheel はコミット 2012d0d3 から作る: `maturin build --release -o W && pip install --no-deps --target PKG W/*.whl`。

実行 (分担ごと、問題・種の組ごとに 1 行。途中から再開できる):

```
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L0.jsonl nofold,def,fP,fB 1 600 physiciansched3-3   # L0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L0.jsonl nofold,def,fP,fB 1 600 supportcase33   # L0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L0.jsonl nofold,def,fP,fB 2 600 supportcase12   # L0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L0.jsonl nofold,def,fP,fB 2 600 neos-4722843-widden   # L0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L1.jsonl nofold,def,fP,fB 1 600 supportcase12   # L1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L1.jsonl nofold,def,fP,fB 1 600 neos-4722843-widden   # L1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L1.jsonl nofold,def,fP,fB 2 600 neos-5093327-huahum   # L1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L1.jsonl nofold,def,fP,fB 2 600 atlanta-ip   # L1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L2.jsonl nofold,def,fP,fB 1 600 neos-5093327-huahum   # L2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L2.jsonl nofold,def,fP,fB 1 600 atlanta-ip   # L2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L2.jsonl nofold,def,fP,fB 2 600 brazil3   # L2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_L2.jsonl nofold,def,fP,fB 2 600 bnatt400   # L2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R0.jsonl nofold,def,fP,fB 1 600 brazil3   # R0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R0.jsonl nofold,def,fP,fB 1 600 bnatt400   # R0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R0.jsonl nofold,def,fP,fB 2 600 cryptanalysiskb128n5obj14   # R0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R0.jsonl nofold,def,fP,fB 2 600 bnatt500   # R0
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R1.jsonl nofold,def,fP,fB 1 600 cryptanalysiskb128n5obj14   # R1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R1.jsonl nofold,def,fP,fB 1 600 bnatt500   # R1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R1.jsonl nofold,def,fP,fB 2 600 cryptanalysiskb128n5obj16   # R1
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R2.jsonl nofold,def,fP,fB 1 600 cryptanalysiskb128n5obj16   # R2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R2.jsonl nofold,def,fP,fB 2 600 physiciansched3-3   # R2
FOLD_PKG=PKG python analysis/foldpre_20261112/run.py heavy_R2.jsonl nofold,def,fP,fB 2 600 supportcase33   # R2
```

結果は analysis/foldpre_20261112/heavy_<分担>.jsonl。1 本あたり約 4 組 x 4 設定 x 10 分 = 2.5〜3 時間。
