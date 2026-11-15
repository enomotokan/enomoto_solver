# ERWAIT の 58 問の分担表 (2026-11-11)

- ビルド: このコミットのソース (試作 `ENOMOTO_MIP_ERWAIT` を含む。既定の経路は変えていない)。
- 設定: `def` (既定) と `ew` (`ENOMOTO_MIP_ERWAIT=sbc`)。どちらも診断 `ENOMOTO_MIP_ERWAIT_DIAG=1` (ログだけ)。
- 60 秒・4 スレッド・種 1・2・3。問題は analysis/restchain_20261111/jobs58.txt の 58 問 (名前順)。
- 分担 4 つ: 分担 k は名前順で i % 4 == k の問題 (14〜15 問) を、種ごとに def → ew の順で続けて回す (同じ (問題, 種) の対は同じ分担)。

| 分担 | コマンド (リポジトリの頭で) | 結果 |
|---|---|---|
| 0 | `bash analysis/erwait_20261111/shard58.sh 4 0` | res58_sh0.jsonl |
| 1 | `bash analysis/erwait_20261111/shard58.sh 4 1` | res58_sh1.jsonl |
| 2 | `bash analysis/erwait_20261111/shard58.sh 4 2` | res58_sh2.jsonl |
| 3 | `bash analysis/erwait_20261111/shard58.sh 4 3` | res58_sh3.jsonl |

終わったら `res58_sh<k>.jsonl` だけをコミットして push する (`git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後)。

## 2 組目: def と ewk (`ENOMOTO_MIP_ERWAIT=sbkc`) (2026-11-12 に追加)

同じ分け方 (分担 k は i % 4 == k の問題、種 1・2・3、問題ごとに def → ewk)。def も同じ分担で回し直す (同じ分担の中で対にするため)。

| 分担 | コマンド (リポジトリの頭で) | 結果 |
|---|---|---|
| 0 | `bash analysis/erwait_20261111/shard58.sh 4 0 def,ewk resk58` | resk58_sh0.jsonl |
| 1 | `bash analysis/erwait_20261111/shard58.sh 4 1 def,ewk resk58` | resk58_sh1.jsonl |
| 2 | `bash analysis/erwait_20261111/shard58.sh 4 2 def,ewk resk58` | resk58_sh2.jsonl |
| 3 | `bash analysis/erwait_20261111/shard58.sh 4 3 def,ewk resk58` | resk58_sh3.jsonl |

終わったら `resk58_sh<k>.jsonl` だけをコミットして push する。比べは `python analysis/erwait_20261111/cmp58.py def ewk "resk58_sh*.jsonl"`。
