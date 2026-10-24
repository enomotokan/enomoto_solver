import json,os,sys,math,collections
d=sys.argv[1]; sub={}
CFGS=['def9','cand9','candsb9']
SETS=['HLq','HLr']
PAIRS=[tuple(x.split(',')) for x in (sys.argv[2:] or ['def7,cand7','mixoc7,mixcand7','def7,mixcand7'])]
for l in open('bench_subsets.txt'):
    t=l.split()
    if t and t[0].isdigit():
        for p in t[1:]: sub.setdefault(p,[]).append(int(t[0]))
R=collections.defaultdict(dict); HI={}
for f in os.listdir(d):
    if '_d20cf31_' not in f or not f.endswith('.json'): continue
    suite='40' if f.startswith('miplib') else '16'
    cfg,s,r=f.split('_d20cf31_')[1][:-5].rsplit('_',2); k=int(s[1:])
    for p,v in json.load(open(f'{d}/{f}')).items():
        if k in sub.get(p,[]): R[(cfg,r[1:])][(suite,p)]=v['ours']; HI[(suite,p)]=v['highs']
keys=sorted(k for k in HI if all(k in R[(c,s)] for c in CFGS for s in SETS))
def pgap(o,ref):
    if o is None or ref is None: return 1.0
    if abs(o-ref)<=1e-9*max(1,abs(ref)): return 0.0
    if o*ref<0: return 1.0
    return abs(o-ref)/max(abs(o),abs(ref))
ref={}
for kk in keys:
    v=[R[c][kk]['objective'] for c in R if kk in R[c] and R[c][kk]['objective'] is not None]
    if HI[kk]['objective'] is not None: v.append(HI[kk]['objective'])
    ref[kk]=min(v) if v else None
sgm=lambda ts,s: math.exp(sum(math.log(t+s) for t in ts)/len(ts))-s
k40=[k for k in keys if k[0]=='40']
print(f"問題 {len(keys)} (40 問のうち {len(k40)})")
print("| 設定 | 組 | 40 問の解けた数 | 40 問の SGM 時間 (shift 10) | 全問の解けた数 | 主ギャップの和 | 解なし | HiGHS が 60 秒で解く問題のうち解けた数 | 不一致 |")
print("|---|---|---|---|---|---|---|---|---|")
hk=[k for k in keys if HI[k]['status']=='Optimal' and HI[k]['time']<60]
for s in SETS:
  for c in CFGS:
    rr=R[(c,s)]
    so=sum(rr[k]['status']=='optimal' for k in k40)
    t=sgm([min(rr[k]['time'],60) if rr[k]['status']=='optimal' else 60 for k in k40],10)
    sa=sum(rr[k]['status']=='optimal' for k in keys)
    pg=sum(pgap(rr[k]['objective'],ref[k]) for k in keys)
    ns=sum(rr[k]['objective'] is None for k in keys)
    hs=sum(rr[k]['status']=='optimal' for k in hk)
    w=[k[1] for k in keys if rr[k]['status']=='optimal' and HI[k]['status']=='Optimal' and abs(rr[k]['objective']-HI[k]['objective'])>1e-6*max(1,abs(HI[k]['objective']))]
    print(f"| {c} | {s} | {so} | {t:.2f} | {sa} | {pg:.3f} | {ns} | {hs}/{len(hk)} | {len(w)} {w or ''} |")
print("\n問題ごと (2 組とも同じ向き: 主ギャップ差 >= 0.01、解けた/解けない、両方解けて時間 1.5 倍)")
for a,b in PAIRS:
  bt,wr=[],[]
  for k in keys:
    S=SETS
    dg=[pgap(R[(b,s)][k]['objective'],ref[k])-pgap(R[(a,s)][k]['objective'],ref[k]) for s in S]
    so=[(R[(b,s)][k]['status']=='optimal')-(R[(a,s)][k]['status']=='optimal') for s in S]
    tt=[(R[(b,s)][k]['time']+1)/(R[(a,s)][k]['time']+1) for s in S]
    bo=all(R[(c,s)][k]['status']=='optimal' for c in (a,b) for s in S)
    tag=f"{k[1]} ({b} {['%s %.1f %s'%(R[(b,s)][k]['status'][:3],R[(b,s)][k]['time'],R[(b,s)][k]['objective']) for s in S]} / {a} {['%s %.1f %s'%(R[(a,s)][k]['status'][:3],R[(a,s)][k]['time'],R[(a,s)][k]['objective']) for s in S]})"
    if all(x<=-0.01 for x in dg) or all(x>0 for x in so) or (bo and all(x<1/1.5 for x in tt)): bt.append(tag)
    elif all(x>=0.01 for x in dg) or all(x<0 for x in so) or (bo and all(x>1.5 for x in tt)): wr.append(tag)
  print(f"- {b} が {a} より良い ({len(bt)}):"); [print("  -",x) for x in bt]
  print(f"- {b} が {a} より悪い ({len(wr)}):"); [print("  -",x) for x in wr]
