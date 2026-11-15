import json,sys,math,csv
S=sys.argv[1]; fs=sys.argv[2:]
ref={}
for r in csv.DictReader(open('/home/user/enomoto_solver/benchmarks/paper/per_problem.csv')):
    if r['set']=='netlib' and r['reference_obj']: ref[r['problem']]=float(r['reference_obj'])
D=[{json.loads(l)['p']:json.loads(l) for l in open(f'{S}/{f}')} for f in fs]
names=sorted(D[0])
print('問題 | '+' | '.join(f'{f}: m/det/sb/受理/時間' for f in fs))
agg=[[0,0,0,0,0.0,0] for _ in fs]
for p in names:
    row=[]
    for k,d in enumerate(D):
        r=d.get(p,{})
        ok = r.get('st')=='optimal' and p in ref and abs(r['obj']-ref[p])<=1e-6*max(1,abs(ref[p]))
        if r.get('st')=='optimal' and p not in ref: ok=True
        a=agg[k]; a[0]+=r.get('accepted',False); a[1]+=r.get('sb',0); a[2]+=ok; a[3]+=1
        t=r.get('t',300); a[4]+=math.log(t+1); a[5]+=t
        row.append(f"{r.get('m','-')}/{r.get('det','-')}/{r.get('sb','-')}/{'Y' if r.get('accepted') else 'n'}/{t:.3f}{'' if ok else ' NG:'+str(r.get('st'))}")
    if len(set(x.split('/')[2]+x.split('/')[3] for x in row))>1 or any('NG' in x for x in row) or max(D[k].get(p,{}).get('t',0) for k in range(len(D)))>0.3:
        print(p,'|',' | '.join(row))
for f,a in zip(fs,agg):
    print(f'{f}: 受理 {a[0]}, 超基底の合計 {a[1]}, 正解 {a[2]}/{a[3]}, 1 秒ずらした幾何平均 {math.exp(a[4]/a[3])-1:.4f}, 合計 {a[5]:.2f} 秒')
