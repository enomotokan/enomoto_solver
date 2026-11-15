"""600 秒の回の集計: 回ごとの暫定解・下界・ノード、暫定解の推移 (時刻と出どころ)、SUBB の計数 (サブ MIP・指紋の繰り返し・多様化・
サブ MIP の中の対称性)、HDIAG (木の改善ヒューリスティクスのサブ MIP) の回数。使い方: python summ600.py [--trace]"""
import json, re, sys
from pathlib import Path
L = Path(__file__).resolve().parent / "logs"
PROBS = ["fiball", "neos-933966", "trento1", "nexp-150-20-8-5", "thor50dday"]
trace = "--trace" in sys.argv
for p in PROBS:
    print(f"\n## {p}\n")
    print("| 回 | 暫定解 | 下界 | ノード | 最後の改善 | 外側のサブ MIP | 同じ指紋 | 広げ/狭め/連続 | プール 交差/変異 | 変えた回の改善 | 木のサブ MIP (HDIAG) | サブ MIP の検出 (対称性あり) | 生成元 | オービトープ (列) | 固定 (orbitopal) | 検出の秒 |")
    print("|---|---:|---:|---:|---|---:|---:|---|---|---:|---:|---|---:|---|---:|---:|")
    for out in sorted(L.glob(f"*_{p}.out")):
        tag = out.name[: -len(f"_{p}.out")]
        try:
            r = json.loads([l for l in out.read_text().splitlines() if l.startswith("{")][-1])
        except Exception:  # noqa: BLE001
            continue
        err = (L / f"{tag}_{p}.err").read_text(errors="replace")
        inc = re.findall(r"new incumbent (\S+) at node \d+ \(([\d.]+)s\) \[([^\]]*)\]", err)
        last = f"{float(inc[-1][1]):.0f} 秒 {inc[-1][2]}" if inc else ""
        d = re.search(r"SUBB div: sub-MIPs (\d+), repeated fingerprints (\d+), widened (\d+), narrowed (\d+), changed after a streak (\d+), pool crossovers (\d+), pool mutations (\d+), improved after a change (\d+)", err)
        s = re.search(r"SUBB sym: sub-MIP detections (\d+), with symmetry (\d+), generators (\d+), orbitopes (\d+) \(columns (\d+)\), orbitopal fixings (\d+), orbital fixings (\d+), detection ([\d.]+)s", err)
        hd = err.count("\nHDIAG:")
        dc = (f"{d[1]} | {d[2]} | {d[3]}/{d[4]}/{d[5]} | {d[6]}/{d[7]} | {d[8]}") if d else "- | - | - | - | -"
        sc = (f"{s[1]} ({s[2]}) | {s[3]} | {s[4]} ({s[5]}) | {s[6]} | {s[8]}") if s else "- | - | - | - | -"
        print(f"| {tag} | {r.get('objective')} | {r.get('bound')} | {r.get('nodes')} | {last} | {dc} | {hd} | {sc} |")
        if trace:
            print("    " + ", ".join(f"{float(t):.0f}s {float(z):.6g} [{w}]" for z, t, w in inc[-8:]))
