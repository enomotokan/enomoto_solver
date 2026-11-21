"""問題をまたいで、木の分離の効き目 (伸びの和/1e9 手間、張った回/LP にあった回) と、木の始めのギャップ・根で閉じた割合の順位相関を見る。
使い方: python corr.py res.jsonl"""
import json, sys
sys.argv = [sys.argv[0], sys.argv[1], "4"]
import summ as S  # noqa: E402


def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i]); r = [0.0] * len(v); i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and v[o[j + 1]] == v[o[i]]:
            j += 1
        for k in range(i, j + 1):
            r[o[k]] = (i + j) / 2
        i = j + 1
    return r


def spearmanr(a, b):
    ra, rb = rank(a), rank(b); n = len(a); ma, mb = sum(ra) / n, sum(rb) / n
    c = sum((x - ma) * (y - mb) for x, y in zip(ra, rb)); da = sum((x - ma) ** 2 for x in ra) ** 0.5; db = sum((y - mb) ** 2 for y in rb) ** 0.5
    return (c / (da * db) if da * db > 0 else float("nan"),)
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
for th in (4, 1):
    data = []
    probs = list(dict.fromkeys(r["problem"] for r in rows if r["threads"] == th))
    for p in probs:
        rr = [r for r in rows if r["problem"] == p and r["threads"] == th]
        recs, ri = [], []
        for r in rr:
            rc = S.load("res", p, th, [r["rep"]]); recs += rc; ri.append(S.rootinfo(rc, p))
        B, C, T = S.summarize(recs)
        tk = sum(C[i]["ticks"] for i in range(6))
        if tk == 0:
            continue
        g = sum(sum(B[i]["gain"]) for i in range(6))
        bind = sum(C[i]["bind"] for i in range(6)); lin = sum(C[i]["lin_sum"] for i in range(6))
        mean = lambda v: sum(v) / len(v) if v else None
        gap = mean([x[1] for x in ri if x and x[1] is not None])
        cl = mean([x[2] for x in ri if x and x[2] is not None])
        eff = mean([x[2] / x[3] for x in ri if x and x[2] is not None and x[3] > 0])
        data.append((p, g / (tk / 1e9), bind / max(lin, 1), gap, cl, eff, eff * gap if eff is not None and gap is not None else None))
    print(f"\n#### スレッド {th}\n\n| 問題 | 伸びの和/1e9 手間 | 張った回/LP にあった回 | 始めのギャップ | 根で閉じた割合 | 閉じた割合/1e9 手間 | 閉じた割合/手間 × ギャップ |\n|---|---|---|---|---|---|---|")
    for d in data:
        print("| " + " | ".join([d[0]] + ["-" if v is None else f"{v:.3g}" for v in d[1:]]) + " |")
    print("\n| 説明に使う量 | 伸びの和/手間との順位相関 | 張った回/LP との順位相関 | 問題数 |\n|---|---|---|---|")
    for j, nm in [(3, "始めのギャップ"), (4, "根で閉じた割合"), (5, "閉じた割合/手間"), (6, "閉じた割合/手間 × ギャップ")]:
        xs = [d for d in data if d[j] is not None]
        r1 = spearmanr([d[j] for d in xs], [d[1] for d in xs])[0]
        r2 = spearmanr([d[j] for d in xs], [d[2] for d in xs])[0]
        print(f"| {nm} | {r1:.2f} | {r2:.2f} | {len(xs)} |")
