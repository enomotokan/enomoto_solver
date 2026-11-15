# FIXMORE の 58 問の分担 (2026-11-12)

既定 (`def`) と試作 `u` (`ENOMOTO_MIP_FIXMORE=u`) の対を、58 問 × 種 1〜3 × 60 秒・4 スレッドで回す。分担は 4 本 (`shards.json`、
各 86〜88 回、見込み約 40 分)。同じ (問題, 種) の 2 設定は同じ分担で続けて回す。分担 3 は FIXMORE のセッション (手元) が回す。

- 固定コミット: **5bc168e1** (計測中はチェックアウトを動かさない。ほかのセッションの push を取り込まない)
- 手順 (リポジトリの根で、ソルバーは同時に 1 つ):

```sh
git fetch origin ccr-1beed81b-ex0uhq && git checkout --detach 5bc168e1
pip install maturin highspy==1.15.1    # 未導入なら
sh analysis/fixmore_20261112/build.sh   # リリースのビルドと入れ直し ("installed" が出る)
python analysis/fixmore_20261112/run_share.py K --live    # K = 0, 1, 2 (live ファイルに追記、途中から再開できる)
```

- 終わったら (または区切りで): ブランチ ccr-1beed81b-ex0uhq の最新に戻り (`git checkout ccr-1beed81b-ex0uhq && git pull --rebase`)、
  `sh analysis/fixmore_20261112/safe_commit.sh "FIXMORE 分担 K: 完了 (N 行)"` で `res_sK.live.jsonl` の行を `res_sK.jsonl` に写して
  コミット・push する (ほかのファイルはコミットしない)。
- 命令の表:

| 分担 | 命令 | 回 |
|---|---|---|
| 0 | `python analysis/fixmore_20261112/run_share.py 0 --live` | 86 |
| 1 | `python analysis/fixmore_20261112/run_share.py 1 --live` | 88 |
| 2 | `python analysis/fixmore_20261112/run_share.py 2 --live` | 88 |
| 3 | (手元) `python analysis/fixmore_20261112/run_share.py 3 --live` | 86 |

## 種 4・5 の追加 (2026-11-12、`shards45.json`、3 分担、各 76〜78 回・見込み約 36 分)

種 1〜3 で SGM の区間の上限が基準を越えた (+6.6%) ので、既定と u の対を種 4・5 に足す。**解く側のソースは種 1〜3 と同じ 5bc168e1**
(その後に入った WRONGANS の既定の直しを混ぜないため)。台本だけ新しいコミットから取る:

```sh
git fetch origin ccr-1beed81b-ex0uhq && git checkout --detach 5bc168e1
git checkout <台本のコミット> -- analysis/fixmore_20261112/      # run_share.py の --file と shards45.json
sh analysis/fixmore_20261112/build.sh
python analysis/fixmore_20261112/run_share.py K --live --file shards45.json   # K = 0, 1, 2 → res45_sK.live.jsonl
```

終わったら、ブランチの最新に戻って (`git checkout -f ccr-1beed81b-ex0uhq && git pull --rebase`。live ファイルとログは追跡しないので残る)
`sh analysis/fixmore_20261112/safe_commit.sh "FIXMORE 種 4・5 分担 K: 完了 (N 行)"`。
