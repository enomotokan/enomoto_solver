import sys
# rename.py <orig.mps> <enomoto_dump.mps> <out.mps>: ENOMOTO の書き出し (列名 C{j}) を元の MPS の j 番目の列の名前に直す
names=[]; sec=None
for l in open(sys.argv[1]):
    if l[0]!=' ': sec=l.split()[0]; continue
    t=l.split()
    if sec=='COLUMNS' and not (len(t)>2 and t[1]=="'MARKER'") and (not names or names[-1]!=t[0]): names.append(t[0])
import re
out=open(sys.argv[3],'w')
for l in open(sys.argv[2]):
    out.write(re.sub(r'\bC(\d+)\b', lambda m: names[int(m.group(1))], l))
