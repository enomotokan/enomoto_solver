# ログから、最初の暫定解が根のカットのループ (最初の求解の cut round 0 ～ cut loop done) の前・途中・後のどこで主に入ったかを数える
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "res.jsonl").read_text().splitlines() if l.strip()]
cat = {}
for r in rows:
    L = (D / "logs" / f"{r['problem']}.log").read_text().splitlines()
    inc = next((i for i, l in enumerate(L) if "MIP: new incumbent" in l), None)
    c0 = next((i for i, l in enumerate(L) if re.match(r"MIP: cut round 0:.*cuts \(of", l)), None)
    cd = next((i for i, l in enumerate(L) if l.startswith("MIP: cut loop done")), None)
    tm = lambda i: re.findall(r"\(([0-9.]+)s\)", L[i])[-1] if i is not None and re.findall(r"\(([0-9.]+)s\)", L[i]) else "-"
    src = re.findall(r"\[(.*)\]", L[inc])[0] if inc is not None and "[" in L[inc] else "-"
    if c0 is None:
        k = "カットのループなし" + ("(暫定解あり)" if inc is not None else "(暫定解なし)")
    elif inc is None:
        k = "暫定解なし (時間内)"
    elif inc < c0:
        k = "前"
    elif cd is None or inc < cd:
        k = "途中"
    else:
        k = "後"
    cat.setdefault(k, []).append((r["problem"], src, tm(inc), tm(c0), tm(cd), r["status"]))
for k, v in cat.items():
    print(f"## {k}: {len(v)} 問")
    for x in v:
        print("  ", " | ".join(map(str, x)))
