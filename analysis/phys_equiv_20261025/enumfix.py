"""enumfix.py: HiGHS の列挙の固定 (logs/h_enum.enum の最初の呼び出し) を ENOMOTO の探りの固定と比べ、元の問題に入れた dump/orig_enumfix.mps を書く"""
import highspy
h=highspy.Highs(); h.setOptionValue('output_flag',False); h.readModel('../../.miplib_cache/physiciansched6-2.mps'); lp=h.getLp()
names=list(lp.col_names_); idx={n:j for j,n in enumerate(names)}
fx=[]
for l in open('logs/h_enum.enum'):
    if l.startswith('#'): break
    a,b,c=l.split(); fx.append((idx[a],float(b),float(c)))
print(len(fx), 'at0', sum(1 for j,a,b in fx if b==0), 'at1', sum(1 for j,a,b in fx if a==1))
def fixed(f):
    e=highspy.Highs(); e.setOptionValue('output_flag',False); e.readModel(f); l=e.getLp()
    cn=list(l.col_names_); lo=list(l.col_lower_); up=list(l.col_upper_)
    return {int(cn[j][1:]) for j in range(len(cn)) if lo[j]==up[j]}
f0=fixed('dump/def_0.mps'); f1=fixed('dump/def_pre1.mps'); hs={j for j,_,_ in fx}
newfix=f1-f0
print('ENOMOTO の探りの固定', len(newfix), 'そのうち HiGHS の列挙の固定', len(newfix & hs), '; HiGHS の列挙の固定で ENOMOTO の探りの前に既に固定', len(f0 & hs), '; どちらでもない', len(hs-f0-f1))
for j,a,b in fx: h.changeColBounds(j,a,b)
h.writeModel('dump/orig_enumfix.mps')
