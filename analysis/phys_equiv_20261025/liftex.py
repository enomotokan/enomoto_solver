"""liftex.py: HiGHS の持ち上げ (logs/h_orig.lift の最初の呼び出し) の例を、元の行の中身と並べる。持ち上げた列が元の行にどう現れるかを数える"""
import highspy, collections, numpy as np
h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel('../../.miplib_cache/physiciansched6-2.mps'); lp=h.getLp()
cn=list(lp.col_names_); rn=list(lp.row_names_); ridx={n:i for i,n in enumerate(rn)}
st=np.array(lp.a_matrix_.start_); ix=np.array(lp.a_matrix_.index_); vl=np.array(lp.a_matrix_.value_)
rows=collections.defaultdict(list); colrows=collections.defaultdict(list)
for j in range(lp.num_col_):
    for k in range(st[j],st[j+1]): rows[ix[k]].append((cn[j],vl[k])); colrows[cn[j]].append(ix[k])
rl=list(lp.row_lower_); ru=list(lp.row_upper_)
L=[]
for l in open('logs/h_orig.lift'):
    if l.startswith('#'): break
    r,c,v,d=l.split(); L.append((r,c,int(v)))
print('val 分布', collections.Counter(v for _,_,v in L))
print('持ち上げた列の元の行の数', collections.Counter(min(len(colrows[c]),9) for _,c,_ in L).most_common())
print('行の元の長さ', collections.Counter(min(len(rows[ridx[r]]),12) for r,_,_ in L).most_common())
for r,c,v in L[:6]:
    i=ridx[r]; print(r, rl[i], ru[i], rows[i][:12], '-> +', c, v, 'col rows', [(rn[k], len(rows[k])) for k in colrows[c]][:5])
