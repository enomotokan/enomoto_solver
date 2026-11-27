"""置く場所の比べ (enumwhere.sh) の表: python ewtab.py > ewtab.md"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
last = {}
for l in open(D / "ours.jsonl"):
    r = json.loads(l)
    if r["tag"].startswith("ew_") and r["tag"] != "ew_test": last[(r["problem"], r["tag"])] = r
tags = ["ew_def", "ew_all_before", "ew_all_after", "ew_all_loop", "ew_impl_before", "ew_impl_after", "ew_impl_loop", "ew_all_loop_prr", "ew_impl_loop_prr"]
print("| 問題 | 設定 | 根の LP | 根の下界 | 前処理の秒 | 根までの秒 (求解) | 列挙の回 | 固定 | 締め付け | 同値な組 | clique (衝突の組) | 列挙の秒 | loop の回 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for p in ["net12", "istanbul-no-cutoff", "ns1830653", "mcsched", "seymour1", "swath1", "tbfp-network"]:
    for t in tags:
        r = last.get((p, t))
        if not r: continue
        e = r.get("enum", [])
        s = [sum(x[i] for x in e) for i in range(1, 8)] if e else [0] * 7
        res = r.get("result", ["?"] * 6)
        print(f"| {p} | {t[3:]} | {r.get('root_lp', [None])[0]:.6g} | {float(res[4]):.6g} | {r.get('pre_secs')} | {res[5]} | {len(e)} | {int(s[2])} | {int(s[3])} | {int(s[4])} | {int(s[5])} | {s[6]:.2f} | {r.get('enum_loop', 0)} |")
