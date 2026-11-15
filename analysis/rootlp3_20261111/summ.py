"""ROOTLP3 の集計 (analysis/rootlp3_20261111.md)。python summ.py <jsonl> [A,B (既定 def,s)]
- 全体: analysis/benchlib.py の report (SGM・解けた回・対の比の 95% 区間)。
- 誤答: optimal の目的値を既知の最適値 (scripts/miplib2017-v31.solu の =opt=) と比べ、下界が既知の最適値を越えていないか (向きは目的値と下界から)。
- 根の LP (診断 ENOMOTO_MIP_ROOTLP3_LOG の行): 種類 (4 最初、3 再スタートの後) ごとの数・基底あり・特異・捨てた・秒・反復の和、
  捨てた根の LP の問題ごとの秒・反復 (A と B)。
- 最大 RSS (MB) の比。
- 根の頂点の揺れ: 同じ (問題, 種, 設定) を 2 回以上 (run) 走らせた行があれば、最初の根の LP の xhash が一致した割合。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
path = sys.argv[1] if Path(sys.argv[1]).is_absolute() else str(D / sys.argv[1])
A, B = (sys.argv[2].split(",") if len(sys.argv) > 2 else ["def", "s"])
raw = [json.loads(l) for l in open(path) if l.strip()]
first = [r for r in raw if r.get("run", 0) == 0]
tmp = D / ".summ_tmp.jsonl"
tmp.write_text("".join(json.dumps(r) + "\n" for r in first))
R = bl.table(bl.load([tmp]))
tmp.unlink()
keys = [k for k in R if A in R[k] and B in R[k]]
bl.report(R, A, B, keys, label="58 問 60 秒")
# 誤答
solu = {}
for l in open(D.parents[1] / "scripts" / "miplib2017-v31.solu"):
    p = l.split()
    if len(p) >= 3 and p[0] == "=opt=":
        solu[p[1]] = float(p[2])
close = lambda a, b: abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))
bad = []
for r in raw:
    res, p = r["result"], r["problem"]
    o, b = res.get("objective"), res.get("bound")
    if res.get("status") == "optimal" and o is not None and p in solu and not close(o, solu[p]):
        bad.append((p, r["cfg"], r["rep"], "obj", o, solu[p]))
    if p in solu and o is not None and isinstance(b, (int, float)) and math.isfinite(b):
        s = solu[p]
        if (o >= b and b > s + 1e-6 * max(1, abs(s))) or (o < b and b < s - 1e-6 * max(1, abs(s))):
            bad.append((p, r["cfg"], r["rep"], "bound", b, s))
    if o is not None and isinstance(b, (int, float)) and math.isfinite(b) and res.get("status") == "optimal" and not close(o, b):
        pass
print(f"\n誤答・暫定解と下界の矛盾 (既知の最適値と比べて): {len(bad)} 件 (行 {len(raw)})")
for x in bad:
    print("  ", x)
# 根の LP
print("\n根の LP (ROOTLP3 の行、run 0)")
agg = defaultdict(lambda: [0, 0, 0, 0, 0.0, 0])
per = defaultdict(dict)
for r in first:
    for i, x in enumerate(r.get("rootlp3", [])):
        a = agg[(r["cfg"], x["kind"])]
        a[0] += 1; a[1] += x["warm"]; a[2] += x["singular"]; a[3] += x["dropped"]; a[4] += x["secs"]; a[5] += x["iters"]
        if x["singular"]:
            per[(r["problem"], r["rep"], x["kind"], i)][r["cfg"]] = (x["secs"], x["iters"], x["dropped"])
print("| 設定 | 種類 | LP 数 | 基底あり | 特異 | 捨てた | 秒の和 | 反復の和 |")
print("|---|---|---|---|---|---|---|---|")
for (c, k), a in sorted(agg.items()):
    print(f"| {c} | {'最初' if k == 4 else '再スタートの後'} | {a[0]} | {a[1]} | {a[2]} | {a[3]} | {a[4]:.1f} | {a[5]} |")
print(f"\n特異な基底の根の LP の問題ごと (秒 / 反復、{A} → {B})")
print("| 問題 | 種 | 種類 | 何番目の根 | " + A + " | " + B + " |")
print("|---|---|---|---|---|---|")
sa = sb = 0.0
for k in sorted(per):
    v = per[k]
    f = lambda c: f"{v[c][0]:.2f} / {v[c][1]}" if c in v else ""
    if A in v and B in v:
        sa += v[A][0]; sb += v[B][0]
    print(f"| {k[0]} | {k[1]} | {k[2]} | {k[3]} | {f(A)} | {f(B)} |")
print(f"両方にある分の秒の和: {sa:.1f} → {sb:.1f}")
# 最大 RSS
ra = [(r["problem"], r["rep"], r["cfg"], r.get("maxrss_kb")) for r in first if r.get("maxrss_kb")]
m = {(p, s, c): v for p, s, c, v in ra}
pairs = [(m[(p, s, A)], m[(p, s, B)]) for (p, s) in keys if (p, s, A) in m and (p, s, B) in m]
if pairs:
    gm = math.exp(sum(math.log(b / a) for a, b in pairs) / len(pairs))
    big = sorted(pairs, key=lambda x: -x[1] / x[0])[:3]
    print(f"\n最大 RSS: 対 {len(pairs)}、比の幾何平均 {gm:.3f}、最大の比 {[round(b / a, 2) for a, b in big]}、最大 {max(b for _, b in pairs) / 1024:.0f} MB ({B}) / {max(a for a, _ in pairs) / 1024:.0f} MB ({A})")
# 揺れ
runs = defaultdict(list)
for r in raw:
    h = next((x["xhash"] for x in r.get("rootlp3", []) if x["kind"] == 4), None)
    hs = tuple(x["xhash"] for x in r.get("rootlp3", []))
    runs[(r["problem"], r["rep"], r["cfg"])].append((h, hs))
multi = {k: v for k, v in runs.items() if len(v) >= 2}
if multi:
    for c in (A, B):
        ks = [k for k in multi if k[2] == c]
        same1 = sum(1 for k in ks if len({h for h, _ in multi[k]}) == 1)
        sameall = sum(1 for k in ks if len({hs for _, hs in multi[k]}) == 1)
        print(f"根の頂点の揺れ ({c}): 2 回以上走らせた {len(ks)} 組、最初の根の頂点が同じ {same1}、全部の根の頂点が同じ {sameall}")
