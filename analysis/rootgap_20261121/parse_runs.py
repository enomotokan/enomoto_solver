"""SCIP のログ (logs/scip_<問題>_<設定>.log) から、走り (run) ごとの根の終わりの様子を抜き出す。

使い方: python parse_runs.py 問題 [設定=def]
走りごとに: 終わりの時刻・LP 反復・列数・行数・カットの本数・分離の巡数 (sepa)・双対の下界・主の上界、再スタートの理由 (大域の固定の数)。"""
import re, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent

def main():
    prob = sys.argv[1]; s = sys.argv[2] if len(sys.argv) > 2 else "def"
    tag = s.replace(":", "_").replace("/", "-").replace(";", "+").replace("=", "")
    last, run = None, 1
    for l in (HERE / "logs" / f"scip_{prob}_{tag}.log").read_text().splitlines():
        if "|" in l and re.match(r"^.?\s*\d+\.\ds\|", l):
            c = [x.strip() for x in l[1:].split("|")]
            last = dict(t=c[0], node=c[1], lpit=c[3], heur=c[5], vars=c[7], rows=c[9], cuts=c[10], sepa=c[11], dual=c[14], primal=c[15], gap=c[16])
            if l[0] != " ": print(f"  解 (run {run}): {l[0]} {c[5]} t {c[0]} 主 {c[15]} 双対 {c[14]}")
        m = re.match(r"\(run (\d+), node (\d+)\) restarting after (\d+) global fixings", l)
        if m:
            print(f"run {run} の終わり: {last} → 再スタート (大域の固定 {m.group(3)})"); run += 1
    print(f"run {run} の終わり (最後): {last}")

if __name__ == "__main__":
    main()
