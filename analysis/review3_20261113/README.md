# REVIEW3: 根の 1 ラウンドのカットの本数の上限と MUSHAGG を cand1114 の上で 58 問で比べる (2026-11-13)

REVIEW2 (analysis/review2_20261113.md の (D)) の候補を、cand1114 (analysis/cand1114_20261113/cfg.py) を土台に 58 問 60 秒で比べる。
- 設定 (cfg.py): base = cand1114、rmc100・rmc150・rmc300 = `ENOMOTO_T_MIP_ROOT_MAX_CUTS` (既定 50) を 100・150・300、mush0 = `ENOMOTO_MIP_MUSHAGG=0`。
- 58 問 × 種 1〜4 × 5 設定 × 60 秒 × 4 スレッド = 1160 回。同じ (問題, 種) の 5 設定は同じ分担で続けて回す。8 分担、見積り各 1.4 時間 (`plan.py`、cand1113 の P の時間から)。
- 集計: `python summ.py > summ.txt` (項目 / base の対の比、shift 1 秒、問題単位のブートストラップ 95%)。

## 手順 (分担 1 つ = セッション 1 つ)

固定するコミット: この README を足したコミット (分担の依頼に書く)。TREE2_FORCEQ の直し (40e80904) と UNB_FEAS は既定に入っている。

```sh
git fetch --unshallow origin ccr-1beed81b-ex0uhq || git fetch origin ccr-1beed81b-ex0uhq
git checkout --detach <固定するコミット>
pip install maturin highspy==1.15.1 && pip install -e .      # リリースのビルド (20〜25 分)
python analysis/review3_20261113/run.py P K                   # K = 0〜7。途中から再開できる。ソルバーは同時に 1 つ (4 スレッド)
```

- 終わったら (途中でも区切りで) `analysis/review3_20261113/res_P_sK.jsonl` だけを名指しで add してコミットし、
  `git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後 push する (`git add -A` は使わない)。
