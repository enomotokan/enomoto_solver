"""run.py のログ (logs/<接頭辞>_<i>.log) から、解けた回と解けなかった回の表を作る (2026-11-01)。
使い方: python analyze.py [接頭辞 ...] (既定は全部)。
時刻は ENOMOTO のソルバーの時計 (前処理の後から) と、前処理の秒 (pre = 結果の first_solution_time − ログの最初の暫定解の時刻)。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"

def parse(p):
    s = p.read_text()
    r = {"name": p.stem}
    js = [l for l in s.splitlines() if l.startswith("{")]
    res = json.loads(js[-1]) if js else {}
    r["status"] = res.get("status"); r["obj"] = res.get("objective"); r["bound"] = res.get("bound")
    r["nodes"] = res.get("nodes"); r["time"] = res.get("time")
    inc = [(float(z), float(t), src) for z, t, src in re.findall(r"new incumbent (\S+) at node \d+ \(([\d.]+)s\) \[([^\]]*)\]", s)]
    r["inc"] = inc
    r["pre"] = res.get("first_solution_time", 0) - inc[0][1] if inc else None
    r["first"] = inc[0] if inc else None
    best = [x for x in inc if res.get("objective") is not None and abs(x[0] - res["objective"]) < 1e-6]
    r["best"] = best[0] if best else None
    m = re.search(r"LP probing: root LP ([\d.]+)s \((\d+) iters.*basis passed (\w+)", s)
    r["probe"] = (float(m.group(1)), int(m.group(2)), m.group(3)) if m else None
    m = re.search(r"root LP (\S+) \((\d+) iters, ([\d.]+)s\)", s)
    r["rootlp"] = (float(m.group(1)), int(m.group(2)), float(m.group(3))) if m else None
    r["cutrounds"] = re.findall(r"cut round (\d+): (\d+ cuts|LP status \w+)", s)
    m = re.search(r"cut loop done: (\d+) cuts added, (\d+) in LP.*-> (\S+)", s)
    r["cutdone"] = (int(m.group(1)), int(m.group(2)), float(m.group(3))) if m else None
    m = re.search(r"cut loop done.*\n(?:.*\n)*?MIP: find thread started \(budget [\d.]+s, ([\d.]+)s\)", s)
    r["cut_end"] = float(m.group(1)) if m else None
    m = re.search(r"background cuts: (\d+) rounds, obj (\S+) (.*)", s)
    r["bg"] = (int(m.group(1)), m.group(2), m.group(3)[:40]) if m else None
    r["rens_free"] = re.findall(r"parallel sub-MIP started \(RENS\): (\d+) free", s)
    r["rens"] = re.findall(r"parallel sub-MIP \(RENS\): status (\w+), nodes (\d+), objective (\S+), ([\d.]+)s", s)
    r["rens_rate"] = re.findall(r"no-incumbent RENS \(sub-MIP depth 0\) try \d+: target ([\d.]+), fixing rate ([\d.]+)", s)
    r["subs"] = re.findall(r"parallel sub-MIP \((\w[^)]*)\): status (\w+), nodes (\d+), objective (\S+), ([\d.]+)s", s)
    r["rins"] = re.findall(r"RINS[^\n]*\n[^\n]*sub-MIP: (\d+) free columns of \d+, status (\w+), nodes (\d+), objective (\S+)", s)
    r["restarts"] = re.findall(r"MIP: restart (\d+) \((\d+) of (\d+) integer columns newly fixed", s)
    m = re.search(r"tree search with \d+ threads \(([\d.]+)s\)", s)
    r["tree"] = float(m.group(1)) if m else None
    prog = re.findall(r"MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)", s)
    r["last"] = prog[-1] if prog else None
    m = re.search(r"branching: (\d+) selections, (\d+) without strong branching", s)
    r["branch"] = (int(m.group(1)), int(m.group(2))) if m else None
    m = re.search(r"sb LPs (\d+) iters (\d+)", s)
    r["sb"] = m.groups() if m else None
    r["interior"] = "interior rounding: none" not in s
    # 根のカットの道 (LP に入ったカットの本数で分ける。10 = 0・1 ラウンドだけ、15 = 3 ラウンド目の密なカット 7 本も、16 = 4 ラウンド目も)
    r["checks"] = re.findall(r"cut round (\d+): root cut time cap check: clock ([\d.]+)s of cap ([\d.]+)s", s)
    r3fail = re.search(r"cut round 3: LP status TimeLimit", s)
    inlp = r["cutdone"][1] if r["cutdone"] else (10 if r3fail else None)
    r["path"] = "P1'" if r3fail else {10: "P1", 15: "P2", 16: "P3"}.get(inlp, str(inlp))
    return r

def f(x, d=1):
    return "—" if x is None else f"{x:.{d}f}"

if __name__ == "__main__":
    pre = sys.argv[1:]
    files = sorted(p for p in D.glob("*.log") if not pre or any(p.stem.startswith(x + "_") for x in pre))
    rows = [parse(p) for p in files]
    rows.sort(key=lambda r: (r["status"] != "optimal", r["name"]))
    print("| 回 | 道 | 3 ラウンド目の前の判定 (時計 / 上限) | 結果 | 時間 | 前処理 | 最初の解 | 最終の解を見つけた時刻・出どころ | 前処理の LP (秒・反復・基底) | 根の LP (反復・秒) | カットの道 | カット後の下界 | 裏のカット | 根の RENS (固定率・列・状態・ノード・値・秒) | ほかの並列サブ MIP | 根の RINS | 再スタート | 木に入る | ノード | 最後の lb / ub |")
    print("|" + "---|" * 21)
    for r in rows:
        fi = r["first"]; b = r["best"]
        cr = ",".join(("T" if "LP" in c[1] else c[1].split()[0]) for c in r["cutrounds"])
        rens = "; ".join(f"{st} {n} {o.replace('Some(','').replace(')','')} {float(sec):.1f}s" for st, n, o, sec in r["rens"])
        rate = "/".join(a[1] for a in r["rens_rate"])
        others = "; ".join(f"{k} {st}" for k, st, n, o, sec in r["subs"] if k != "RENS")
        rins = "; ".join(f"{a} {st} {n} {o}" for a, st, n, o in r["rins"]) or "—"
        rs = "; ".join(f"{a}: {b_} 固定" for a, b_, c in r["restarts"]) or "なし"
        last = f"{float(r['last'][3]):.0f} / {float(r['last'][4]):.0f}" if r["last"] and r["last"][3] != "inf" else "—"
        ck = {a: (b_, c) for a, b_, c in r["checks"]}.get("3")
        ck = f"{ck[0]} / {ck[1]}" if ck else "—"
        print(f"| {r['name']} | {r['path']} | {ck} | {r['status']} {f(r['obj'],0)} | {f(r['time'])} | {f(r['pre'])} | {f(fi[0],0)} @{f(fi[1])} {fi[2]} | "
              + (f"{f(b[0],0)} @{f(b[1])} {b[2]}" if b else "—")
              + f" | {r['probe'][0]:.2f}・{r['probe'][1]}・{r['probe'][2]} | {r['rootlp'][1]}・{r['rootlp'][2]:.2f} | {cr} | "
              + (f"{r['cutdone'][2]:.0f} ({r['cutdone'][0]}/{r['cutdone'][1]}) @{f(r['cut_end'])}" if r['cutdone'] else "—")
              + f" | {r['bg'][0] if r['bg'] else '—'} | {rate} {'/'.join(r['rens_free'])} {rens} | {others} | {rins} | {rs} | {f(r['tree'])} | {r['nodes']} | {last} |")
    print()
    for r in rows:
        print(r["name"], "incumbents:", " ".join(f"{z:.0f}@{t:.1f}[{s}]" for z, t, s in r["inc"]), "| branch", r["branch"], "sb", r["sb"])
