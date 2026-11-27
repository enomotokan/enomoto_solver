"""run3.py の結果 (res3.jsonl) の集計: 24 問を既定 (def) と OBJSHAVE=djc で比べる。python summ3.py > summ3.txt
下界の積分は summ.py・summ2.py と同じ定義 (進みの行の lb と、djc では受け取った下界)。
ただし進みの行の lb は、最後に報告される下界より高いことがある (lectsched-5-obj の既定は 292 秒で 23、最後は 15)。
そのため、最後に報告された下界のギャップ g = (opt−lb)/max(|opt|,|lb|,1) も並べる (こちらが確かな値)。
暫定解 (primal) は最後の値。証明の数は OBJSHAVE: tries の行から読む (proved = 示せた、time limit = 空振り)。"""
import json, math, re
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "res3.jsonl").read_text().splitlines()]
def g(opt, lb): return 1.0 if lb is None or not math.isfinite(lb) else max(0.0, min(1.0, (opt - lb) / max(abs(opt), abs(lb), 1.0)))
def integral(r):
    opt, T = r["opt"], r["tl"]; res = r["res"]
    end = res.get("time", T) if res.get("status") == "optimal" else T
    ev = [(t, lb) for t, lb, _ in r["prog"] if math.isfinite(lb)]
    if r["cfg"] != "def": ev += [(t, v) for t, v in r["recv"]]
    ev.sort(); cur = -math.inf; tprev = 0.0; acc = 0.0
    for t, lb in ev:
        t = min(t, end); acc += g(opt, cur) * (t - tprev); tprev = t; cur = max(cur, lb)
    return acc + g(opt, cur) * max(0.0, end - tprev)
def stat(r, k):
    m = re.search(k + r" (\d+)", r["summary"] or ""); return int(m[1]) if m else 0
by = {}
for r in rows: by.setdefault(r["problem"], {})[r["cfg"]] = r
print(f"TL {rows[0]['tl']} 秒、4 スレッド・種 1、ビルド a083189b8。積分は秒、g は最後の下界のギャップ (どちらも小さいほど良い)")
print("| 問題 | 最適値 | 既定: 下界 g 積分 | djc: 下界 g 積分 | 示せた/試し (FJ 見つけ) | 暫定解 既定 / djc |")
print("|---|---:|---|---|---|---|")
ratio, gb, gw, pb, pw, act = [], [], [], [], [], []
tp = tt = 0
for p, d in by.items():
    a, b = d["def"], d["djc"]
    ia, ib = integral(a), integral(b); ratio.append((ib + 1) / (ia + 1))
    la, lb = a["res"].get("bound"), b["res"].get("bound"); ga, gbb = g(a["opt"], la), g(b["opt"], lb)
    pr, tr = stat(b, "proved"), stat(b, "tries"); tp += pr; tt += tr
    if tr: act.append(p)
    if gbb < ga - 1e-9: gb.append(p)
    if gbb > ga + 1e-9: gw.append(p)
    oa, ob = a["res"].get("objective"), b["res"].get("objective")
    if oa is not None and ob is not None and abs(oa - ob) > 1e-6 * max(1, abs(oa)):
        (pb if ob < oa else pw).append(p)
    elif oa is None and ob is not None: pb.append(p)
    elif ob is None and oa is not None: pw.append(p)
    print(f"| {p} | {a['opt']:.10g} | {la:.10g} {ga:.4f} {ia:.1f} | {lb:.10g} {gbb:.4f} {ib:.1f} | {pr}/{tr} ({stat(b, 'fj found')}) | {oa} / {ob} |")
gm = math.exp(sum(math.log(x) for x in ratio) / len(ratio))
print(f"\n下界の積分の比 (djc / 既定、shift 1 秒の幾何平均、{len(ratio)} 問): {gm:.3f}")
print(f"専任が試しを始めた問題: {len(act)} 問 {act}")
print(f"最後の下界: djc が良い {gb}、悪い {gw}")
print(f"暫定解: djc が良い {pb}、悪い {pw}")
print(f"証明: 示せた {tp} / 試し {tt} (空振り {tt - tp})")
