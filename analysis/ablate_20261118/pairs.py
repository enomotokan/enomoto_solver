"""既定にした機能ごとの採用時の対の計測 (res*.jsonl) から、問題ごとの効き目の二極化を拾う (2026-11-18、計測なし)。
使い方: python pairs.py 名前 'res のグロブ' 入り切りの設定 (機能あり) 切りの設定 (機能なし) [tl]
  比 = (機能ありの時間 + 10) / (機能なしの時間 + 10) (時間切れは tl)。同じ (問題, 種, tl) で対にする (同じ組が複数あれば幾何平均)。
  出力: 全種で 1.25 倍超遅い問題 (slow)・全種で 0.8 倍未満速い問題 (fast)・片方だけ解けた種のある問題 (one) と、全体の SGM 比。"""
import glob, json, math, sys, collections
name, pat, on, off = sys.argv[1:5]
tlsel = int(sys.argv[5]) if len(sys.argv) > 5 else None
runs = collections.defaultdict(list)
# 機能ありの設定を含むファイルだけを使う (同じ場所に別の計測の既定が混ざっていても、その計測の対だけを取る)
allf = sorted(glob.glob(pat, recursive=True))
files = [f for f in allf if any(('"cfg": "%s"' % on) in l or ('"solver": "ours:%s"' % on) in l for l in open(f))]
# 設定ごとにファイルを分けた計測 (mitt600 の res8) は、機能ありのファイルに既定がない。そのときは全部のファイルを使う
if not any(('"cfg": "%s"' % off) in l or ('"solver": "%s"' % off) in l for f in files for l in open(f)):
    files = allf
for f in files:
    for l in open(f):
        try:
            r = json.loads(l)
        except Exception:
            continue
        c = r.get("cfg") or str(r.get("solver", "")).split(":")[-1]
        tl = r.get("tl") or tlsel or 600  # 古い計測 (mitt600 の res8 など) は `solver` = `ours:<設定>` で tl を持たない
        if c not in (on, off) or (tlsel and tl != tlsel):
            continue
        res = r.get("result") or {}
        st = res.get("status")
        t = res.get("time")
        if st is None or t is None or str(st).startswith("crash"):
            continue
        solved = st in ("optimal", "infeasible")
        t = min(t, tl) if solved else tl
        seed = r.get("seed", r.get("rep", r.get("run")))
        runs[(r["problem"], seed, tl, c)].append((t, solved))
pairs = collections.defaultdict(list)  # (problem, tl) -> [(ratio, solved_on, solved_off, seed)]
keys = {(p, s, tl) for (p, s, tl, c) in runs}
for (p, s, tl) in sorted(keys, key=str):
    a, b = runs.get((p, s, tl, on)), runs.get((p, s, tl, off))
    if not a or not b:
        continue
    ga = math.exp(sum(math.log(t + 10) for t, _ in a) / len(a))
    gb = math.exp(sum(math.log(t + 10) for t, _ in b) / len(b))
    pairs[(p, tl)].append((ga / gb, all(x for _, x in a), all(x for _, x in b), s))
out = {"name": name, "on": on, "off": off, "tl": {}, "slow": [], "fast": [], "one": []}
for (p, tl), v in sorted(pairs.items(), key=str):
    rs = [x[0] for x in v]
    if all(not x[1] and not x[2] for x in v):
        continue  # 両方とも全種で解けない (時間切れどうし) は比にしない
    rec = {"problem": p, "tl": tl, "ratios": [round(r, 2) for r in rs], "seeds": [x[3] for x in v], "solved_on": sum(x[1] for x in v), "solved_off": sum(x[2] for x in v)}
    rs2 = [x[0] for x in v if x[1] or x[2]]  # 両方とも時間切れの種 (比 1) は全種一致の判定から外す
    if len(rs2) >= 2 and all(r > 1.25 for r in rs2):
        out["slow"].append(rec)
    if len(rs2) >= 2 and all(r < 0.8 for r in rs2):
        out["fast"].append(rec)
    if any(x[1] != x[2] for x in v):
        out["one"].append(rec)
for tl in sorted({tl for (_, tl) in pairs}):
    rs = [r for (p, t), v in pairs.items() if t == tl for r, *_ in v]
    out["tl"][tl] = {"pairs": len(rs), "sgm_ratio": round(math.exp(sum(map(math.log, rs)) / len(rs)), 3) if rs else None,
                     "solved_on": sum(x[1] for (p, t), v in pairs.items() if t == tl for x in v), "solved_off": sum(x[2] for (p, t), v in pairs.items() if t == tl for x in v)}
print(json.dumps(out, ensure_ascii=False))
