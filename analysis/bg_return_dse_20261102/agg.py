"""res.jsonl とログから表を作る (run 99 は動作確認なので除く)。"""
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(D / "res.jsonl") if l.strip()]
rows = [r for r in rows if r["run"] != 99]
print("| 問題 | 設定 | 回 | 結果 | 時間 | 目的値 | 下界 | 当てはめ | 置き換えた後の LP 反復 | 同 時間 | 裏のカット |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    log = (D / "logs" / f"{r['problem']}_{r['cfg']}_{r['run']}.log").read_text()
    m = re.search(r"LP with the replaced cuts: (\w+), (\d+) iters", log)
    t = re.search(r"LP solve time with the replaced cuts ([\d.]+)s", log)
    a = re.search(r"returned DSE weights (applied|not applied|not available)", log)
    u = re.search(r"background cuts: (\d+) rounds, (\d+) cut rows", log)
    nu = "使わない" if "not used (waited" in log or "not above" in log else ("使った" if u else "-")
    it = m.group(2) if m else "-"
    ts = t.group(1) if t else "-"
    ap = a.group(1) if a else "-"
    ob = r.get("objective"); bd = r.get("bound")
    f = lambda v: "-" if v is None else f"{v:.6g}"
    print(f"| {r['problem']} | {r['cfg']} | {r['run']} | {r['status']} | {r['time']:.1f} | {f(ob)} | {f(bd)} | {ap} | {it} | {ts} | {nu} |")
