"""res の jsonl を問題 × 設定の表にする (analysis/intvub_20261106.md)。使い方: python summ.py ファイル [設定 ...]
列: 状態・時間・目的値・下界・根 (最初のカットのループの LP の値 前 -> 後、足したカットの数、ラウンド数、時刻)。INTVUB の行の数も出す。"""
import json, sys, collections
f = sys.argv[1]
R = [json.loads(l) for l in open(f)]
cfgs = sys.argv[2:] or sorted({r["cfg"] for r in R})
by = collections.defaultdict(list)
for r in R:
    by[(r["problem"], r["cfg"])].append(r)
probs = list(dict.fromkeys(r["problem"] for r in R))
def fmt(v, nd=6):
    return "-" if v is None else (f"{v:.{nd}g}" if isinstance(v, float) else str(v))
print("| 問題 | 設定 | 回 | 状態 | 時間 | 目的値 | 下界 | 根: LP 前 → 後 (カット/ラウンド, 秒) | INTVUB |")
print("|---|---|---|---|---|---|---|---|---|")
for p in probs:
    for c in cfgs:
        for r in by.get((p, c), []):
            res = r["result"]; lp = r["root_loops"][0] if r.get("root_loops") else None
            root = f"{fmt(lp['obj0'])} → {fmt(lp['obj1'])} ({lp['added']}/{lp['rounds']}, {fmt(lp['t'], 3)})" if lp else "-"
            iv = r["intvub"][0].replace("INTVUB: ", "") if r.get("intvub") else ""
            st = res.get("status", "?").replace("time_limit", "TL")
            print(f"| {p} | {c} | {r['rep']} | {st} | {fmt(res.get('time'), 4)} | {fmt(res.get('objective'), 8)} | {fmt(res.get('bound'), 8)} | {root} | {iv} |")
