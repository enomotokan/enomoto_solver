"""seed 3470 の問題を MPS に書き出し、HiGHS の解の実行可能性を確かめる (python3 repro.py)。"""
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../scripts'))
sys.argv=[sys.argv[0],'0']
import importlib.util
spec=importlib.util.spec_from_file_location('fz',os.path.join(os.path.dirname(__file__),'../../scripts/mip_fuzz_vs_highs.py'))
src=open(spec.origin).read().split("N=int(sys.argv[1])")[0]
g={}; exec(src,g)
import highspy
inf=float('inf')
lp=g['gen'](3470)
lo,up,c,it,rows,rlo,rup=lp
h=highspy.Highs(); h.setOptionValue('output_flag',False); h.setOptionValue('mip_rel_gap',1e-9)
K=highspy.kHighsInf
for j in range(len(lo)):
    h.addVar(lo[j] if lo[j]>-inf else -K, up[j] if up[j]<inf else K); h.changeColCost(j,c[j])
    if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
for r,l,u in zip(rows,rlo,rup): h.addRow(l if l>-inf else -K,u if u<inf else K,len(r),[a for a,_ in r],[b for _,b in r])
h.writeModel(os.path.join(os.path.dirname(__file__),'seed3470.mps'))
h.run(); x=list(h.getSolution().col_value)
print(h.modelStatusToString(h.getModelStatus()), h.getInfo().objective_function_value)
print("x =",[round(v,6) for v in x])
for j in range(len(lo)):
    print(f"x{j}: [{lo[j]},{up[j]}] int={it[j]} c={c[j]} val={x[j]:.9g}")
    assert lo[j]-1e-9<=x[j]<=up[j]+1e-9
    if it[j]: assert abs(x[j]-round(x[j]))<1e-9, j
for i,(r,l,u) in enumerate(zip(rows,rlo,rup)):
    a=sum(v*x[j] for j,v in r)
    print(f"row{i}: {l:.6g} <= {a:.9g} <= {u:.6g}  {' + '.join(f'{v:g}*x{j}' for j,v in r)}", "VIOL" if a<l-1e-7 or a>u+1e-7 else "")
print("--- 残差 (行の活動度と近い側の端の差、負が違反)")
for i,(r,l,u) in enumerate(zip(rows,rlo,rup)):
    a=sum(v*x[j] for j,v in r)
    print(f"row{i}: lo-slack {a-l:.3e}  up-slack {u-a:.3e}")
