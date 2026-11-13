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

## 結果 (2026-11-13、`summ.txt`)

8 分担 1160 回すべて完了、crash 0、目的値の食い違い 0。比は「項目 / base」(shift 1 秒の SGM、どちらかが解けた問題、問題単位のブートストラップ 95%)。

| 項目 | 対の比 [95%] | 解けた (base 170) | 4 種とも 1.25 倍以上遅い / 0.8 倍以下 |
|---|---|---|---|
| rmc100 | +1.5% [−2.1, +5.2] | 170 | - / - |
| **rmc150** | **−1.0% [−6.7, +4.3]** | 169 | - / - |
| rmc300 | +5.4% [+0.1, +11.4] | 164 | - / - |
| mush0 | +1.8% [−1.3, +5.3] | 171 | - / - |

- 事実: 58 問 60 秒では rmc150 は中立、rmc300 は害 (区間が 0 を越え、解けた −6)、rmc100・mush0 は中立。4 種そろって遅い・速い問題はどれにもない。
- REVIEW2 (Mittelmann 17 問 600 秒、土台 cand1113 + RCSTRONG=k) では rmc150 が −17.4% [−33.4, +0.1] (揺れの大きい 3 問を除き −10.9%)、mush0 は中立 (解けた +3)。

## 提案

1. **rmc150 (`ENOMOTO_T_MIP_ROOT_MAX_CUTS=150`) を cand1114 の次の束に入れる**。58 問で害がなく、Mittelmann で得の向き。束では Mittelmann 186 問 × 種 2 で確かめる
   (REVIEW2 は土台が cand1114 と 5 項目違い、17 問だけなので)。300 は 58 問で害があるので上げすぎない。
2. **mush0 は見送り**: 58 問・Mittelmann とも速さは中立で、解けた回の差 (+1・+3) は揺れの範囲。外しても得が見えない。
