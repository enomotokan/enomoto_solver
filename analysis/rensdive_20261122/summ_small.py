"""res_small.jsonl の集計 (def と rensdive の比べ): python summ_small.py [res_small.jsonl] [設定A] [設定B] > summ_small.txt"""
import json, math, sys
f = sys.argv[1] if len(sys.argv) > 1 else "res_small.jsonl"
A = sys.argv[2] if len(sys.argv) > 2 else "def"
B = sys.argv[3] if len(sys.argv) > 3 else "rensdive"
R = {}
for l in open(f):
    r = json.loads(l); R[(r["problem"], r["seed"], r["threads"], r["cfg"])] = r
keys = sorted({(p, s, t) for p, s, t, c in R})
def sol(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def fmt(v, d=2): return "-" if v is None else (f"{v:.{d}f}" if isinstance(v, float) else str(v))
print(f"問題 種 スレッド | {A}: 状態 時間 最初の解(秒/値) 根の終わりの解 | {B}: 同じ + LPBOX の受け入れ・出どころ")
T = {A: [], B: []}; cnt = {A: 0, B: 0}; firsts = {A: [], B: []}
for p, s, t in keys:
    a, b = R.get((p, s, t, A)), R.get((p, s, t, B))
    if not a or not b: continue
    row = []
    for c, r in ((A, a), (B, b)):
        x = r["result"]; tm = x.get("time") or r["tl"]
        T[c].append(min(tm, r["tl"]) if sol(r) else r["tl"]); cnt[c] += sol(r)
        ft = x.get("first_solution_time"); firsts[c].append(ft if ft is not None else r["tl"])
        ri = r.get("root_inc")
        row.append(f"{x.get('status')} {fmt(tm,1)} {fmt(ft)}/{fmt(x.get('first_solution_objective'),6) if x.get('first_solution_objective') is not None else '-'} root {fmt(ri[0],6) if ri else '-'}")
    lb = ""
    if b.get("lpbox"):
        m = b["lpbox"][-1]
        import re
        g = re.search(r"found (\d+).*obj (\S+), accepted (\w+)", m)
        lb = f" LPBOX found {g.group(1)} obj {g.group(2)} acc {g.group(3)}" if g else ""
    src = {k: v for k, v in b.get("src_count", {}).items() if k.startswith("RENSDIVE")}
    print(f"{p} {s} {t} | {row[0]} | {row[1]} |{lb} src {src} last {b.get('last_src')}")
g = lambda xs: math.exp(sum(math.log(x + 1) for x in xs) / len(xs)) - 1
print(f"\n解けた数 {A} {cnt[A]} {B} {cnt[B]} / {len(T[A])}")
print(f"SGM (ずらし 1 秒、時間切れは上限) {A} {g(T[A]):.2f} {B} {g(T[B]):.2f} 比 {(g(T[B])+1)/(g(T[A])+1):.3f}")
print(f"最初の解の時間の SGM {A} {g(firsts[A]):.2f} {B} {g(firsts[B]):.2f}")
