"""res_diag1.jsonl の集計: 問題 × スレッド × 設定ごとに、LP プロービングの最初の LP・子の LP (状態ごとの本数・反復・秒)・固定・全体の秒"""
import json, re, sys, collections
rows = [json.loads(l) for l in open(sys.argv[1] if len(sys.argv) > 1 else "res_diag1.jsonl")]
M_FIRST = re.compile(r"LPPDIAG first: rows (\d+) cols (\d+) nnz (\d+) iters (\d+) secs ([\d.]+) per_lp (\d+)")
M_CH = re.compile(r"LPPDIAG child: col (\d+) v \S+ (up|down) (\w+) iters (\d+) secs ([\d.]+)")
M_TOT = re.compile(r"MIP: LP probing: tried (\d+) columns, fixed (\d+), LP (\S+) -> (\S+) \((\d+) LPs, (\d+) iters, ([\d.]+)s")
M_ROOT = re.compile(r"MIP: LP probing: root LP ([\d.]+)s")
M_STOP = re.compile(r"MIP: LP probing: the first LP stopped: (\w+) \(([\d.]+)s")
print("| 問題 | T | 設定 | 行×列 | 最初の LP 反復・秒 | per_lp | 子 (状態: 本・反復・秒) | 試した列・固定 | LP プロービング秒 | 根の後 tree | 状態・秒・ノード |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    f = ch = tot = None; chs = collections.defaultdict(lambda: [0, 0, 0.0]); stop = None
    for t, l in r["lines"]:
        if (m := M_FIRST.match(l)): f = m.groups()
        elif (m := M_CH.match(l)):
            c = chs[m.group(3)]; c[0] += 1; c[1] += int(m.group(4)); c[2] += float(m.group(5))
        elif (m := M_TOT.match(l)): tot = m.groups()
        elif (m := M_STOP.match(l)): stop = m.groups()
    cfg = "def" if not r["extra"] else "pass0"
    res = r["result"]
    fs = f"{f[3]}・{float(f[4]):.2f}" if f else (f"止まった {stop[0]} {stop[1]}" if stop else "—")
    chs_s = "; ".join(f"{k}: {v[0]}・{v[1]}・{v[2]:.2f}" for k, v in sorted(chs.items())) or "—"
    tr = f"{tot[0]}・{tot[1]}" if tot else "—"
    sec = tot[6] if tot else "—"
    print(f"| {r['problem']} | {r['threads']} | {cfg} | {f[0]+'×'+f[1] if f else '—'} | {fs} | {f[5] if f else '—'} | {chs_s} | {tr} | {sec} | {r['tree_start']} | {res.get('status')}・{res.get('time',0):.1f}・{res.get('nodes')} |")
