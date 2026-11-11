"""IPMRC (analysis/ipmrc_20261111.md) の集計。使い方: python summ.py A.jsonl [B.jsonl ...]
根の終わり (再スタート・木に入る直前) の XOLAZY の行の固定数 (最適な双対 fix_opt・内点法 fix_ipm・和集合 fix_union・途中の基底 fix_simplex) と、
内点法の点を待った秒 (t_wait) を集める。固定数は根の LP の時点の大域的な境界と根の終わりの打ち切り値で数えたもの (実際の固定 newly とは別)。"""
import json, sys

rows = [json.loads(l) for f in sys.argv[1:] for l in open(f) if l.strip()]
ev_all = []
tot_time = 0.0
for r in rows:
    tot_time += r["result"].get("time") or 0.0
    ev = r.get("xolazy", [])
    last_root = None
    for e in ev:
        if e["ev"] == "rootlp":
            last_root = e
        elif e["ev"] in ("restart", "tree") and "fix_opt" in e:
            ev_all.append((r, e, last_root))
n = len(ev_all)
s = lambda k: sum(e.get(k) or 0 for _, e, _ in ev_all)
has_ipm = [x for x in ev_all if x[1].get("gap_ipm") is not None]
print(f"回 {len(rows)}、時間の和 {tot_time:.1f} 秒、固定を数えた根の終わり {n} (内点法の双対あり {len(has_ipm)})")
print(f"固定数の和: 最適な双対 {s('fix_opt')}・内点法 {s('fix_ipm')}・和集合 {s('fix_union')}・途中の基底 {s('fix_simplex')}・実際の新しい固定 {s('newly')}")
print(f"実際の固定に足される分 (今は固定されていない列): 最適な双対 {s('fix_opt_new')}・内点法 {s('fix_ipm_new')}・途中の基底 {s('fix_simplex_new')}")
tw = [x[2].get("t_wait") for x in ev_all if x[2] is not None and x[2].get("t_wait") is not None]
all_tw = [e.get("t_wait") for r in rows for e in r.get("xolazy", []) if e["ev"] == "rootlp" and e.get("t_wait") is not None]
print(f"内点法の点を待った秒: 根の LP {len(all_tw)} 本で和 {sum(all_tw):.2f} 秒 (最大 {max(all_tw, default=0):.2f})、全体の {100 * sum(all_tw) / max(tot_time, 1e-9):.3f}%")
nw = sum(1 for x in ev_all if x[2] is not None and x[2].get("winner") == "ipm")
print(f"内点法が勝った根の LP の根の終わり {nw}")
print("\n和集合が最適な双対より多い根の終わり:")
print("| 問題 | 種 | 出来事 | 再スタート回 | 勝った側 | 根の LP 秒 | 待った秒 | gap 内点法 | 最適 | 内点法 | 和集合 | 途中の基底 | 実際の newly | 内点法で足される | 列 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
f2 = lambda v: "-" if v is None else (f"{v:.2f}" if isinstance(v, float) else str(v))
for r, e, lr in sorted(ev_all, key=lambda x: -((x[1].get("fix_union") or 0) - x[1]["fix_opt"])):
    if (e.get("fix_union") or 0) <= e["fix_opt"]:
        continue
    print(f"| {r['problem']} | {r['rep']} | {e['ev']} | {e['restarts']} | {lr['winner'] if lr else '-'} | {lr['secs'] if lr else 0:.2f} | {f2(lr.get('t_wait') if lr else None)} | "
          f"{e.get('gap_ipm') if e.get('gap_ipm') is None else format(e['gap_ipm'], '.1e')} | {e['fix_opt']} | {e.get('fix_ipm')} | {e.get('fix_union')} | {e.get('fix_simplex')} | {e.get('newly')} | {e.get('fix_ipm_new')} | {lr['cols'] if lr else '-'} |")
