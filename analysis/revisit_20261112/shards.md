# REVISIT の計測の分担 (手順、2026-11-12)

以前に既定化を見送った試作 13 件 (`cfg.py`) と既定 (`def`) を、MIPLIB 58 問 × 種 1・2 × 60 秒・4 スレッドで回す。
同じ (問題, 種) の 14 設定は同じ分担で続けて回す (対で比べる)。ソルバーは 1 つの機械で同時に 1 つ。
台本は PRUNE (analysis/prune_20261112/) と同じ形。

## 手順 (1 分担 = 1 セッション、約 1.1 時間)

```sh
cd <リポジトリの根>
git fetch origin ccr-1beed81b-ex0uhq && git checkout <コミット>     # 下の「コミット」
sh analysis/revisit_20261112/build.sh                                # "installed" と出れば良い (maturin・highspy==1.15.1 を入れてビルド)
cd analysis/revisit_20261112
python run.py P <K>                                                  # 途中で切れても同じ命令で続きから回る
```

終わったら `res_P_s<K>.jsonl` だけをコミットして push する (`git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後
`git push -u origin ccr-1beed81b-ex0uhq`、失敗したら間を空けて最大 4 回)。途中でも 20 分ごとに push してよい。
コミットメッセージは「REVISIT P<K>: 途中 (n 行)」「REVISIT P<K>: 完了 (n 行)」。

- コミット: このファイルを足したコミット (`git log -1 --format=%h -- analysis/revisit_20261112/shards.md`)。src は adebd1ef5 と同じ
  (この台本は既定の動きを変えない。後の記録だけのコミットでもよい。src が同じであること)。
- 分担: 0〜11 (`shards_P.json`、見積り各 約 1.1 時間)。10・11 は REVISIT のセッション自身が回す。

| 分担 | 命令 | 結果 |
|---|---|---|
| 0〜9 | `python run.py P 0` 〜 `python run.py P 9` | res_P_s0〜s9.jsonl |
| 10・11 | (REVISIT のセッション) | res_P_s10・s11.jsonl |

集計: `python summ.py` (設定ごとに既定との対の SGM・95% 区間・解けた回・揃って遅い問題・RSS の比・食い違い)。
