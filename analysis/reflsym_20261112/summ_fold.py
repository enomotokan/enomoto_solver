"""前処理の前に畳む (ri) と前処理の後に畳む (riP) の畳み量を比べる (analysis/reflsym_20261112.md)。
python summ_fold.py count_fold.jsonl count_foldP.jsonl"""
import json, re, sys

def load(fn):
    d = {}
    for l in open(fn):
        r = json.loads(l)
        pre = None
        n0 = n1 = None
        for ln in r["fold"]:
            m = re.search(r"presolved before folding: n (\d+) -> (\d+)", ln)
            if m:
                pre = int(m.group(2))
            m = re.search(r"MIP_FOLD: n (\d+) -> (\d+), m (\d+) -> (\d+)", ln)
            if m:
                n0, n1 = int(m.group(1)), int(m.group(2))
        base = pre if pre is not None else None
        d[r["name"]] = (n0, n1, pre, r["fold"])
    return d

a, b = load(sys.argv[1]), load(sys.argv[2])
fa = {k for k, v in a.items() if v[1] is not None and v[1] < v[0]}
fb = {k for k, v in b.items() if v[1] is not None and v[1] < v[0]}
print(f"問題 {len(set(a) & set(b))}、畳めた: 前 {len(fa)}、後 {len(fb)}、両方 {len(fa & fb)}、前だけ {len(fa - fb)}、後だけ {len(fb - fa)}")
print("問題 | 前: n -> 畳んだ n (減り) | 後: 前処理の n -> 畳んだ n (減り)")
for k in sorted(fa | fb):
    va, vb = a.get(k, (None,) * 4), b.get(k, (None,) * 4)
    def s(v):
        if v[1] is None or v[1] >= (v[0] or 0):
            return "なし"
        return f"{v[0]} -> {v[1]} ({100 * (v[0] - v[1]) / v[0]:.1f}%)"
    print(k, "|", s(va), "|", s(vb))
