"""bgdse の集計: logs/ と res.jsonl から 1 回ごとの表 (Markdown) を出す。
使い方: python summ.py [問題,...] [設定,...]
写しの行は「background cuts: basis with ...」(写しを起こした) から「background cuts: N rounds」(写しの終わり) までの
`cut round` の行 (この間、主はカットのループを回さない)。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
probs = sys.argv[1].split(",") if len(sys.argv) > 1 and sys.argv[1] else None
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
rows = [json.loads(l) for l in open(D / "res.jsonl")]


def parse(log):
    t = log.read_text(errors="replace").splitlines()
    s = next((i for i, l in enumerate(t) if "background cuts: basis with the column values" in l or "background cuts: columns with global" in l), None)
    if s is None:
        return None
    e = next((i for i in range(s, len(t)) if re.search(r"background cuts: \d+ rounds, (obj|\d+ cut rows)", t[i])), len(t) - 1)
    w = t[s:e + 1]
    o = {}
    m = next((m for m in (re.search(r"\[snapshot\] root LP (\w+) \((\d+) iters, ([\d.]+)s", l) for l in w) if m), None)
    o["root"] = f"{m.group(2)} 反復 {m.group(3)}s" if m else "-"
    o["passed"] = "passed" if any("column values passed" in l for l in w) else ("not" if any("column values not passed" in l for l in w) else "-")
    o["dse"] = "渡した" if any("DSE weights of the main thread applied" in l for l in w) else ("取れず" if any("DSE weights not available" in l for l in w) else ("根で厳密" if any("root LP from the exact DSE weights" in l for l in w) else ("厳密" if any("from the exact DSE weights" in l for l in w) else "")))
    lps, fails = [], 0
    for l in w:
        m = re.search(r"cut round (\d+): \d+ cuts .*LP (\d+) iters", l)
        if m:
            lps.append(m.group(2))
        m = re.search(r"cut round (\d+): LP status (\w+), removed the cuts .*cut LP (\d+) iters ([\d.]+)s", l)
        if m:
            fails += 1
            lps.append(f"✗{m.group(3)}({m.group(2)[:4]} {m.group(4)}s)")
    o["lps"] = " ".join(lps[:8]) + (" …" if len(lps) > 8 else "")
    o["fails"] = fails
    o["ok"] = len(lps) - fails
    m = next((re.search(r"rounds so far \[(.*)\]", l) for l in t if "rounds so far" in l), None)
    if m:
        xs = [x.strip() for x in m.group(1).split(",") if x.strip()]
        o["prog"] = f"{len(xs)}" + (f" (最後 {xs[-1].split(' at ')[0]})" if xs else "")
    else:
        o["prog"] = "-"
    ends = [re.search(r"background cuts: (\d+) rounds, (.*)", l) for l in t if re.search(r"background cuts: \d+ rounds, (obj|\d+ cut rows)", l)]
    def fin(m):
        g = m.group(2)
        if "->" in g:
            v = re.search(r"-> (\S+)", g).group(1)
            return f"{m.group(1)} ラウンド・使う ({float(v):.6g})"
        v = re.search(r"obj (\S+)", g)
        return f"{m.group(1)} ラウンド・不採用 ({float(v.group(1)):.6g})" if v and v.group(1) != "None" else f"{m.group(1)} ラウンド・不採用"
    o["end"] = " / ".join(fin(m) for m in ends) or "-"
    return o


print("| 問題 | 設定 | 回 | 結果 | 時間 | 写しの根の LP | 重み | 写しのカットの LP の反復 (✗ = 外した) | 通った LP | 失敗 | 止めの合図の時点の記録 | 写しの終わり |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    if probs and r["prob"] not in probs or cfgs and r["cfg"] not in cfgs:
        continue
    log = D / "logs" / f"{r['prob']}_{r['cfg']}_{r['run']}.log"
    o = parse(log) if log.exists() else None
    ob = r.get("objective")
    res = f"{r['status']} {'' if ob is None else f'{ob:.6g}'} / 下界 {r.get('bound', float('nan')):.6g}"
    tm = f"{r['time']:.1f}" if r.get("time") is not None else "-"
    if o is None:
        print(f"| {r['prob']} | {r['cfg']} | {r['run']} | {res} | {tm} | 写しなし | | | | | | |")
    else:
        print(f"| {r['prob']} | {r['cfg']} | {r['run']} | {res} | {tm} | {o['root']} ({o['passed']}) | {o['dse']} | {o['lps']} | {o['ok']} | {o['fails']} | {o['prog']} | {o['end']} |")
