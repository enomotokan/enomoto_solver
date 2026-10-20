import sys, random
# shufrows.py <in> <out> <seed>: ROWS 節の行の順を入れ替える (目的の行は先頭のまま)
src,dst,seed=sys.argv[1],sys.argv[2],int(sys.argv[3])
L=open(src).read().split('\n'); i=L.index('ROWS'); j=i+1
while L[j].startswith(' '): j+=1
body=L[i+1:j]; obj=[l for l in body if l.split()[0]=='N']; rest=[l for l in body if l.split()[0]!='N']
random.Random(seed).shuffle(rest)
open(dst,'w').write('\n'.join(L[:i+1]+obj+rest+L[j:]))
