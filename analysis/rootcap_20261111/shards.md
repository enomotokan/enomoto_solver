# ROOTCAP の 58 問の分担表 (2026-11-12)

- ビルド: このコミットのソース (試作 `ENOMOTO_MIP_ROOTCAP` を含む。既定の経路は変えていない)。台本 `shard58.sh` が wheel を作って /tmp/rootcap_py/rc に展開する。
- 設定: `def` (既定) と `rc` (`ENOMOTO_MIP_ROOTCAP=hdw`)。
- 60 秒・4 スレッド・種 1・2・3。問題は analysis/restchain_20261111/jobs58.txt の 58 問 (名前順)。
- 分担 5 つ: 分担 k は名前順で i % 5 == k の問題 (11〜12 問) を、種ごとに def → rc の順で続けて回す (同じ (問題, 種) の対は同じ分担)。
  1 分担は最長 12 問 × 3 種 × 2 設定 × 60 秒 = 72 分 (解ける問題があるのでふつうはもっと短い)。

| 分担 | コマンド (リポジトリの頭で) | 結果 |
|---|---|---|
| 0 | `bash analysis/rootcap_20261111/shard58.sh 5 0` | res58_sh0.jsonl |
| 1 | `bash analysis/rootcap_20261111/shard58.sh 5 1` | res58_sh1.jsonl |
| 2 | `bash analysis/rootcap_20261111/shard58.sh 5 2` | res58_sh2.jsonl |
| 3 | `bash analysis/rootcap_20261111/shard58.sh 5 3` | res58_sh3.jsonl |
| 4 | `bash analysis/rootcap_20261111/shard58.sh 5 4` | res58_sh4.jsonl |

終わったら `analysis/rootcap_20261111/res58_sh<k>.jsonl` だけをコミットして push する (`git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後)。
比べ方: `python analysis/rootcap_20261111/cmp58.py def rc "res58_sh*.jsonl"`。
