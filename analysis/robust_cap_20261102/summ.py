"""robust_cap の集計: logs/ と res.jsonl から 1 回ごとの表 (Markdown) を出す。
使い方: python summ.py [問題,...] [設定,...]
写しの行は「background cut LP work cap」(写しのカットのループの始め) から「background cuts: ... N rounds」までの `cut round` の行、
それ以外の `cut round` の行を主のカットのループとする (主の根のカットのループは写しの前に終わる)。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
probs = sys.argv[1].split(",") if len(sys.argv) > 1 and sys.argv[1] else None
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 and sys.argv[2] else None
rows = [json.loads(l) for l in open(D / "res.jsonl")]


def part(w):
    o = {"ok": 0, "fail": 0, "dbl": 0, "dblok": 0, "prog": 0, "past": 0, "maxit": 0, "lps": [], "end": None}
    for l in w:
        m = re.search(r"cut round (\d+): \d+ cuts .*LP (\d+) iters\), LP rows", l)
        if m:
            o["ok"] += 1
            o["maxit"] = max(o["maxit"], int(m.group(2)))
            o["lps"].append(m.group(2))
            o["end"] = re.search(r"obj ([-\d.e+]+)", l).group(1)
        m = re.search(r"cut round (\d+): LP status (\w+), removed the cuts .*cut LP (\d+) iters ([\d.]+)s", l)
        if m:
            o["fail"] += 1
            o["lps"].append(f"✗{m.group(3)}({m.group(2)[:4]} {m.group(4)}s)")
        m = re.search(r"continued with twice the cap (\d+): (\w+) after (\d+) more", l)
        if m:
            o["dbl"] += 1
            o["dblok"] += m.group(2) == "Optimal"
            o["lps"].append(f"↻{m.group(2)[:3]}+{m.group(3)}")
        if "stopped by the progress check" in l:
            o["prog"] += 1
        m = re.search(r"went past the cap (\d+) on progress: (\w+) after (\d+)", l)
        if m:
            o["past"] += 1
            o["lps"].append(f"→{m.group(3)}/{m.group(1)}")
    return o


def parse(log):
    t = log.read_text(errors="replace").splitlines()
    s = next((i for i, l in enumerate(t) if "background cut LP work cap" in l), None)
    e = next((i for i in range(s, len(t)) if re.search(r"background cuts: .*\d+ rounds, obj", t[i])), len(t) - 1) if s is not None else None
    main = t if s is None else t[:s] + t[e + 1:]
    bg = [] if s is None else t[s:e + 1]
    o = {"main": part(main), "bg": part(bg)}
    m = next((re.search(r"(\d+) rounds, obj (\S+)", l) for l in bg[-1:] if re.search(r"\d+ rounds, obj", l)), None)
    o["bgend"] = f"{m.group(1)} ラウンド {m.group(2).replace('Some(', '').rstrip(')')[:9]}" if m else "-"
    m = next((re.search(r"root cut loop.*?([\d.]+)s", l) for l in t if "MIP: root cuts done" in l), None)
    return o


def show(o, k):
    p = o[k]
    return f"{p['ok']}/{p['fail']}" + (f" ↻{p['dblok']}/{p['dbl']}" if p["dbl"] else "") + (f" p{p['prog']}" if p["prog"] else "") + (f" →{p['past']}" if p["past"] else "") + f" max {p['maxit']}"


print("| 問題 | 設定 | 回 | 結果 | 秒 | 目的値 | 主 (解けた/外した、↻倍で続けて解けた/続けた、p 進みで打ち切り、→上限を超えて解けた、最大反復) | 写し (同じ) | 写しの終わり |")
print("|---|---|---|---|---|---|---|---|---|")
for r in rows:
    if probs and r["prob"] not in probs or cfgs and r["cfg"] not in cfgs:
        continue
    log = D / "logs" / f"{r['prob']}_{r['cfg']}_{r['run']}.log"
    o = parse(log)
    obj = r.get("objective")
    print(f"| {r['prob']} | {r['cfg']} | {r['run']} | {r['status']} | {r['time'] and round(r['time'], 1)} | {obj if obj is None else round(obj, 3)} | {show(o, 'main')} | {show(o, 'bg')} | {o['bgend']} |")
    if "-v" in sys.argv:
        print(f"|  |  |  |  |  |  | {' '.join(o['main']['lps'][:40])} | {' '.join(o['bg']['lps'][:60])} | |")
