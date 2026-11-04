"""tree_close3: res.jsonl の 600 秒の結果を問題 x 設定の表にする (Markdown)。使い方: python summ600.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "res.jsonl").read_text().splitlines() if l.strip()]
P = ["tbfp-network", "neos-873061", "neos-3216931-puriri", "satellites2-60-fs", "mzzv11", "net12"]
C = [("ours", "def"), ("ours", "def2"), ("ours", "jc"), ("ours", "t3"), ("highs", "def")]
def cell(r):
    x = r["result"]; o, b, n = x.get("objective"), x.get("bound"), x.get("nodes")
    st = "最適" if str(x.get("status")).lower().startswith("optimal") else "時間切れ"
    if st == "最適":
        return f"最適 {x['time']:.0f} 秒 ({n})"
    if o is None:
        return f"解なし, lb {b:.6g} ({n})"
    g = abs(o - b) / max(abs(o), abs(b), 1e-9)
    return f"{o:.6g} / {b:.6g} ({g*100:.1f}%, {n})"
print("| 問題 | " + " | ".join(f"{s} {c}" for s, c in C) + " |")
print("|---" * (len(C) + 1) + "|")
for p in P:
    cs = []
    for s, c in C:
        rs = [r for r in rows if r["problem"] == p and r["solver"] == s and r["cfg"] == c]
        cs.append("; ".join(cell(r) for r in rs) if rs else "-")
    print(f"| {p} | " + " | ".join(cs) + " |")
