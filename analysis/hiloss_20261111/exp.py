"""HILOSS: 切り分けの実験 (run2.py、res2_s*〜res5_s*.jsonl) を、同じ問題の既定の回 (res_s*.jsonl の ENOMOTO) と HiGHS に並べる。
使い方: python exp.py > exp.txt   (別の機械・別の時刻の回どうしなので、解けた/解けない・下界・暫定解の違いを見る。数 % の時間差は機械の差を含む)"""
import json, glob, math
from pathlib import Path
D = Path(__file__).resolve().parent
base, hi, ex = {}, {}, {}
for f in sorted(glob.glob(str(D / "res_s*.jsonl")), reverse=True):
    for l in open(f):
        r = json.loads(l)
        (base if r["solver"] == "ours" else hi)[r["problem"]] = r  # 取り直した 4 問はログのある res_s0 の行が勝つ
for f in sorted(glob.glob(str(D / "res[2345]_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l); ex.setdefault(r["problem"], {})[r["cfg"]] = r
NAME = {"cq": "クリークカット常用", "cr": "ラウンド上限 100", "cqr": "cq + cr", "sj": "SUBCAP=JB", "ru": "RCUNION", "sd": "退化でも強分岐", "t1": "TREE1_SB", "es": "LOSE6_1_STOP=w", "fb": "CLIQUE_FB_RATIO=0.1"}
def g(v): return "-" if v is None or (isinstance(v, float) and not math.isfinite(v)) else f"{v:.9g}"
def row(tag, r):
    x = r["result"]; st = str(x.get("status", "")).lower()
    solved = st.startswith(("optimal", "infeasible"))
    return f"  {tag:22} {'解けた' if solved else '時間切れ':6} {x.get('time', 0):6.0f} 秒  暫定解 {g(x.get('objective')):>14}  下界 {g(x.get('bound')):>14}  ノード {str(x.get('nodes')):>8}  根の下界 {g(x.get('root_bound')):>14}"
for p in sorted(ex):
    print(p)
    if p in hi: print(row("HiGHS", hi[p]))
    if p in base: print(row("既定", base[p]))
    for c, r in ex[p].items(): print(row(f"{c} ({NAME.get(c, c)})", r))
