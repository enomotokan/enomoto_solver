# 使い方: python summ.py 接頭辞 (raw/接頭辞_設定_rN.out を設定ごとにまとめる)
import json, sys, glob, re, statistics, collections
d = collections.defaultdict(list)
for f in sorted(glob.glob(f'/home/user/enomoto_solver/analysis/qiu_margin_20261104/raw/{sys.argv[1]}_*_r*.out')):
    m = re.search(rf'{sys.argv[1]}_(.+)_r(\d+)\.out', f)
    try:
        j = json.loads([l for l in open(f) if l.startswith('{')][-1])
    except Exception:
        continue
    ok = j['status'] == 'optimal'
    d[m.group(1)].append((ok, j['time'] if ok else 60.0, j['nodes'], j['bound']))
for k, v in d.items():
    print(f"{k:8s} solved {sum(o for o,*_ in v)}/{len(v)} median {statistics.median(t for _,t,_,_ in v):.1f}  times {' '.join(f'{t:.1f}' for _,t,_,_ in v)}  nodes {' '.join(str(n) for *_,n,_ in v)}")
