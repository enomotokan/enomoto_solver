# HiGHS の計装ログ (HIGHS_PATH_LOG の HP/HB 行) から ENOMOTO_MIP_BRANCH_REPLAY のファイルを作る
# 使い方: python3 highs_to_replay.py <HiGHS の標準エラー> > replay.txt
# 出力: seq 結果 分枝した列 先の子が上か lp nfix 深さ 経路...
#  (同じ経路のノードの再評価 (inbr=1) は最初の seq のまま、結果・LP 値は最後のものにする)
import sys
nodes = {}   # key -> dict
order = []
last = None
for l in open(sys.argv[1]):
    t = l.split()
    if not t:
        continue
    if t[0] == 'HP':
        seq, d, r, lp, lb, rows, inbr, nfix, rs = t[1:10]
        path = [x for x in t[10:] if not x.startswith('#')]
        rs = [x for x in t[10:] if x.startswith('#')]
        key = tuple(sorted(path))
        if key not in nodes:
            nodes[key] = dict(seq=len(order), res=int(r), col=-1, up=0, lp=lp, nfix=nfix, depth=d, path=path, rs=rs)
            order.append(key)
        else:
            nodes[key].update(res=int(r), lp=lp, nfix=nfix, rs=rs)
        last = key
    elif t[0] == 'HB':
        nodes[last]['col'] = int(t[1])
        nodes[last]['up'] = int(t[3])
for k in order:
    n = nodes[k]
    print(n['seq'], n['res'], n['col'], n['up'], n['lp'], n['nfix'], n['depth'], *n['path'], *n['rs'])
