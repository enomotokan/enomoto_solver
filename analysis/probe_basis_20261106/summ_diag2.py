"""diag_v2.jsonl の表: 既定どおりの基底 (行つきなら dflt、列だけなら warm) / 基底なし (同時実行) / 行の中身で作り直した基底 (fix)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
f = sys.argv[1] if len(sys.argv) > 1 else "diag_v2.jsonl"
def s(x):
    if not x: return "-"
    return f"{x['t']:.1f} ({x['it']}){'' if x['st'] == 'Optimal' else ' ' + x['st']}"
print("| 問題 | 既定の基底 | 中身の違う行 | 特異 (既定の基底) | 主実行不能 (既定) | 既定の基底から 秒 (反復) | 基底なし 秒 (反復) | 作り直し 照合 / 外した列 | 作り直しから 秒 (反復) |")
print("|---|---|---:|---|---:|---|---|---|---|")
for l in open(D / f):
    r = json.loads(l); d = r["diag"] or {}
    fe = d.get("feat") or {}; kind = d.get("kind")
    if kind == "rows":
        df = d.get("dflt") or {}; res = df.get("res"); sing = df.get("singular"); pinf = df.get("pinf_cnt")
    else:
        res = d.get("warm"); sing = fe.get("singular"); pinf = fe.get("pinf_cnt")
    chg = 1 - fe["same_rows"] / max(fe["m1"], 1) if fe else None
    fx = d.get("fix") or {}
    print(f"| {r['problem']} | {'行つき' if kind == 'rows' else '列だけ' if kind == 'cols' else kind} | {'' if chg is None else f'{chg:.1%}'} | {'特異' if sing else '-'} | {pinf if pinf is not None else '-'} | {s(res)} | {s(d.get('cold'))} | {str(fx.get('matched', '-')) + ' / ' + str(fx.get('dropped', '-')) if fx else '-'}{' 特異' if fx.get('singular') else ''} | {s(fx.get('res'))} |")
