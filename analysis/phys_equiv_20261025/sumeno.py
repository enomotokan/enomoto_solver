"""sumeno.py <ログ>...: ENOMOTO のログから前処理の時間・延長・根の LP・カットのラウンド (値@時刻)・再スタート・暫定解・結果"""
import sys, re, json
for f in sys.argv[1:]:
    s = open(f).read()
    ext = re.findall(r'clique extension: (\d+) rows extended, (\d+) nonzeros', s)
    lp = re.findall(r'MIP: root LP ([0-9.e+]+) \(\d+ iters, ([0-9.]+)s\)', s)
    rounds = re.findall(r'cut round (\d+): .*?obj ([0-9.e+]+) \(([0-9.]+)s\)', s)
    done = re.findall(r'cut loop done: .*?-> ([0-9.e+]+)', s)
    rs = re.findall(r'restart (\d+) \((\d+) of .*?\) \(([0-9.]+)s\)', s)
    inc = re.findall(r'new incumbent ([0-9.e+]+) at node \d+ \(([0-9.]+)s\) \[(.*?)\]', s)
    res = json.loads(s.strip().splitlines()[-1])
    print(f"{f}: {res['status']} {res['objective']} bb {res['best_bound']:.1f} t {res['time']:.1f}  ext {ext[:1]}  rootLP {[(float(a), float(b)) for a, b in lp]}")
    print("   rounds:", ' '.join(f"{float(v):.0f}@{float(t):.1f}" for _, v, t in rounds[:12]), "| done", [f"{float(d):.0f}" for d in done], "| restarts", rs, "| inc", [(f"{float(a):.0f}", b, c) for a, b, c in inc][-3:])
