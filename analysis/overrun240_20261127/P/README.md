# 裏の研磨 `ENOMOTO_MIP_POLISH_BG=i` の P の計測 (2026-11-28、利用者の承認・親の依頼)

- 試作: `ENOMOTO_MIP_POLISH_BG=i` (src/mip/solver.rs の `polish_bg`・`PolishBg`)。中身は ../README.md の 9 節。
  - 暫定解が良くなるたびに、その暫定解の研磨 (整数を固定した LP、前処理つき) を頼みとして置く。
  - 共有の待ち行列でノードを待っている従のスレッドが、それを研磨する。
  - 返すときに、返す解の研磨が済んでいれば、その結果を使う。
- 見るもの: 木の探索に害が出ないか。比べるのは、同じスレッドの数の既定。
- 大きさ: P = MIPLIB の 58 問 × 種 1〜3 × {既定, i} × {2, 4 スレッド}、60 秒。計 696 回。
- 分担 (shards.json、plan.py): 分担 0 = 2 スレッド (def_t2・i_t2)、分担 1 = 4 スレッド (def_t4・i_t4)。各 348 回、見積りは各 2.8 時間。
- 版: f7c088a80 (試作のコミットは da3d183b7)。`<版>` はこれ。回す前に、短い動作の確かめ (下の「回す前の確かめ」) をする。

## 回し方 (分担 K)

ほかの仕事と同じ入れ物で回すときは、計測専用の worktree を作る。
`pip install -e .` は入れ物の enomoto_solver をその worktree に向けるので、同じ入れ物で別の計測やビルドを同時にしない。

```bash
cd /home/user/enomoto_solver
git fetch origin ccr-1beed81b-ex0uhq
git worktree add --detach /home/user/es_pbg FETCH_HEAD
cd /home/user/es_pbg
git diff --stat <版> HEAD -- src python Cargo.toml Cargo.lock pyproject.toml   # 空なら次へ。空でなければ:
#   git checkout <版> -- src python Cargo.toml Cargo.lock pyproject.toml
ln -s /home/user/enomoto_solver/.miplib_cache .miplib_cache
pip install -e .
python -c "import highspy" || pip install highspy==1.15.1
python analysis/protoscreen_20261124/check_default.py          # 「全部一致」(mik-250-20-75-4 は 3287 ノード)
python analysis/overrun240_20261127/P/run.py K                 # → analysis/overrun240_20261127/P/res_sK.jsonl (worktree の中)
```

- 2 時間の上限で止まったら、同じコマンドで回し直す。setsid・nohup は使わない。計測の途中でビルドし直さない。
- 結果は、区切りごとに本体のチェックアウトへ写して commit し、取り込んでから push する。

```bash
cd /home/user/enomoto_solver
cp /home/user/es_pbg/analysis/overrun240_20261127/P/res_sK.jsonl analysis/overrun240_20261127/P/
git add analysis/overrun240_20261127/P/res_sK.jsonl && git commit -m "overrun240 P: 分担 K の途中の結果"
git fetch origin ccr-1beed81b-ex0uhq && git merge --no-edit FETCH_HEAD && git push -u origin ccr-1beed81b-ex0uhq
```

## 回す前の確かめ (組んだ後に 1 回、約 1 分。cuanza は根で時間が尽きて木に入らないので、木に入る neos5 を使う)

組んだ機械で試作が動くかを見る (GGo5 の機械は別の計測中で、まだ組めていないため)。

```bash
ENOMOTO_MIP_POLISH_BG=i ENOMOTO_MIP_POLISH_STATS=v ENOMOTO_MIP_THREADS=2 python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/neos5.mps --time-limit 40 --threads 2 2>&1 | grep -E "POLISH_BG|^\{"
```

- 期待: `MIP: POLISH_BG: posted … started … solved …` の行と、結果の JSON。
- 落ちる (panic・crash)、または JSON が出ないときは、P を回さずに GGo5 に知らせる。

## 集計

`python analysis/overrun240_20261127/P/summ.py` (res_s*.jsonl を読む。`--perprob` で問題単位の比も出す)。
