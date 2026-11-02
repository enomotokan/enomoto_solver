"""問題・設定ごとのまとめ (summ.py の parse を使う)。使い方: python agg.py"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.argv = [sys.argv[0], ""]
import importlib.util
spec = importlib.util.spec_from_file_location("summ", D / "summ.py")
src = (D / "summ.py").read_text().split("\nprint(")[0]
ns = {"__file__": str(D / "summ.py")}
exec(src, ns)
parse = ns["parse"]
rows = [json.loads(l) for l in open(D / "res.jsonl")]
g = defaultdict(list)
for r in rows:
    g[(r["prob"], r["cfg"])].append(r)
print("| 問題 | 設定 | 回数 | 解けた | 時間 (秒) | 写しのカットの LP (通った/外した/半分化) | 写しの終わりのラウンド | 写しを使った回 | 写しの最後の目的値 |")
print("|---|---|---|---|---|---|---|---|---|")
for (p, c), rs in g.items():
    os_ = [parse(D / "logs" / f"{p}_{c}_{r['run']}.log") for r in rs]
    ok = sum(r["status"] == "optimal" for r in rs)
    tm = ", ".join(f"{r['time']:.1f}" for r in rs)
    lp = ", ".join(f"{o['ok']}/{o['fails']}/{o['halves']}" if o else "-" for o in os_)
    end = [o["end"] if o else "-" for o in os_]
    rnd = ", ".join(e.split(" ラウンド")[0] for e in end)
    used = sum("使う" in e for e in end)
    last = ", ".join((e.split("(")[-1].rstrip(")") if "(" in e else "-") for e in end)
    print(f"| {p} | {c} | {len(rs)} | {ok}/{len(rs)} | {tm} | {lp} | {rnd} | {used}/{len(rs)} | {last} |")
