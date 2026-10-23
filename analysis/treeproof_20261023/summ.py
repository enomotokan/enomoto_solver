"""ログ 1 本の要約: python3 summ.py <log>...
再スタートで時刻が 0 に戻るので、"Restart k: ... (T s)" の T を足して通しの時刻にする。
出す項目: 状態・時間・ノード・最適値の暫定解の時刻と出どころ・木の開始時刻・根の下界 (Root cuts と背景カット後)・再スタート回数・強いカットの判定"""
import re, sys, json
for f in sys.argv[1:]:
    off = 0.0; nres = 0; tree = None; inc = []; rootcuts = []; bg = []; strong = 0
    last = None
    for l in open(f):
        m = re.match(r"Restart \d+: .*\((\d+\.\d+)s\)", l)
        if m:
            off += float(m.group(1)); nres += 1; continue
        m = re.search(r"new incumbent (\S+) at node (\d+) \((\d+\.\d+)s\) \[(.*)\]", l)
        if m and m.group(4) != 'tree' or (m and m.group(2) != '0'):
            inc.append((float(m.group(1)), off + float(m.group(3)), m.group(4)))
        m = re.search(r"tree search with \d+ threads \((\d+\.\d+)s\)", l)
        if m: tree = off + float(m.group(1))
        m = re.search(r"Root cuts: objective \S+ -> (\S+),", l)
        if m: rootcuts.append(float(m.group(1)))
        m = re.search(r"background cuts: \d+ rounds, .*obj \S+ -> (\S+) \(", l)
        if m: bg.append(float(m.group(1)))
        if 'strong cuts at the round limit' in l: strong += 1
        if l.startswith('{'): last = json.loads(l)
    best = min(inc)[0] if inc else None
    first_best = next(((t, s) for v, t, s in inc if abs(v - best) <= 1e-6 * max(1, abs(best))), None)
    st = last['status'] if last else '?'
    print(f"{f.split('/')[-1]:34s} {st:10s} t={last['time'] if last else 0:5.1f} nodes={last['nodes'] if last else 0:5d} "
          f"obj={last['objective'] if last else None} optfound={first_best[0]:.1f}s[{first_best[1]}] tree@{tree if tree else 0:.1f} "
          f"restarts={nres} rootcuts={[round(x,2) for x in rootcuts]} bg={[round(x,2) for x in bg]} strong={strong} rb={last['root_bound'] if last else 0:.6g}")
