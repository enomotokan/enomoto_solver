# PRUNE の計測の分担 (手順、2026-11-12)

既定の部品を 1 つずつ止めた形 (leave-one-out、`cfg.py` の 12 設定) と既定 (`def`) を、MIPLIB 58 問 × 種 1・2 × 60 秒・4 スレッドで回す。
同じ (問題, 種) の 13 設定は同じ分担で続けて回す (対で比べる)。ソルバーは 1 つの機械で同時に 1 つ。

## 手順 (1 分担 = 1 セッション、約 1 時間)

```sh
cd <リポジトリの根>
git fetch origin ccr-1beed81b-ex0uhq && git checkout <コミット>     # 下の「コミット」(既定の動きは HEAD と同じ、台本を足しただけ)
sh analysis/prune_20261112/build.sh                                  # "installed" と出れば良い (maturin・highspy==1.15.1 を入れてビルド)
cd analysis/prune_20261112
python run.py P <K>                                                  # 途中で切れても同じ命令で続きから回る
```

終わったら `res_P_s<K>.jsonl` だけをコミットして push する (`git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後
`git push -u origin ccr-1beed81b-ex0uhq`、失敗したら間を空けて最大 4 回)。途中でも 20 分ごとに push してよい。

- コミット: `d644a0e0` (このコミットの後の記録だけのコミットでもよい。src は d644a0e0 と同じであること)
- 分担: 0〜11 (`shards_P.json`、見積り各 約 1.04 時間)。10・11 は PRUNE のセッション自身が回す。

| 分担 | 命令 | 結果 |
|---|---|---|
| 0〜9 | `python run.py P 0` 〜 `python run.py P 9` | res_P_s0〜s9.jsonl |
| 10・11 | (PRUNE のセッション) | res_P_s10・s11.jsonl |

## 2 段目: Mittelmann 600 秒 (2026-11-12)

58 問で止めても害のなかった部品のうち、SUBFIX を単独で (`subfix0`)、HNFPRE・オービトープ 2 種・LP_PROBE_FINISH を束で (`prune3`) 止め、
既定 (`def`) と比べる。問題は analysis/hiloss_20261111.md の 10 問 (`plan_m.py`) × 種 1・2、600 秒・4 スレッド。同じ (問題, 種) の 3 設定は同じ分担。

- コミット: (下に書く)。手順は上と同じで、最後の命令だけ `python run.py M <K>`。結果は `res_M_s<K>.jsonl`。
- 分担 0〜7 (`shards_M.json`、見積り 0.65〜0.94 時間)。0・1 は PRUNE のセッション自身が回す。

| 分担 | 命令 | 結果 |
|---|---|---|
| 2〜7 | `python run.py M 2` 〜 `python run.py M 7` | res_M_s2〜s7.jsonl |
| 0・1 | (PRUNE のセッション) | res_M_s0・s1.jsonl |
