"""ab.jsonl を問題ごとの表 (Markdown) にする。"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = [json.loads(l) for l in (D / "ab.jsonl").read_text().splitlines()]
by = defaultdict(list)
for r in R:
    by[(r["prob"], r["cfg"])].append(r)


def rounds(r):
    """写しのラウンド: 止めの合図の時点の記録 (rounds so far) の数・最後の値、終わりの N rounds、使われたか"""
    sf = r["so_far"]
    n = 0 if not sf else len(sf.split(", "))
    m = re.match(r"(\d+) rounds", r["bg_end"] or "")
    end = m.group(1) if m else "-"
    used = {True: "使う", False: "不使用", None: "-"}[r["bg_used"]]
    last = sf.split(", ")[-1].split(" at ")[0] if sf else ""
    return f"{n if sf is not None else '-'}/{end} {used}" + (f" ({last})" if last else "")


def fmt(r):
    st = {"optimal": "最適", "time_limit": "時間切れ"}.get(r["status"], r["status"])
    t = f"{r['time']:.1f}" if r["status"] == "optimal" else ""
    rb = r.get("root_bound")
    return f"{st}{(' ' + t) if t else ''}", rounds(r), (r["bgcopy"] or "-").replace(" it ", " 反復 "), f"{rb:.6g}" if rb is not None else "-", f"{r['objective']:.6g}" if r.get("objective") is not None else "なし"


print("| 問題 | 形 | 回 | 列の値 | 結果 | 写しのラウンド (記録/終わり 使ったか (最後の値)) | 写しの根の LP | 根の下界 | 解 |")
print("|---|---|---|---|---|---|---|---|---|")
for (p, c), rs in by.items():
    for r in sorted(rs, key=lambda r: r["rep"]):
        a, b, cpy, rb, ob = fmt(r)
        print(f"| {p} | {c} | {r['rep']} | {r['bx'] or '-'} | {a} | {b} | {cpy} | {rb} | {ob} |")
