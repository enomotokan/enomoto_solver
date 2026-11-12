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

- コミット: `224a15d4c` (ここに台本を足した。後の記録だけのコミットでもよいが、src・Cargo は 224a15d4c と同じであること)。
- 分担: 0〜11 (`shards_P.json`、見積り各 約 1.1 時間)。10・11 は REVISIT のセッション自身が回す。

| 分担 | 命令 | 結果 |
|---|---|---|
| 0〜9 | `python run.py P 0` 〜 `python run.py P 9` | res_P_s0〜s9.jsonl |
| 10・11 | (REVISIT のセッション) | res_P_s10・s11.jsonl |

集計: `python summ.py` (設定ごとに既定との対の SGM・95% 区間・解けた回・揃って遅い問題・RSS の比・食い違い)。

## 種 3 の追加 (Q、2026-11-12)

種 1・2 の 116 対の集計 (`summ_P.txt`) で、区間の上端が +5% 以下かつ解けた回が減らなかった 6 件 (lm2・prepar・sbt・ncm・ns250s・opf3) と def を、
58 問 × 種 3 で回す (`plan_s3.py 4` → `shards_Q.json`、406 回、見積り各 約 0.83 時間)。コミット・ビルドは上と同じ (224a15d4c)。

| 分担 | 命令 | 結果 |
|---|---|---|
| Q0〜Q2 | `python run.py Q 0` 〜 `python run.py Q 2` | res_Q_s0〜s2.jsonl |
| Q3 | (REVISIT のセッション) | res_Q_s3.jsonl |

コミットメッセージは「REVISIT Q<K>: 途中 (n 行)」「REVISIT Q<K>: 完了 (n 行)」。
集計: `python summ.py 'res_*_s*.jsonl'` (種 3 の無い設定の「揃って遅い」は出ない。候補の 6 件だけを見る)。
