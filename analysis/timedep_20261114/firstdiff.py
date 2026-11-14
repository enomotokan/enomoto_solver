"""2 つのログが最初に分かれる行を出す (秒の表示は消して比べる): python firstdiff.py A.log B.log [前後の行数]"""
import re, sys
def norm(l):
    l = re.sub(r"\d+\.\d+s\b", "S", l)
    l = re.sub(r"(clock|cap|wall|budget|waited|at|probe|in total|cpu|root LP) [\d.]+s?", r"\1 T", l)
    l = re.sub(r"\(\d+\.\d+\)", "(T)", l)
    l = re.sub(r"(secs?\"?[:=]\s*)[\d.]+", r"\1T", l)
    return l
def load(p):
    return [l.rstrip("\n") for l in open(p, errors="replace") if "slope-intercept" not in l and not l.startswith(("TRSTAT", "MIP: MEMCAP", "Running HiGHS", "Includes"))
            and not re.match(r"\s+[\d.]+s\s", l) and not re.match(r"MIP:\s+[\d.]+s nodes", l)]
a, b = load(sys.argv[1]), load(sys.argv[2]); k = int(sys.argv[3]) if len(sys.argv) > 3 else 2
for i, (x, y) in enumerate(zip(a, b)):
    if norm(x) != norm(y):
        for j in range(max(0, i - k), i): print("  ", a[j][:220])
        print("A>", x[:300]); print("B>", y[:300]); break
else:
    print("同じ (短い方の終わりまで)")
