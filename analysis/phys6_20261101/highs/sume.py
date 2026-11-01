"""ENOMOTO のログ (ENOMOTO_MIP_LOG=1) の要約: python sume.py logs/e_*.log
1 行に: カット選択の試しの選択、前景のカットのラウンド数と最後の下界 (MIP の時計)、49324 に着いたラウンド、裏のカットの写しの根の LP の反復・
ラウンド数と下界、最初の暫定解 (時刻・出どころ)、49324 の解 (時刻・出どころ)、結果。時刻は MIP の時計 (+ 下の off で呼び出しからの秒)。"""
import json, re, sys

for f in sys.argv[1:]:
    L = open(f, errors="replace").read().splitlines()
    j = json.loads([l for l in L if l.startswith("{")][-1])
    probe = next((re.search(r"-> (\w+)", l).group(1) for l in L if "cut selection probe (" in l), "-")
    rounds = []; t49 = "-"
    in_main = True
    for l in L:
        if l.startswith("Root cuts"):
            in_main = False
        m = re.match(r"MIP: cut round (\d+): \d+ cuts \(of .*obj ([\d.e+]+) \(([\d.]+)s\)", l)
        if m and in_main:
            rounds.append((int(m.group(1)), float(m.group(2)), float(m.group(3))))
            if t49 == "-" and float(m.group(2)) >= 49323.99:
                t49 = f"r{m.group(1)}@{m.group(3)}"
    rc = next((l for l in L if l.startswith("Root cuts")), "")
    m = re.search(r"-> ([\d.e+]+), (\d+) cuts in the LP \(([\d.]+)s\)", rc)
    root = f"{float(m.group(1)):.0f}@{m.group(3)}" if m else "-"
    snap = next((re.search(r"\((\d+) iters, ([\d.]+)s\)", l).groups() for l in L if "[snapshot] root LP" in l), None)
    bg = next((l for l in L if re.match(r"MIP: background cuts: \d+ rounds", l)), "")
    mb = re.search(r"(\d+) rounds.*?obj ([\d.e+]+) (?:->|not above) ([\d.e+]+)", bg)
    bgs = f"{mb.group(1)}r {float(mb.group(2)):.0f}->{float(mb.group(3)):.0f}" if mb else "-"
    if "reached the prune limit" in " ".join(L): bgs += " (cutoff)"
    incs = [(float(m.group(2)), float(m.group(1)), m.group(3)) for l in L
            for m in [re.search(r"new incumbent ([\d.e+]+) at node \d+ \(([\d.]+)s\) \[([^\]]+)\]", l)] if m]
    off = j["first_solution_time"] - incs[0][0] if incs and j.get("first_solution_time") else 0.0
    first = f"{incs[0][1]:.0f}@{incs[0][0]:.1f}[{incs[0][2]}]" if incs else "-"
    s49 = next((f"@{t:.1f}[{s}]" for t, v, s in incs if v <= 49324.001), "-")
    print(f"{f.split('/')[-1]:18s} probe {probe:7s} fg {len(rounds)}r->{root} lb49324 {t49:9s} bgLP {'/'.join(snap) if snap else '-':12s} bg {bgs:22s} "
          f"first {first:22s} sol49324 {s49:14s} {j['status']} {j['objective'] or 0:.0f} lb {j['best_bound'] or 0:.0f} n{j['nodes']} {j['time']:.1f}s off {off:.1f}")
