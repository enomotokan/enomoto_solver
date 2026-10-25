"""前処理の後の形の比較。使い方: python3 cmp.py <jsonl> [<jsonl> ...]。ラベルごとに (行, 列, 非零, LP, md5) を並べ、
ラベルによって形 (md5) が変わる問題を表にする。前処理の時間の最大・最小も出す。"""
import sys, json, collections
recs = collections.defaultdict(dict)
labels = []
for f in sys.argv[1:]:
    for l in open(f):
        r = json.loads(l)
        recs[r['name']][r['label']] = r
        if r['label'] not in labels:
            labels.append(r['label'])
def sh(r):
    e = r.get('e')
    if not e:
        return '—'
    lp = e.get('lp')
    return f"{e['rows']}×{e['cols']}, {e['nnz']}, {lp:.6g}" if lp is not None else f"{e['rows']}×{e['cols']}, {e['nnz']}, —"
print('| 問題 | ' + ' | '.join(labels) + ' | 前処理の秒 (最小〜最大) |')
print('|---|' + '---|' * (len(labels) + 1))
nvar = 0; same = []
for p in sorted(recs):
    rs = recs[p]
    md5s = {rs[l]['md5'] for l in labels if l in rs}
    pts = [rs[l]['pt'] for l in labels if l in rs and rs[l]['pt'] is not None]
    if len(md5s) > 1:
        nvar += 1
        # 最初のラベルと同じ md5 なら「=」
        m0 = rs[labels[0]]['md5'] if labels[0] in rs else None
        cells = [('= ' if (l in rs and rs[l]['md5'] == m0 and l != labels[0]) else '') + (sh(rs[l]) if l in rs else '') for l in labels]
        print(f"| {p} | " + ' | '.join(cells) + f" | {min(pts):.2f}〜{max(pts):.2f} |")
    else:
        same.append(p)
print(f"\n形 (書き出しの md5) が変わる問題 {nvar} 問 / {len(recs)} 問。変わらない問題: {' '.join(same)}")
