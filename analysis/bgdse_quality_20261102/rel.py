"""bgdse_quality: 標本のずれ (中央値 |ln(厳密/今)|) と、写しのカットの LP の反復・失敗の関係の表 (Markdown) を出す。
使い方: python rel.py [設定,...]   (summ.py の parse を使う)"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.argv = [sys.argv[0]] + sys.argv[1:]
cfgs = sys.argv[1].split(",") if len(sys.argv) > 1 else None
import importlib.util
spec = importlib.util.spec_from_file_location("summ", D / "summ.py")
src = (D / "summ.py").read_text().split("\nprint(")[0]  # 表の出力の前まで
ns = {"__file__": str(D / "summ.py")}
_argv = sys.argv; sys.argv = [sys.argv[0]]
exec(compile(src, "summ.py", "exec"), ns)
sys.argv = _argv
rows = [json.loads(l) for l in open(D / "res.jsonl")]
print("| 問題 | 設定 | 回 | 写しの根の LP | 全体 中央値 \\|ln\\| | 平均 ln | ばらつき | 実行不能 行数 | 実行不能 中央値 \\|ln\\| | 実行不能 ばらつき | 標本 (秒) | 厳密 (秒) | 最初のカットの LP | 2〜4 本目 | 失敗 | 写しの終わり |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
out = []
f = lambda v: "-" if v is None else f"{v:.3f}"
for r in sorted(rows, key=lambda r: (r["prob"], r["cfg"], r["run"])):
    if cfgs and r["cfg"] not in cfgs:
        continue
    o = ns["parse"](D / "logs" / f"{r['prob']}_{r['cfg']}_{r['run']}.log")
    if o is None:
        continue
    s = o["sampd"]
    lps = o["lps"].split()
    first = lps[0] if lps else "-"
    rest = " ".join(lps[1:4])
    if s:
        print(f"| {r['prob']} | {r['cfg']} | {r['run']} | {o['root']} ({o['passed']}) | {s['med']:.3f} | {s['mean']:.3f} | {f(s['spread'])} | {s['ninf'] if s['ninf'] is not None else '-'} | {f(s['imed'])} | {f(s['ispread'])} | {s['secs']:.3f} | {s['full']} {s['full_secs']:.3f} | {first} | {rest} | {o['fails']} | {o['end']} |")
    else:
        print(f"| {r['prob']} | {r['cfg']} | {r['run']} | {o['root']} ({o['passed']}) | - | - | - | - | - | - | - | {o['dse'] or '-'} | {first} | {rest} | {o['fails']} | {o['end']} |")
