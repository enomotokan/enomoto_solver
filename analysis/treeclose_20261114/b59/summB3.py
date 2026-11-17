"""調べ B3 の集計: python summB3.py > summ3.txt。問題ごとに設定の状態・秒・節点数 (結果と主の木の最後の行)・最後の下界。
比べ: b59 の highs_cut (HiGHS、木 1 本) と ours_cut (4 スレッド)。"""
import gzip, json, glob, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from probsB3 import PROBS, CFGS  # noqa: E402
OURS = re.compile(r"MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)")
R = {(r["problem"], r["cfg"]): r["result"] for f in glob.glob(str(D / "res3_*.jsonl")) for l in open(f) for r in [json.loads(l)]}
R0 = {(r["problem"], r["cfg"]): r["result"] for f in glob.glob(str(D / "res_*.jsonl")) for l in open(f) for r in [json.loads(l)]}
for p, opt, integ in PROBS:
    print(f"\n## {p} (最適 {opt})")
    h = R0.get((p, "highs_cut")); o = R0.get((p, "ours_cut"))
    if h: print(f"  {'HiGHS cut':10s} {str(h.get('status'))[:9]:9s} {h.get('time', 0):6.1f}s 節点 {h.get('nodes')}")
    if o: print(f"  {'b59 ours_cut':10s} {str(o.get('status'))[:9]:9s} {o.get('time', 0):6.1f}s 節点 {o.get('nodes')} 下界 {o.get('bound')}")
    t = {r["cfg"]: r["result"] for f in glob.glob(str(D / "res2_*.jsonl")) for l in open(f) for r in [json.loads(l)] if r["problem"] == p}.get("t1")
    if t: print(f"  {'B2 t1 (基準)':10s} {str(t.get('status'))[:9]:9s} {t.get('time', 0):6.1f}s 節点 {t.get('nodes')} 下界 {t.get('bound')}")
    for c in CFGS:
        x = R.get((p, c))
        if not x: print(f"  {c:10s} -"); continue
        f = D / "logs" / f"B3_{p}_{c}.log.gz"; last = None; nrst = 0
        if f.exists():
            for l in gzip.open(f, "rt", errors="replace"):
                m = OURS.search(l)
                if m: last = (int(m.group(2)), m.group(4))
                if re.search(r"restart \d+: pseudocost", l): nrst += 1
        print(f"  {c:10s} {str(x.get('status'))[:9]:9s} {x.get('time', 0):6.1f}s 節点 {x.get('nodes')} 下界 {x.get('bound')} 主の木の最後 {last} 再スタート {nrst}")
