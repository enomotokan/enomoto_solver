import re, sys
# ENOMOTO のログから前処理後の大きさ・根の LP・カット後の根・最初の解・結果を 1 行にまとめる
s=open(sys.argv[1]).read()
def g(p, d="-"):
    m=re.search(p, s); return m.group(1) if m else d
pre=g(r"Presolve: [0-9]+ rows, [0-9]+ columns -> ([0-9]+ rows, [0-9]+ columns)")
lp=g(r"MIP: root LP ([-0-9.e+]+)")
cut=g(r"Root cuts: objective [-0-9.e+]+ -> ([-0-9.e+]+)")
inc=re.findall(r"new incumbent ([-0-9.e+]+) at node \d+ \(([0-9.]+)s\) \[([^\]]+)\]", s)
first=f"{float(inc[0][0]):.0f}@{inc[0][1]}s[{inc[0][2]}]" if inc else "none"
r=re.search(r"RESULT (\{.*\}) ([0-9.]+)", s)
res=eval(r.group(1), {"inf": float("inf"), "nan": float("nan")}) if r else {}
print(f"{sys.argv[2]}: pre {pre} | LP {float(lp) if lp!='-' else lp} | cuts {cut} | first {first} | obj {res.get('objective')} | bound {res.get('best_bound')} | root_bound {res.get('root_bound')} | nodes {res.get('nodes')} | status {res.get('status')} | time {float(r.group(2)) if r else '-':.1f}")
