"""res58.jsonl の集計 (analysis/mushroom_cuts_20261106.md、intvub_20261106/summ58.py を写したもの): 回ごと・設定ごとの SGM (ずらし 10 秒、解けなければ 60 秒)・解けた数、
根の時間 (最初のカットのループの終わりの時刻) の和と比、問題ごとの差 (時間が 1.5 倍以上・解けた/解けないが変わったもの)。
使い方: python summ58.py [設定 A] [設定 B]"""
import json, math, sys, collections
A, B = (sys.argv[1:3] if len(sys.argv) > 2 else ("def", "mc"))
R = [json.loads(l) for l in open("res58.jsonl")]
by = {(r["problem"], r["cfg"], r["rep"]): r for r in R}
probs = list(dict.fromkeys(r["problem"] for r in R))
reps = sorted({r["rep"] for r in R})
def t(r):
    res = r["result"]
    return res["time"] if res.get("status") in ("optimal", "infeasible") else 60.0
def solved(r):
    return r["result"].get("status") in ("optimal", "infeasible")
def rt(r):
    l = r.get("root_loops") or []
    return l[0]["t"] if l and l[0]["t"] is not None else None
for rep in reps:
    both = [p for p in probs if (p, A, rep) in by and (p, B, rep) in by]
    for c in (A, B):
        xs = [by[(p, c, rep)] for p in both]
        sgm = math.exp(sum(math.log(t(r) + 10) for r in xs) / len(xs)) - 10
        print(f"rep {rep} {c}: problems {len(xs)} solved {sum(solved(r) for r in xs)} SGM {sgm:.2f}")
    rts = [(rt(by[(p, A, rep)]), rt(by[(p, B, rep)])) for p in both]
    rts = [(a, b) for a, b in rts if a is not None and b is not None]
    print(f"rep {rep} root time sum {A} {sum(a for a, _ in rts):.1f} {B} {sum(b for _, b in rts):.1f} (problems {len(rts)})")
    print(f"rep {rep} wrong answers / crashes:", [(p, c, by[(p, c, rep)]["result"].get("status")) for p in both for c in (A, B) if by[(p, c, rep)]["result"].get("status") in ("crash", "error")])
print("problem differences (time ratio >= 1.5 or solved status differs), per rep: A time / B time")
for p in probs:
    row = []
    flag = False
    for rep in reps:
        if (p, A, rep) in by and (p, B, rep) in by:
            a, b = by[(p, A, rep)], by[(p, B, rep)]
            ta, tb = t(a), t(b)
            if solved(a) != solved(b) or max(ta + 1, tb + 1) / min(ta + 1, tb + 1) >= 1.5:
                flag = True
            row.append(f"{ta:.1f}/{tb:.1f}")
    iv = next((r["intvub"][0] for r in R if r["problem"] == p and r.get("intvub")), "")
    if flag:
        print(f"  {p}: {' '.join(row)}  {iv}")
# 試作の分離が候補を出した問題 (logs/<B>_<問題>_60_r<回>.err.gz の SEP mushagg の行の cands の和)
import gzip, os
print(f"problems where {B} separation produced candidates (sum of cands per rep):")
for p in probs:
    cs = []
    for rep in reps:
        f = f"logs/{B}_{p}_60_r{rep}.err.gz"
        if os.path.exists(f):
            cs.append(sum(int(l.split()[-1]) for l in gzip.open(f, "rt") if l.startswith("SEP mushagg")))
    if any(cs):
        print(f"  {p}: {cs}")
