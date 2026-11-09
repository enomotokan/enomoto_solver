"""GAP4: ENOMOTO_MIP_LOG のログ (logs/<名前>.log.gz) から時間の行き先を抜き出す。
前処理 (外側の時計 − 求解の時計)、根の LP の終わり、カットの選び方の比べ、カットのループ (最後のラウンドの時刻)、根のヒューリスティクス
(各 "root heuristic" の秒の和と、始めから "after root heuristics" まで)、裏のカットを主で続けた分、強分枝の秒、木の始まり、ノードの LP の秒、反復数。
使い方: python summ_time.py 名前 ..."""
import gzip, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
F = lambda s: float(s)
for name in sys.argv[1:]:
    L = gzip.open(D / "logs" / f"{name}.log.gz", "rt", errors="replace").read().splitlines()
    r = {"rootlp": None, "probe": 0.0, "cut_end": None, "heur_sum": 0.0, "heur_end": None, "bgcont": 0.0, "sb": 0.0, "tree_solve": None,
         "tree_outer": None, "nodelp": None, "restarts": 0, "fixlp_fail": 0, "heur": {}, "cutrounds": 0, "cutlp_iters": 0, "rootlp_iters": None}
    first_heur = None
    for l in L:
        m = re.search(r"MIP: root LP [-0-9.e+]+ \((\d+) iters, ([0-9.]+)s\)", l)
        if m and r["rootlp"] is None: r["rootlp"] = F(m.group(2)); r["rootlp_iters"] = int(m.group(1))
        m = re.search(r"cut selection probe: compared .* probe wall ([0-9.]+)s", l)
        if m: r["probe"] += F(m.group(1))
        m = re.search(r"MIP: cut round \d+: \d+ cuts .*LP (\d+) iters.*\(([0-9.]+)s\)$", l)
        if m:
            r["cutrounds"] += 1; r["cutlp_iters"] += int(m.group(1))
            if r["heur_end"] is None: r["cut_end"] = F(m.group(2))
        m = re.search(r"MIP: root heuristic (.+?): \w+ \(([0-9.]+)s\)", l)
        if m:
            r["heur_sum"] += F(m.group(2)); r["heur"][m.group(1)] = r["heur"].get(m.group(1), 0) + F(m.group(2))
        m = re.search(r"after root heuristics: .*\(([0-9.]+)s\)", l)
        if m: r["heur_end"] = F(m.group(1))
        m = re.search(r"background cuts: determ: continued in the main cut loop: .*\(([0-9.]+)s, ([0-9.]+)s\)", l)
        if m: r["bgcont"] += F(m.group(1))
        m = re.search(r"sb_secs ([0-9.]+)", l)
        if m: r["sb"] = F(m.group(1))
        m = re.search(r"TRSTAT: tree start outer ([0-9.]+) solve ([0-9.]+) restarts (\d+)", l)
        if m: r["tree_outer"], r["tree_solve"], r["restarts"] = F(m.group(1)), F(m.group(2)), int(m.group(3))
        m = re.search(r"node diag: nodes (\d+) node LPs (\d+) .*iters (\d+) .*LP secs ([0-9.]+)", l)
        if m: r["nodelp"] = (int(m.group(1)), int(m.group(3)), F(m.group(4)))
        if "fixed-integer LP: TimeLimit" in l: r["fixlp_fail"] += 1
        m = re.search(r"s nodes +(\d+) .*lp_iters (\d+) sb_iters (\d+).*node_iters (\d+).*heur_iters (\d+) dive_iters (\d+)", l)
        if m: r["last"] = tuple(int(x) for x in m.groups())
        if l.startswith("{"): r["res"] = l
    pre = (r["tree_outer"] - r["tree_solve"]) if r["tree_outer"] else None
    top = sorted(r["heur"].items(), key=lambda kv: -kv[1])[:5]
    print(f"== {name}")
    print(f"  pre {pre and round(pre,1)} | rootLP end {r['rootlp']} ({r['rootlp_iters']} it) | probe {r['probe']:.1f} | cut rounds {r['cutrounds']} ({r['cutlp_iters']} it) end {r['cut_end']}"
          f" | heur sum {r['heur_sum']:.1f} end {r['heur_end']} | bgcont {r['bgcont']:.1f} | tree start (solve) {r['tree_solve']} restarts {r['restarts']} | sb {r['sb']:.1f} | nodeLP {r['nodelp']} | fixLP fail {r['fixlp_fail']}")
    print(f"  heur top: " + ", ".join(f"{k} {v:.1f}" for k, v in top))
    print(f"  last (nodes, lp_iters, sb_iters, node_iters, heur_iters, dive_iters) {r.get('last')}")
    print(f"  {r.get('res','')[:200]}")
