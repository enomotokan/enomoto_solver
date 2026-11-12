"""段ごとの数え (count_stages.jsonl、ENOMOTO_MIP_FOLD=riNc) を集計する (analysis/foldpre_20261112.md)。
畳まず前処理だけの自由な列・行と、N の最後の大きさ、段の数、2 段目以降に縮んだかを出す。"""
import json, re, sys
rows = []
for l in open(sys.argv[1]):
    r = json.loads(l)
    base = None; stages = []; last = None
    for ln in r["fold"]:
        m = re.search(r"STAGE base: presolve only: free cols (\d+) -> (\d+), m (\d+) -> (\d+)", ln)
        if m: base = (int(m.group(2)), int(m.group(4)))
        m = re.search(r"STAGE (\d+): (fold|presolve): free cols (\d+) -> (\d+), m (\d+) -> (\d+)", ln)
        if m:
            d, kind = int(m.group(1)), m.group(2)
            stages.append((d, kind, int(m.group(3)), int(m.group(4)), int(m.group(5)), int(m.group(6))))
            last = (int(m.group(4)), int(m.group(6)))
    if base is None or last is None:
        continue
    folds = [s for s in stages if s[1] == "fold" and s[3] < s[2]]
    nst = max([s[0] for s in folds], default=0)
    later = [s for s in folds if s[0] >= 2]
    rows.append((r["name"], base, last, nst, len(later) > 0, [(s[0], s[2], s[3]) for s in folds]))
print(f"{len(rows)} 問、畳めた {sum(r[3] > 0 for r in rows)}、2 段目以降も畳めた {sum(r[4] for r in rows)}")
print("問題 | 前処理だけ 列/行 | N の最後 列/行 | 列の減り | 畳んだ段 (段, 前, 後)")
for r in sorted(rows, key=lambda r: (r[2][0] - r[1][0]) / max(1, r[1][0])):
    if r[3] == 0:
        continue
    red = 100 * (r[1][0] - r[2][0]) / max(1, r[1][0])
    print(f"{r[0]} | {r[1][0]}/{r[1][1]} | {r[2][0]}/{r[2][1]} | {red:.1f}% | {r[5]}")
