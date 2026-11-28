# heurfp の MIPLIB 240 問 (2026-11-28、親の依頼・利用者の判断「240 問で確かめてから既定化を判断」)

既定と heurfp (HEUR_ADAPT=1mkr + FP_STAGE3_LS=a) を、MIPLIB 240 問 × 1・4 スレッド × 種 1、上限 600 秒で比べる (960 回)。
台本は analysis/combo3_20261126/s2 の写しで、c3 を heurfp に置き換えただけ (分担の組み替え takeover.json・`--tail`、OOM 2 回で確定する決まりもそのまま)。

- 設定: cfg.py (def_t1・def_t4・heurfp_t1・heurfp_t4)。同じ (問題, スレッド) の def と heurfp は同じ分担で続けて回す (同じ機械・同じビルド)。
- 分担: `plan.py 6` → shards.json (6 分担、各約 19.7 時間の見積り)。
- 割り当て: 0 は私 (VB)、1 Eugj、2 FcsJ、3 Gkoa、4 Ei45、5 JJTM。
- 版: **bb9540475** (enum・nra の既定化の後の先端)。PREGD=pg の既定化 (Gm3w) は、始めた時点で届いていなかったので含まない。
  check_default は 6 問とも一致 (`check_default.out`)。heurfp の形の fuzz 7 本・cargo test は `fuzz_heurfp.out`。
- 集計: `python analysis/heurfp_20261128/s2/summ.py > analysis/heurfp_20261128/s2/summ.txt`。採用の目安: 区間の上端 ≲ 1.05、解けた数が減らない、
  全種 1.25 倍超の遅れなし、誤答 0。600 秒を越えた回は、終わってからログつきで解き直し、研磨 (`final polish`) によるものとそれ以外に分ける。

## 回し方 (分担 K)

```bash
cd /home/user/enomoto_solver
git fetch origin ccr-1beed81b-ex0uhq
git worktree add --detach /home/user/es_hf240 FETCH_HEAD
cd /home/user/es_hf240
git diff --stat bb9540475 HEAD -- src python Cargo.toml Cargo.lock pyproject.toml   # 空でなければ git checkout bb9540475 -- (同じパス)
ln -s /home/user/enomoto_solver/.miplib_cache .miplib_cache
pip install -e .
python -c "import highspy" || pip install highspy==1.15.1
python analysis/protoscreen_20261124/check_default.py          # 「全部一致」(mik-250-20-75-4 は 3277、markshare_4_0 は 265575 ノード)
python analysis/heurfp_20261128/s2/run.py K                    # → analysis/heurfp_20261128/s2/res_sK.jsonl (worktree の中)
python analysis/heurfp_20261128/s2/prog.py                     # 進み
```

- 2 時間の上限で止まったら同じコマンドで回し直す。setsid・nohup は使わない。計測の途中でビルドし直さない。同じ機械でほかの計測を重ねない。
- 結果は区切りごとに本体のチェックアウトに写して commit し、取り込んでから push する:
  `cp /home/user/es_hf240/analysis/heurfp_20261128/s2/res_sK.jsonl /home/user/enomoto_solver/analysis/heurfp_20261128/s2/` のあと、本体で add・commit・fetch・merge・push。
