"""bench58_s*.jsonl の集計 (python3 summ58.py): 種ごとの解けた数・シフト 1 の幾何平均・主のギャップの平均と、試作が働いた問題
(ログ logs/<問題>_both_s<種>.log.gz の `root LP both` の行) の時間・根の下界・根の終わりの時刻 (`after root heuristics`)・待ち。"""
import gzip, json, math, os, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sg = lambda ts: math.exp(sum(math.log(t + 1) for t in ts) / len(ts)) - 1


def gap(r):
    z, b = r.get("objective"), r.get("bound")
    if r.get("status") in ("optimal", "infeasible"):
        return 0.0
    if z is None or b is None or not math.isfinite(b):
        return 1.0
    return min(1.0, abs(z - b) / max(abs(z), abs(b), 1e-9)) if z != b else 0.0


def loginfo(p, c, s):
    fn = D / "logs" / f"{p}_{c}_s{s}.log.gz"
    if not fn.exists():
        return {}
    t = gzip.open(fn, "rt", errors="replace").read()
    m = re.findall(r"after root heuristics: .*\(([\d.]+)s\)", t)
    info = {"root_end": float(m[0]) if m else None}
    lost = re.search(r"root LP both: the (\w+) side lost at ([\d.]+)s", t)
    if lost:
        info["lost"] = lost.group(1)
        info["win"] = float(lost.group(2))
        w = re.search(r"second vertex from the \w+ side at ([\d.]+)s \(waited ([\d.]+)s\).*?, (\d+) basic variables differ", t)
        info["waited"] = float(w.group(2)) if w else None
        info["arrive"] = float(w.group(1)) if w else None
        nov = re.search(r"no second vertex from the \w+ side \(waited ([\d.]+)s\)", t)
        if nov:
            info["waited"] = float(nov.group(1)); info["dec"] = "届かない"
        d = re.search(r"probe gain at the current vertex (\S+?)( \(flat cut loop\))?, at the second vertex (\S+) \(.*?, ([\d.]+)s\) -> (.*)", t)
        if d:
            info["dec"] = {"cut loop from the second vertex": "2 つ目", "back to the first vertex": "元"}.get(d.group(5), d.group(5))
            info["gA"], info["gB"] = float(d.group(1)), float(d.group(3))
        elif "dec" not in info:
            info["dec"] = "比べなし"
    return info


for s in (1, 2):
    f = D / f"bench58_s{s}.jsonl"
    if not f.exists():
        continue
    R = {}
    for l in open(f):
        d = json.loads(l); r = d["result"]
        ok = r.get("status") in ("optimal", "infeasible")
        R[(d["problem"], d["cfg"])] = (r, r.get("time", 60) if ok else 60.0, ok)
    probs = sorted({p for p, c in R if (p, "def") in R and (p, "both") in R})
    print(f"## 種 {s} ({len(probs)} 問)")
    for c in ("def", "both"):
        print(c, "解けた数", sum(R[(p, c)][2] for p in probs), "SGM %.2f" % sg([R[(p, c)][1] for p in probs]),
              "主ギャップの平均 %.4f" % (sum(gap(R[(p, c)][0]) for p in probs) / max(1, len(probs))))
    print("| 問題 | 既定 (秒) | 試作 (秒) | 既定の根の下界 | 試作の根の下界 | 根の終わり 既定→試作 (秒) | 負け | 届くまで / 待ち (秒) | 判断 (伸び 今 / 2 つ目) |")
    print("|---|---:|---:|---:|---:|---|---|---|---|")
    nw = {"lost": 0, "2 つ目": 0}
    for p in probs:
        a, b = R[(p, "def")], R[(p, "both")]
        ia, ib = loginfo(p, "def", s), loginfo(p, "both", s)
        if "lost" in ib:
            nw["lost"] += 1
        if ib.get("dec") == "2 つ目":
            nw["2 つ目"] += 1
        big = a[2] != b[2] or abs(a[1] - b[1]) > max(3, 0.25 * min(a[1], b[1])) or abs(gap(a[0]) - gap(b[0])) > 0.01
        if "lost" in ib or big:
            g = lambda x: f"{x[1]:.1f}" if x[2] else f"時間切れ ({gap(x[0])*100:.2f}%)"
            ra, rb = a[0].get("root_bound"), b[0].get("root_bound")
            fr = lambda v: "-" if v is None else f"{v:.6g}"
            re_ = f"{fr(ia.get('root_end'))} → {fr(ib.get('root_end'))}"
            wa = f"{fr(ib.get('arrive'))} / {fr(ib.get('waited'))}" if "lost" in ib else "-"
            dec = ib.get("dec", "-") + (f" ({ib['gA']:.3g} / {ib['gB']:.3g})" if "gA" in ib else "")
            print(f"| {p} | {g(a)} | {g(b)} | {fr(ra)} | {fr(rb)} | {re_} | {ib.get('lost', '-')} | {wa} | {dec} |")
    print(f"負けた側が裏で続いた問題 {nw['lost']}、2 つ目の頂点からカットのループを始めた問題 {nw['2 つ目']}")
    print()
