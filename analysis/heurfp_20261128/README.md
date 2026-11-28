# heurfp (HEUR_ADAPT=1mkr + FP_STAGE3_LS=a) の 600 秒の確かめ (2026-11-28、親の依頼・利用者の承認)

analysis/base1_20261128 7 節: heurfp は P の 60 秒で 4 スレッドは候補 (1.004、解けた数同じ)、1 スレッドは 1.011・解けた数 −3 (どれも上限すれすれ)、
mc11 (問題単位 1.07)・pk1 (1.05) が小さく遅い向きだった。600 秒で、遅い向きの問題と狙いの問題を 1・4 スレッド、種 1〜3 で既定と比べる。

- 問題: mc11・pk1 (1 スレッドで遅い向き)、30n20b8・CMS750_4・bnatt400・neos-1171448 (heurfp の狙い)。
- 設定: def (enum・nra の既定化 f281fa7b8 の後の既定) と heurfp (cfg.py)。6 問 × 種 3 × 2 設定 × 2 スレッド数 = 72 回、上限 600 秒。
- 分担: `plan.py 4` → shards.json (4 分担、各 18 回、最長 3 時間)。
- 版: **55c7060b1** (enum・nra の既定化の後の先端)。
- 集計: `python analysis/heurfp_20261128/summ.py > analysis/heurfp_20261128/summ.txt` (base1 の summ.py、上限を 600 秒にしたもの)。

## 回し方 (分担 K)

```bash
cd /home/user/enomoto_solver
git fetch origin ccr-1beed81b-ex0uhq
git worktree add --detach /home/user/es_heurfp FETCH_HEAD     # 前の worktree が残っていれば別の名前で
cd /home/user/es_heurfp
git diff --stat 55c7060b1 HEAD -- src python Cargo.toml Cargo.lock pyproject.toml   # 空でなければ git checkout 55c7060b1 -- (同じパス)
ln -s /home/user/enomoto_solver/.miplib_cache .miplib_cache
pip install -e .
python -c "import highspy" || pip install highspy==1.15.1
python analysis/protoscreen_20261124/check_default.py          # 「全部一致」(mik-250-20-75-4 は 3277、markshare_4_0 は 265575 ノード)
python analysis/heurfp_20261128/run.py K                       # → analysis/heurfp_20261128/res_sK.jsonl (worktree の中)
```

- 2 時間の上限で止まったら同じコマンドで回し直す。setsid・nohup は使わない。計測の途中でビルドし直さない。同じ機械でほかの計測を重ねない。
- 結果は本体のチェックアウトに写して commit し、取り込んでから push する。
- 割り当て: 0 は私 (VB)、1 FKBv、2 FVbj、3 GU3WAp。
