"""res.jsonl (600 秒の 5 問) と res58.jsonl (58 問・60 秒) の表を出す。使い方: python analysis/tree_restart_20261104/summ.py"""
import json, math, re
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent

def fmt(x):
    return "-" if x is None else f"{x:.10g}"

if (D / "res.jsonl").exists():
    print("| 問題 | 設定 | 状態 | 時間 | 下界 | 暫定解 | ノード | 再スタート (根) | 木の中の再スタート (固定数) | 固定不足で見送り |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for l in open(D / "res.jsonl"):
        e = json.loads(l); r = e["result"]
        tree = [re.search(r"TREE restart: (\d+) newly", s).group(1) for s in r.get("restarts_tree", [])]
        err = (D / "logs" / f"{e['cfg']}{'' if e['secs'] == 600 else int(e['secs'])}_{e['problem']}.err").read_text(errors="replace")
        nall = len(re.findall(r"^MIP: restart \d+ \(", err, re.M))
        nroot = nall - len(tree)
        skips = err.count("skipped: newly fixed")
        st = "最適" if r.get("status") in ("optimal", "Optimal") else r.get("status")
        print(f"| {e['problem']} | {e['cfg']} | {st} | {r.get('time', 0):.1f} | {fmt(r.get('bound'))} | {fmt(r.get('objective'))} | {r.get('nodes')} | {nroot} | {len(tree)} ({', '.join(tree)}) | {skips} |")

if (D / "res58.jsonl").exists():
    res = defaultdict(dict)
    for l in open(D / "res58.jsonl"):
        e = json.loads(l)
        res[(e["problem"], e["rep"])][e["cfg"]] = e["result"]
    cfgs = sorted({c for v in res.values() for c in v})
    reps = sorted({k[1] for k in res})
    ok = lambda r: r.get("status") in ("optimal", "Optimal")
    print()
    print("| rep | 設定 | 解けた数 | SGM (ずらし 10 秒) | WARNING |")
    print("|---|---|---|---|---|")
    for rp in reps:
        keys = [k for k in res if k[1] == rp and all(c in res[k] for c in cfgs)]
        for c in cfgs:
            rs = [res[k][c] for k in keys]
            t = [min(r["time"], 60.0) if ok(r) and r.get("time") is not None else 60.0 for r in rs]
            sgm = math.exp(sum(math.log(x + 10) for x in t) / len(t)) - 10
            print(f"| {rp} | {c} | {sum(ok(r) for r in rs)} / {len(rs)} | {sgm:.2f} | {sum(r.get('mip_warnings', 0) for r in rs)} |")
    print()
    print("片方だけ解けた・差の大きいもの:")
    for (p, rp), v in sorted(res.items()):
        if len(v) < len(cfgs):
            continue
        a, b = v[cfgs[0]], v[cfgs[1]]
        if ok(a) != ok(b):
            print(f"  rep {rp} {p}: " + ", ".join(f"{c} {v[c].get('status')} {v[c].get('time') and round(v[c]['time'], 1)}" for c in cfgs))
        elif not ok(a) and (a.get("objective") != b.get("objective")):
            print(f"  rep {rp} {p} (両方時間切れ): " + ", ".join(f"{c} 下界 {fmt(v[c].get('bound'))} 暫定解 {fmt(v[c].get('objective'))}" for c in cfgs))
