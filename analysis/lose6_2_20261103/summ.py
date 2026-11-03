"""lose6_2: ENOMOTO のログ (logs/<問題>_<設定>_<回>.log) から時間の内訳を 1 回 1 行の表 (Markdown) にする。
時刻は MIP の時計 (外側の前処理の後に 0)。「外」は呼び出しからの秒 − MIP の時計の終わり (外側の前処理・LP の探りの根の LP・後処理)。
使い方: python summ.py 問題 [設定,...]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
P = sys.argv[1]
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
res = {}
for l in (D / "res.jsonl").read_text().splitlines():
    r = json.loads(l)
    res[(r["problem"], r["cfg"], r["run"])] = r
def f(pat, s, g=1, conv=float, last=False):
    m = re.findall(pat, s)
    if not m: return None
    v = m[-1] if last else m[0]
    if isinstance(v, tuple): v = v[g - 1]
    return conv(v)
rows = []
for (p, c, i), r in sorted(res.items(), key=lambda x: (x[0][1], x[0][2])):
    if p != P or c == "highs" or (cfgs and c not in cfgs): continue
    lf = D / "logs" / f"{p}_{c}_{i}.log"
    if not lf.exists(): continue
    s = lf.read_text()
    end = f(r"solve returned \w+ after ([\d.]+)s", s)
    tot = r["time"]
    fin0 = f(r"finish start \w+ \(([\d.]+)s\)", s)
    outer = end - fin0 if end and fin0 else None
    race = re.search(r"MIP root race: (\w+) won at ([\d.]+)s", s)
    lpp = f(r"LP probing: root LP ([\d.]+)s", s)
    rlp = re.search(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", s)
    probe = re.search(r"cut selection probe \(\d+ rounds\):.*-> (\w+) \(rule \w+, probe ([\d.]+)s in total, ([\d.]+)s\)", s)
    # 主の根のカットのループの最後 (最初の "cut loop done")
    cl = re.search(r"cut loop done: .* obj (\S+) -> (\S+)", s)
    rounds_main = None
    if cl:
        pre = s[:cl.start()]
        rr = re.findall(r"MIP: cut round (\d+): \d+ cuts .*\(([\d.]+)s\)", pre)
        rounds_main = (int(rr[-1][0]) + 1, float(rr[-1][1])) if rr else None
    heur_end = f(r"after root heuristics: incumbent \S+ \(([\d.]+)s\)", s)
    bgw = re.search(r"background cuts: determ: waited ([\d.]+)s \(safety cap ([\d.]+)s, (\w+), background end ([^)]*)\), (\d+) rounds", s)
    bgres = re.search(r"background cuts: (\d+) rounds, (?:\d+ cut rows \(was \d+\), )?obj (\S+) (?:->|not above) (\S+)", s)
    used = None
    if bgres: used = "not above" not in bgres.group(0)
    rob = re.search(r"root objective probing: gap .*?, ([\d.]+)s\)", s)
    robp = f(r"root objective probing proved the incumbent \S+ optimal \(([\d.]+)s\)", s)
    obj = r.get("objective")
    inc = re.findall(r"new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[([^\]]+)\]", s)
    first_opt = None
    if obj is not None:
        for z, nd, t, src in inc:
            if abs(float(z) - obj) <= 1e-6 * max(1, abs(obj)):
                first_opt = f"{float(t):.1f} ({src}{', node ' + nd if nd != '0' else ''})"; break
    fin = re.search(r"finish start (\w+) \(([\d.]+)s\)", s)
    nodes = r.get("nodes")
    rst = len(re.findall(r"MIP: restart", s))
    rb = r.get("root_bound")
    rows.append([c, i, r["status"], f"{tot:.1f}", f"{outer:.1f}" if outer else "",
                 f"{race.group(1)} {float(race.group(2)):.1f}" if race else "", f"{lpp:.1f}" if lpp else "",
                 f"{float(rlp.group(3)):.1f}" if rlp else "",
                 f"{probe.group(1)} {float(probe.group(2)):.1f}" if probe else "",
                 f"{rounds_main[0]}r {float(cl.group(2)):.6g} @{rounds_main[1]:.1f}" if cl and rounds_main else "",
                 f"{heur_end:.1f}" if heur_end else "",
                 f"{float(bgw.group(1)):.1f} ({bgw.group(3)}, {bgw.group(5)}r)" if bgw else "",
                 (f"{float(bgres.group(3 if used else 2).rstrip(';')):.6g}" + ("" if used else " 不採用")) if bgres else "",
                 (f"{float(rob.group(1)):.1f}" + (" 証明" if robp else "")) if rob else "",
                 f"{rb:.6g}" if rb is not None else "", first_opt or "", f"{float(fin.group(2)):.1f}" if fin else "", nodes, rst])
hdr = ["設定", "回", "結果", "計", "外", "根の LP の同時実行 (勝ち・秒)", "LP の探りの根の LP", "MIP の根の LP", "カット選択の試し (選択・秒)",
       "主のカットのループ (ラウンド・下界・終わり)", "根のヒューリの終わり", "裏のカットの待ち (秒、理由、ラウンド)", "裏のカットの下界",
       "根の目的値のプロービング (秒)", "根の下界", "最適解 (MIP の時計、出どころ)", "終わり (MIP の時計)", "ノード", "再スタート"]
print("| " + " | ".join(hdr) + " |")
print("|" + "---|" * len(hdr))
for r in rows:
    print("| " + " | ".join(str(x) for x in r) + " |")
