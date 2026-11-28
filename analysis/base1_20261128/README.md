# c2c の土台の部品を 1 つずつ既定に足して測る (2026-11-28、親の依頼・利用者の承認)

analysis/combo3_20261126 8.4 節: c3 の新しい部品は、どれも c2c の土台の部品を変える形で、単独では働かない。部品ごとに既定化を決めるため、
土台の部品を 1 つずつ今の既定に足して、P (MIPLIB の 58 問) の 1・4 スレッド、種 1〜3、60 秒で既定と比べる。

## 1. 設定 (cfg.py)

| 名前 | 足す環境変数 | 備考 |
|---|---|---|
| heur | HEUR_ADAPT=1mkr | `r` は漏れの直し (analysis/heurfix_20261128) |
| heurfp | HEUR_ADAPT=1mkr + FP_STAGE3_LS=a | FP_STAGE3_LS は HEUR_ADAPT が開く FP の第 3 段にだけ効くので、単独では測らない |
| nra | NODEROWS_ADAPT=r0k23p + NRA_SHARE=1 | share は 4 スレッドだけ働く |
| enum | ENUM_ADAPT=1st | |
| sbpb | SB_PROBEBD=1 + SB_PROP_NO_LP=1 | c3 の nolp の形 |

既定 (def_t1・def_t4) と合わせて 12 設定 × 58 問 × 種 3 = 2088 回。`plan.py 8` → shards.json (8 分担、各約 2.4 時間の見積り)。

## 2. 版

- **3f3054603** (先端: 7d43fcdac の DUALEQ・RSTLOOSE・PRR_TIGHT の既定化と、HEUR_ADAPT の `r` を含む)。
- check_default (`check_default.out`) と、全部の部品を入れた形の fuzz 7 本・cargo test (`fuzz_all.out`): 3 節。

## 3. 確かめ

(回している途中)

## 4. 回し方 (分担 K)

ほかの仕事と同じ入れ物で回すときは、計測専用の worktree を作る (`pip install -e .` は入れ物の enomoto_solver をその worktree に向けるので、
同じ入れ物で別の計測やビルドを同時にしない)。

```bash
cd /home/user/enomoto_solver
git fetch origin ccr-1beed81b-ex0uhq
git worktree add --detach /home/user/es_base1 FETCH_HEAD
cd /home/user/es_base1
git diff --stat 3f3054603 HEAD -- src python Cargo.toml Cargo.lock pyproject.toml   # 空なら次へ。空でなければ:
#   git checkout 3f3054603 -- src python Cargo.toml Cargo.lock pyproject.toml
ln -s /home/user/enomoto_solver/.miplib_cache .miplib_cache
pip install -e .
python -c "import highspy" || pip install highspy==1.15.1
python analysis/protoscreen_20261124/check_default.py          # 「全部一致」(mik-250-20-75-4 は 3287 ノード)
python analysis/base1_20261128/run.py K                        # → analysis/base1_20261128/res_sK.jsonl (worktree の中)
```

- 2 時間の上限で止まったら、同じコマンドで回し直す。setsid・nohup は使わない。計測の途中でビルドし直さない。
- 結果は区切りごとに本体のチェックアウトに写して commit し、取り込んでから push する:
  ```bash
  cd /home/user/enomoto_solver
  cp /home/user/es_base1/analysis/base1_20261128/res_sK.jsonl analysis/base1_20261128/
  git add analysis/base1_20261128/res_sK.jsonl && git commit -m "base1: 分担 K の途中の結果"
  git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD && git push -u origin ccr-1beed81b-ex0uhq
  ```

## 5. 集計

`python analysis/base1_20261128/summ.py > analysis/base1_20261128/summ.txt` (combo3 の第 1 段の summ.py と同じ表。部品ごとに、同じスレッドの数の既定と比べる)。
既定化の候補の目安: 区間の上端 ≲ 1.05、解けた数が減らない、全種で 1.25 倍を超える仕組みの害がない、食い違い 0。
