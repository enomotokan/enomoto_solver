import statistics as st
exec(open('/tmp/claude-0/rdump/an/join.py').read().split("m = [")[0].replace("sys.argv[1]","__import__('sys').argv[1]"))
m = [(e, hp[e['seq']]) for e in er if e['seq'] >= 0 and e['seq'] in hp]
sel = [(e,h) for e,h in m if h['res']==0 and e['out']=='br']
for k in 'BRQXOUCFZLY':
    print('HiGHS', k, round(st.mean(h['cnt'].get(k,0) for e,h in sel),1))
for k in 'BRLSOPFJEC':
    print('ours', k, round(st.mean(e['cnt'].get(k,0) for e,h in sel),1))
print('ours nfix-244', st.mean(e['nfix']-244 for e,h in sel), 'HiGHS nfix', st.mean(h['nfix'] for e,h in sel))
