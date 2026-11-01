"""logs/<問題>_<設定>_<回>.log の集計 (ENOMOTO_MIP_SUBMIP_VERBOSE=1 の `SMT` の行を読む)。
使い方: python summ.py [問題 ...] > summ.txt
列: 結果・時間 (前処理を含む通し)・根のカット (足した本数/LP に残った本数)・根の RENS (自由な列・ノード・秒・目的値)・
外側の根のヒューリスティクスの秒 (`SMT rooth tag=M restarts=0`) とその時点の暫定解・最初に -238 などの最良値を出した時刻 (ソルバーの時計) と出どころ・
試作 (ENOMOTO_MIP_SUBMIP_NESTED) で深さ 1 の中から作った深さ 2 のサブ MIP (locks・clique・vbounds・RINS・root reduced cost) の本数・秒の和・受け入れた本数"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
KV = re.compile(r"(\w+)=(\[[^\]]*\]|\S+)")
NEST = {"locks", "clique", "vbounds (loose)", "vbounds (tight)", "RINS", "root reduced cost"}
def parse(f):
    res, cut, subs, inc, rooth, rooth_inc = {}, None, {}, [], None, None
    for line in open(f, errors="replace"):
        if line.startswith("{") and '"status"' in line:
            res = json.loads(line)
        elif cut is None and line.startswith("MIP: cut loop done:"):
            m = re.search(r"(\d+) cuts added, (\d+) in LP", line)
            cut = f"{m.group(1)}/{m.group(2)}"
        elif line.startswith("MIP: SMT "):
            ev = line.split()[2]
            kv = dict(KV.findall(line))
            if ev == "begin":
                subs[int(kv["id"])] = dict(parent=kv["parent"], depth=int(kv["depth"]), kind=kv["kind"][1:-1], node=int(kv["caller_node"]), free=kv["free"].split("/")[0])
            elif ev == "end" and int(kv["id"]) in subs:
                subs[int(kv["id"])].update(status=kv["status"], nodes=int(kv["nodes"]), obj=kv["objective"], secs=float(kv["secs"]))
            elif ev == "acc" and int(kv["id"]) in subs:
                subs[int(kv["id"])]["acc"] = kv["accepted"] == "1"
            elif ev == "inc" and kv["tag"] == "M":
                inc.append((float(kv["z"]), float(kv["t"]), kv["src"][1:-1]))
            elif ev == "rooth" and kv["tag"] == "M" and kv["restarts"] == "0" and rooth is None:
                rooth = float(kv["secs"])
                rooth_inc = inc[-1][0] if inc else None
    return res, cut, subs, inc, rooth, rooth_inc
def row(f):
    res, cut, subs, inc, rooth, rooth_inc = parse(f)
    st = {"optimal": "最適", "time_limit": "T"}.get(res.get("status"), res.get("status"))
    rens = next((s for s in subs.values() if s["parent"] == "M" and s["depth"] == 1 and s["kind"] == "RENS" and s["node"] == 0), None)
    rs = f"{rens['free']} 列・{rens.get('nodes')} ノード・{rens.get('secs', 0):.1f} 秒・{rens.get('obj')}" if rens else "—"
    best = min((z for z, _, _ in inc), default=None)
    first = next(((t, s) for z, t, s in inc if z == best), None)
    nest = [s for s in subs.values() if s["depth"] == 2 and s["kind"] in NEST]
    ns = f"{len(nest)} 本・{sum(s.get('secs', 0) for s in nest):.1f} 秒・受入 {sum(s.get('acc', False) for s in nest)}" if nest else "—"
    rh = f"{rooth:.1f} 秒 ({rooth_inc:g})" if rooth is not None and rooth_inc is not None else "—"
    fb = f"{best:g} @{first[0]:.1f} {first[1]}" if first else "—"
    return f"| {f.stem} | {st} {res.get('objective')} | {res.get('time', 0):.1f} | {cut} | {rs} | {rh} | {fb} | {res.get('nodes')} | {ns} |"
if __name__ == "__main__":
    probs = sys.argv[1:] or sorted({f.stem.rsplit("_", 2)[0] for f in D.glob("*_*_*.log") if not f.stem.startswith("dump")})
    for p in probs:
        print(f"\n### {p}\n")
        print("| 回 | 結果 | 時間 | 根のカット | 根の RENS | 根のヒューリスティクス (その時の暫定解) | 最良値 @時刻 出どころ | ノード | 深さ 2 の試作のサブ MIP |")
        print("|---|---|---|---|---|---|---|---|---|")
        for f in sorted(D.glob(f"{p}_*_*.log"), key=lambda f: (f.stem.rsplit("_", 2)[1], int(f.stem.rsplit("_", 1)[1]))):
            if f.stem.rsplit("_", 2)[0] == p:
                print(row(f))
