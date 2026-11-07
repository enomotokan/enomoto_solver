"""diag_v3.jsonl の表 (列だけの基底の補い方: 既定 w・基底なし c・作り直し f・g・h・H)。"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
def s(x):
    if not x: return "-"
    return f"{x['t']:.2f} ({x['it']}){'' if x['st'] == 'Optimal' else ' ' + x['st']}"
def sg(v):
    return "特異" if v else "正則"
print("| 問題 | 既定 (列だけ、行 0,1,2.. で埋める) | 基底なし (競争) | f 作り直し | g 前処理の行で埋める [基底, 新, 余裕] | h = g + 修復 | H = 既定 + 修復 |")
print("|---|---|---|---|---|---|---|")
for l in open(D / "diag_v3.jsonl"):
    r = json.loads(l); d = r["diag"] or {}
    fe = d.get("feat") or {}
    w = d.get("warm"); f = d.get("fix") or {}
    def g(t):
        x = d.get(t)
        if not x: return "-"
        return f"{sg(x['singular'])} {x['fill']} {s(x['res'])}" if t != "H" else f"{sg(x['singular'])} {s(x['res'])}"
    print(f"| {r['problem']} | {sg(fe.get('singular'))} {s(w)} | {s(d.get('cold'))} | {sg(f.get('singular')) if f else ''} {s(f.get('res'))} | {g('g')} | {g('h')} | {g('H')} |")
