import sys
OFF=475112.25418842083; CUT=211913.0001
target=int(sys.argv[1])
hp={}
for l in open('hdet.err',errors='replace'):
    if l.startswith('HP '):
        f=l.split()
        path=[x for x in f[10:] if not x.startswith('#')]
        tag=[x for x in f if x.startswith('#B')][0]
        cnt=dict((kv[0],int(kv[1:])) for kv in tag[1:].split(','))
        hp[int(f[1])]=(int(f[2]),int(f[3]),float(f[4]),tuple(path),cnt)
er={}
for l in open('er_det.log',errors='replace'):
    if l.startswith('ER '):
        f=l.split(); tag=[x for x in f if x.startswith('#B')][0]
        cnt=dict((kv[0],int(kv[1:])) for kv in tag[1:].split(','))
        er[int(f[2])]=(f[4],float(f[6]) if f[6]!='NaN' else float('nan'),cnt)
tp=hp[target][3]
anc=sorted([s for s,v in hp.items() if s<=target and len(v[3])<=len(tp) and tp[:len(v[3])]==v[3]], key=lambda s:len(hp[s][3]))
seen=set()
for s in anc:
    d,res,lp,path,c=hp[s]
    if len(path) in seen: continue
    seen.add(len(path))
    o=er.get(s)
    print('seq %5d depth %2d HiGHS lp-gap %8.1f res %d L %3d Q %3d | ours %s' % (s,len(path),CUT-(lp+OFF),res,c.get('L',0),c.get('Q',0), ('out %s gap %8.1f L %d R %d B %d' % (o[0],CUT-o[1],o[2].get('L',0),o[2].get('R',0),o[2].get('B',0))) if o else '-'))
