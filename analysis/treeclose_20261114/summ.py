"""TREECLOSE の表: python summ.py [res_*.jsonl ...]  (logs/ のログを parse.py で読み、問題ごとに ENOMOTO と HiGHS を並べる)。"""
import glob, json, re, sys
from pathlib import Path
import parse

D = Path(__file__).resolve().parent
files = sys.argv[1:] or sorted(glob.glob(str(D / "res_plan_s*.jsonl")))
res = {}
for f in files:
    for l in open(f):
        r = json.loads(l)
        res[(r["problem"], r["cfg"])] = r
probs = sorted({p for p, _ in res})


def rgap(lb, ub):
    if lb is None or ub is None or lb != lb or abs(ub) == float("inf") or abs(lb) == float("inf"):
        return "-"
    return f"{(ub - lb) / max(abs(ub), 1e-9):.1e}"


def num(pat, s, k=1, cast=float):
    m = re.search(pat, s or "")
    return cast(m.group(k)) if m else None


for p in probs:
    print(f"## {p}")
    out = {}
    for c in ("cand", "highs"):
        r = res.get((p, c))
        lf = D / "logs" / f"{p}_{c}.log.gz"
        if not r or not lf.exists():
            continue
        d = parse.highs(str(lf)) if c == "highs" else parse.eno(str(lf))
        out[c] = (r, d)
    if len(out) < 2:
        print("  (片方なし)")
        continue
    # 最終の暫定解 (両方の良い方) で暫定解の時刻を見る
    zbest = min(min((z for _, z, _ in d["inc"]), default=float("inf")) for _, d in out.values())
    for c, (r, d) in out.items():
        rr = r["result"]
        pts = d["pts"]
        ck = " ".join(f"{t}s:{rgap(*(parse.at(pts, t) or (0, None, None))[1:3])}" for t in (30, 60, 120, 240, 400, 600))
        tbest = next((t for t, z, _ in d["inc"] if z <= zbest + 1e-6 * max(1, abs(zbest))), None)
        print(f"  {c:5} {rr.get('status')} {rr.get('time', 0):.0f}s nodes {rr.get('nodes')} restarts {d['restarts']} best-inc-at {tbest} | gap {ck}")
        st = d["stats"]
        if c == "highs":
            tot = int(st.get("LP iterations", "0"))
            sp = [int(x.split()[0]) for x in st.get("lp_split", "").split(" / ") if x.strip() and x.split()[0].isdigit()]
            n = max(int(st.get("Nodes", "1")), 1)
            if len(sp) == 3:
                print(f"        LP iters {tot} (sb {sp[0]} sep {sp[1]} heur {sp[2]}; node {(tot - sum(sp)) / n:.1f}/node) submip {st.get('submip')}")
        else:
            print(f"        node_diag {st.get('node_diag')}")
            print(f"        branching {st.get('branching')}")
            print(f"        threads {st.get('threads')}")
            print(f"        restores {st.get('restores')}")
            for pr in st.get("prof", []):
                print(f"        {pr[:260]}")
