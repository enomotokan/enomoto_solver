"""2 つのログの最初に食い違う行を出す (時刻・秒の数を伏せて比べる)。使い方: python firstdiff.py a.log b.log [前後の行数]"""
import re, sys
def norm(l):
    l = re.sub(r"\bt [0-9.]+ ", "t T ", l)
    l = re.sub(r"[0-9]+\.[0-9]+ ?s\b", "Ts", l)
    l = re.sub(r"\b(secs|sec|time|cpu|wall|elapsed|t)[ =:]+[0-9.e+-]+", r"\1 T", l)
    l = re.sub(r"[0-9.]+ (ms|us)\b", r"T \1", l)
    l = re.sub(r'("?[a-z_]*(secs|time|_t|_s)"?\s*[:=]\s*)[0-9.e+-]+', r"\1T", l)
    l = re.sub(r"(outer|solve) [0-9.]+", r"\1 T", l)
    l = re.sub(r"time_limit [0-9.]+", "time_limit T", l)
    l = re.sub(r"^MIP:\s+[0-9.]+s", "MIP: Ts", l)
    l = re.sub(r"([a-z_]*secs) [0-9.]+", r"\1 T", l)
    return l
a = [l.rstrip("\n") for l in open(sys.argv[1])]
b = [l.rstrip("\n") for l in open(sys.argv[2])]
k = int(sys.argv[3]) if len(sys.argv) > 3 else 3
# 経過の行 (時刻で間隔を決めて出す) は除く
skip = lambda l: re.match(r"^MIP:\s+[0-9.]+s nodes", l) is not None
a = [x for x in a if not skip(x)]
b = [x for x in b if not skip(x)]
na, nb = [norm(x) for x in a], [norm(x) for x in b]
for i in range(min(len(na), len(nb))):
    if na[i] != nb[i]:
        print(f"first diff at line {i + 1}")
        for j in range(max(0, i - k), min(i + k + 1, len(a))):
            print("A", j + 1, a[j][:260])
        for j in range(max(0, i - k), min(i + k + 1, len(b))):
            print("B", j + 1, b[j][:260])
        break
