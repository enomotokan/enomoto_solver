"""誤答の確かめ: 全部の jsonl の回で、どの回の下界 (最小化に直したもの) も、どの回の実行可能解の目的値を越えないか、
最適と言った回どうしの目的値が一致するか。使い方: python conflict.py a.jsonl b.jsonl ..."""
import json, sys
from collections import defaultdict
R = defaultdict(list)
for f in sys.argv[1:]:
    for l in open(f):
        if l.strip():
            r = json.loads(l); x = r["result"]
            if x.get("objective") is not None and x.get("bound") is not None:
                R[r["problem"]].append((f, r["cfg"], r["rep"], x["status"], x["objective"], x["bound"]))
bad = 0
for p, rs in R.items():
    lo = min(min(o, b) for *_, o, b in rs)  # 符号の向きは、各回の (目的値, 下界) の大小で決める
    minim = sum(o >= b - 1e-6 for *_, o, b in rs) >= sum(o <= b + 1e-6 for *_, o, b in rs)
    best = min(o for *_, o, b in rs) if minim else max(o for *_, o, b in rs)
    for f, c, s, st, o, b in rs:
        tol = 1e-6 * max(1, abs(best))
        if (minim and b > best + tol) or (not minim and b < best - tol):
            bad += 1; print("下界が解を越える", p, f, c, s, o, b, best)
        if st == "optimal" and abs(o - best) > 1e-4 * max(1, abs(best)):
            bad += 1; print("最適の値が違う", p, f, c, s, o, best)
print("問題", len(R), "回", sum(len(v) for v in R.values()), "食い違い", bad)
