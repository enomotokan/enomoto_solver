"""担当 A の集計: res.jsonl とログ (logs/<設定>_<問題>_s<種>.log[.gz]) から、問題ごと・設定ごとに
状態・秒・暫定解・下界・ギャップ・最初の解・根の後の暫定解・暫定解の推移・サブ MIP の本数と秒 (ログの `sub-MIP:` 行) を出す。
使い方: python summ.py [問題,...]"""
import gzip, json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
want = set(sys.argv[1].split(",")) if len(sys.argv) > 1 else None


def readlog(tag, p, seed):
    for f in (D / "logs" / f"{tag}_{p}_s{seed}.log", D / "logs" / f"{tag}_{p}_s{seed}.log.gz"):
        if f.exists():
            return (gzip.open(f, "rt", errors="replace") if f.suffix == ".gz" else open(f, errors="replace")).read()
    return ""


def fmt(v):
    if v is None:
        return "-"
    return f"{v:.6g}"


rows = defaultdict(list)
for l in open(D / "res.jsonl"):
    r = json.loads(l)
    if want and r["problem"] not in want:
        continue
    rows[r["problem"]].append(r)
for p, rs in rows.items():
    print(f"## {p}\n")
    print("| 設定 | 種 | 状態 | 秒 | 暫定解 | 下界 | ギャップ | 最初の解 (秒) | 根の後 | 暫定解の推移 (秒:値) | サブ MIP 本数・秒 | 捨てた解 (SUBA_CUT) |")
    print("|---|---|---|---:|---:|---:|---:|---|---:|---|---|---:|")
    for r in sorted(rs, key=lambda r: (r["tag"] != "def", r["tag"], r["seed"])):
        res = r["result"]
        log = readlog(r["tag"], p, r["seed"])
        incs = [(float(t), float(z)) for z, t in re.findall(r"new incumbent (\S+) at node \d+ \(([\d.]+)s\)", log)]
        root = re.findall(r"after root heuristics: incumbent Some\(([^)]+)\)", log)
        subs = [float(s) for s in re.findall(r"(?m)(?:sub-MIP|parallel sub-MIP \([^)]*\)): (?:\d+ free|status).*?([\d.]+)s\s*$", log)]
        rej = re.findall(r"SUBA_CUT rejected (\d+)", log)
        z, b = res.get("objective"), res.get("bound")
        gap = abs(z - b) / max(abs(z), 1e-9) if z is not None and b is not None else None
        traj = " ".join(f"{t:.0f}:{v:.6g}" for t, v in incs[:1] + incs[1:][-6:]) if incs else "-"
        if len(incs) > 7:
            traj = traj.replace(" ", " … ", 1)
        fs = res.get("first_solution_time")
        print(f"| {r['tag']} | {r['seed']} | {res.get('status')} | {res.get('time') or 0:.1f} | {fmt(z)} | {fmt(b)} | {fmt(gap)} | "
              f"{fmt(res.get('first_solution_objective'))} ({fs or 0:.1f}) | {fmt(float(root[0])) if root else '-'} | {traj} | {len(subs)}・{sum(subs):.0f} | {rej[0] if rej else '-'} |")
    print()
