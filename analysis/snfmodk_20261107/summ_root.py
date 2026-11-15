"""根だけの比べの集計: python summ_root.py 入力.jsonl 設定,... → 問題ごとの根の下界 (最初の根のループの obj1) と SNF の記録の要約"""
import gzip, json, re, sys
from collections import defaultdict, Counter
from pathlib import Path
D = Path(__file__).resolve().parent
inp, cfgs = sys.argv[1], sys.argv[2].split(",")
R = defaultdict(dict)
for l in open(D / inp):
    r = json.loads(l)
    R[r["problem"]][(r["cfg"], r["rep"])] = r
def rootb(r):
    lp = r.get("root_loops") or []
    return lp[0]["obj1"] if lp else None
def snfinfo(p, c, rep, tl=60):
    f = D / "logs" / f"{c}_{p}_{tl}_r{rep}_root.err.gz"
    if not f.exists():
        return None
    divs = Counter(); calls = 0; cp = cn = 0; maxrows = 0; gave = Counter(); effn = effp = 0.0; t = 0.0
    for l in gzip.open(f, "rt"):
        if l.startswith("SEP snfmodk"):
            calls += 1
            m = re.search(r"rows (\d+) cols (\d+).*gaveup (\d) divs (\S+) elems (\d+) cuts_p (\d+) cuts_new (\d+) eff_p (\S+) eff_new (\S+)", l)
            maxrows = max(maxrows, int(m.group(1))); gave[m.group(3)] += 1
            cp += int(m.group(6)); cn += int(m.group(7)); effp = max(effp, float(m.group(8))); effn = max(effn, float(m.group(9)))
            t += float(re.search(r" ([\d.]+)s ", l).group(1))
            if m.group(4) != "-" and calls <= 50:
                for kv in m.group(4).split("#")[0].split(","):
                    q, c_ = kv.split(":"); divs[int(q)] += int(c_)
        if l.startswith("SEP modkvb") and " snf " in l:
            s = l.split(" snf ")[1].strip().split("/")[0].split("#")[0]
            if s not in ("-", ""):
                for kv in s.split(","):
                    q, c_ = kv.split(":"); divs[("v", int(q))] += int(c_)
    return dict(calls=calls, divs=divs, cuts_p=cp, cuts_new=cn, maxrows=maxrows, gave=dict(gave), effp=effp, effn=effn, t=t)
def cls(q):
    # 素数べき q を 2,3,5,7 の素数 / その高いべき / ほかの素数 に分ける
    for pr in (2, 3, 5, 7):
        x = q
        k = 0
        while x % pr == 0:
            x //= pr; k += 1
        if x == 1:
            return "p" if k == 1 else "pp"
    return "other"
if __name__ == "__main__":
    print("問題 | " + " | ".join(cfgs) + " | snf: 呼び | 因子 (素数 2357 / そのべき / ほか) | カット (p/新) | 効き目の最大 (p/新) | 時間")
    for p in R:
        bs = []
        for c in cfgs:
            r = R[p].get((c, 1))
            b = rootb(r) if r else None
            bs.append("-" if b is None else f"{b:.6g}")
        si = snfinfo(p, "snf", 1)
        if si:
            cc = Counter()
            for q, v in si["divs"].items():
                if isinstance(q, int):
                    cc[cls(q)] += v
            extra = f"{si['calls']} | {cc['p']}/{cc['pp']}/{cc['other']} | {si['cuts_p']}/{si['cuts_new']} | {si['effp']:.3f}/{si['effn']:.3f} | {si['t']:.2f}"
        else:
            extra = "-"
        print(p, "|", " | ".join(bs), "|", extra)
