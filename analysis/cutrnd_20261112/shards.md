# CUTRND の 58 問の分担表 (2026-11-12)

- ビルド: このファイルを入れたコミットのソース (試作 `ENOMOTO_MIP_CUTRND` を含む。既定の経路は変えていない)。台本 `shard58.sh` が wheel を作って
  /tmp/cutrnd_py/a に展開する。計測中はチェックアウトを動かさない (wheel を作った後なら動かしてよいが、作り直さないこと)。
- 設定: `def` (既定) と `cr` (`ENOMOTO_MIP_CUTRND=esf`)。
- 60 秒・4 スレッド・種 1・2・3。問題は analysis/restchain_20261111/jobs58.txt の 58 問 (名前順)。
- 分担 5 つ: 分担 k は名前順で i % 5 == k の問題 (11〜12 問) を、種ごとに def → cr の順で続けて回す (同じ (問題, 種) の対は同じ分担)。
  1 分担は最長 12 問 × 3 種 × 2 設定 × 60 秒 = 72 分 (解ける問題があるのでふつうはもっと短い)。ソルバーは同時に 1 つ。

| 分担 | コマンド (リポジトリの頭で) | 結果 |
|---|---|---|
| 0 | `bash analysis/cutrnd_20261112/shard58.sh 5 0` | res58_sh0.jsonl |
| 1 | `bash analysis/cutrnd_20261112/shard58.sh 5 1` | res58_sh1.jsonl |
| 2 | `bash analysis/cutrnd_20261112/shard58.sh 5 2` | res58_sh2.jsonl |
| 3 | `bash analysis/cutrnd_20261112/shard58.sh 5 3` | res58_sh3.jsonl |
| 4 | `bash analysis/cutrnd_20261112/shard58.sh 5 4` | res58_sh4.jsonl |

終わったら `analysis/cutrnd_20261112/res58_sh<k>.jsonl` だけをコミットして push する (`git pull --rebase --autostash origin ccr-1beed81b-ex0uhq` の後)。
比べ方: `python analysis/cutrnd_20261112/cmp58.py def cr "res58_sh*.jsonl"`。
