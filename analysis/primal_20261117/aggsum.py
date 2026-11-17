"""組み立てた HiGHS (highs_agglog.patch) の AGGLOG 行を集計する: python aggsum.py agglog.txt"""
import sys, collections
L=[]
for ln in open(sys.argv[1]):
    if not ln.startswith('AGGLOG'): continue
    t=ln.split()
    d=dict(row=int(t[2]),col=int(t[4]),orow=int(t[6]),ocol=int(t[8]),eq=int(t[10]),rs=int(t[12]),cs=int(t[14]),int=int(t[16]),
           lb=float(t[18]),ub=float(t[20]),il=float(t[22]),iu=float(t[23]),rlo=float(t[25]),rup=float(t[27]),dl=float(t[29]),du=float(t[30]),cost=float(t[32]))
    d['i']=len(L); L.append(d)
print('total',len(L))
c=collections.Counter((d['eq'],d['int']) for d in L); print('(eq,int)',dict(c))
for k in [(0,0),(1,0),(0,1),(1,1)]:
    S=[d for d in L if (d['eq'],d['int'])==k]
    if not S: continue
    print(k,'rowsize',collections.Counter(d['rs'] for d in S).most_common(6),'colsize',collections.Counter(d['cs'] for d in S).most_common(6))
    print('  bounds',collections.Counter((d['lb'],d['ub']) for d in S).most_common(5))
    print('  impl',collections.Counter((d['il'],d['iu']) for d in S).most_common(5))
    print('  rowlo/up',collections.Counter((d['rlo'],d['rup']) for d in S).most_common(5))
    print('  dual',collections.Counter((d['dl'],d['du']) for d in S).most_common(5), 'cost', collections.Counter(d['cost'] for d in S).most_common(3))
    print('  first idx',[d['i'] for d in S[:10]])
cont=[d for d in L if d['int']==0]
print('cont ocol distinct',len(set(d['ocol'] for d in cont)))
print('first cont subs:')
for d in cont[:15]: print(' ',{k:d[k] for k in ['i','orow','ocol','eq','rs','cs','lb','ub','il','iu','rlo','rup','dl','du','cost']})
