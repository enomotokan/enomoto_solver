"""scan_<名前>.jsonl を集計する: 前処理の前・後のどちらかで対称性 (生成元か双子) のある問題を並べ、前で多い・後で多いを数える。
使い方: python summ.py 名前 [設定]"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
name = sys.argv[1]; cfg = sys.argv[2] if len(sys.argv) > 2 else "def"
rs = [json.loads(l) for l in open(D / f"scan_{name}.jsonl")]
rs = [r for r in rs if r["cfg"] == cfg]
z = {}
print(f"| 問題 | 畳み (DRCR) | 前: 生成元 / 生成元で動く列 / 双子の列 / 軌道の列 (整数) / 秒 | 後: 同じ | 今の検出のログ (生成元 / 軌道 / 列) |")
print("|---|---|---|---|---|")
cnt = {"both": 0, "before_only": 0, "after_only": 0, "more_before": 0, "more_after": 0, "none": 0, "no_after": 0}
for r in rs:
    b, a = r.get("before", z), r.get("after")
    if a is None:
        cnt["no_after"] += 1
        if b.get("orb_cols"):
            print(f"| {r['problem']} | {(r.get('fold') or '')[:40]} | {b.get('gens')} / {b.get('moved')} / {b.get('twin_cols')} / {b.get('orb_cols')} ({b.get('orb_int')}) / {b.get('secs')} | (後なし: {r.get('rc')}{' skipped' if r.get('skipped') else ''}) | |")
        continue
    bc, ac = b.get("orb_cols", 0), a.get("orb_cols", 0)
    if bc == 0 and ac == 0:
        cnt["none"] += 1; continue
    if bc and ac: cnt["both"] += 1
    elif bc: cnt["before_only"] += 1
    else: cnt["after_only"] += 1
    if bc > ac: cnt["more_before"] += 1
    if ac > bc: cnt["more_after"] += 1
    f = lambda x: f"{x['gens']} / {x['moved']} / {x['twin_cols']} / {x['orb_cols']} ({x['orb_int']}) / {x['secs']}"
    fold = r.get("fold") or ""
    fold = "畳まない" if fold.startswith("no fold") else (fold.split(" (")[0] if fold else "")
    print(f"| {r['problem']} | {fold} | {f(b)} | {f(a)} | {r.get('log')} |")
print()
print(len(rs), "問", cnt)
