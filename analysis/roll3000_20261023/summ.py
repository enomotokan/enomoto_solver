# summ.py <ログ>... : 根のカット後の値・再スタート・木の開始・各時刻の下界と上界・最適値 12890 を見つけた時刻
import re, sys
for f in sys.argv[1:]:
    t = open(f).read()
    roots = [float(x) for x in re.findall(r"Root cuts: objective \S+ -> (\S+),", t)]
    rs = re.findall(r"MIP: restart 1 .*\((\S+)s\)", t)
    tree = re.findall(r"tree search with \d+ threads \((\S+)s\)", t)
    pts = [(float(a), float(b), float(c)) for a, b, c in re.findall(r"MIP:\s+(\S+)s nodes\s+\d+ open\s+\d+ lb (\S+) ub (\S+)", t)]
    def at(s):
        p = [x for x in pts if x[0] <= s]
        return f"{p[-1][1]:.0f}" if p else "-"
    inc = re.findall(r"new incumbent 1\.2890000000e4 .*\((\S+)s\) \[(.*)\]", t)
    j = re.findall(r'\{"status".*', t)
    import json
    r = json.loads(j[-1]) if j else {}
    print(f"{f.split('/')[-1]:28s} 根 {','.join(f'{x:.0f}' for x in roots):14s} 再{rs[0] if rs else '-':>6s} 木{tree[0] if tree else '-':>6s} "
          f"lb20 {at(20)} lb40 {at(40)} | 12890 {inc[0][0]+'s '+inc[0][1] if inc else '-':30s} | {r.get('status','?')[:9]} {r.get('objective',0):.0f} {r.get('best_bound',0):.0f} {r.get('time',0):.1f}s n{r.get('nodes')}")
