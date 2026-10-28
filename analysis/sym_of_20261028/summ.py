import json, math, collections, sys
sol={}
for l in open('miplib.solu'):
    p=l.split()
    if len(p)>=3 and p[0] in('=opt=','=best='): sol[p[1]]=float(p[2])
R=collections.defaultdict(dict)
for l in open(sys.argv[1]):
    d=json.loads(l); R[d['problem']][(d['cfg'],d['rep'])]=d['result']
def t(o): return o['time'] if o.get('status')=='optimal' else 60.0
def gap(o):
    ob,bd=o.get('objective'),o.get('best_bound')
    if ob is None or bd is None: return 1.0
    return abs(ob-bd)/max(abs(ob),abs(bd),1e-9)
bad=0; sg={'base':[], 'of':[]}
for p,m in R.items():
    row=[]
    for c in('base','of'):
        for r in(0,1):
            o=m.get((c,r),{}); sg[c].append(t(o))
            if o.get('status')=='optimal' and p in sol and abs(o['objective']-sol[p])>1e-6*max(1,abs(sol[p])): bad+=1; print('MISMATCH',p,c,o['objective'],sol[p])
            row.append(f"{c}{r}:{'%.1f'%o['time'] if o.get('status')=='optimal' else 'TL/gap%.3f'%gap(o)}")
    print(f"{p:20s} "+"  ".join(row))
for c in sg: print(c,'SGM %.2f'%(math.exp(sum(math.log(x+10) for x in sg[c])/len(sg[c]))-10), 'solved', sum(x<60 for x in sg[c]))
print('mismatch',bad)
