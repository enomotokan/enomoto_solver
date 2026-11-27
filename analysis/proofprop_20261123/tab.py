"""logs/*.log (python tab.py [ディレクトリ]) の区間計時 (XPROF 主・XPROFW 従の合計) から m_proof の内訳の表を出す。python tab.py > tab.txt"""
import glob, json, re
from pathlib import Path
D = Path(__file__).resolve().parent
KEYS = ["m_pobj", "p_refresh", "p_delta", "p_scan", "p_rowprop", "p_conflict", "m_pdual", "m_plit", "m_proof"]
CNT = ["c_refresh", "c_dirty"]
print("| 回 | 状態・秒・ノード | 目的 | 証明 作り直し | 証明 差分 | 証明 走査 | 証明 後の行の伝播 | 証明 刈った後の衝突 | 証明 残り | リテラル | カットプール | 計 (秒) | 作り直しの回数 | 差分の作り直しの回数 |")
print("|---|---|" + "---:|" * 12)
import sys
for f in sorted(glob.glob(str(D / (sys.argv[1] if len(sys.argv) > 1 else "logs") / "*.log"))):
    accm, accw = {}, {}; res = {}
    blocks = {}; cur = None
    for l in open(f):
        if l.startswith("XPROFBLOCK"):
            cur = blocks[l.split(None, 1)[1].strip()] = {}
            continue
        m = re.match(r"XPROFW? (\S+)\s+([\d.]+) ms", l)
        if m:
            # 主のスレッドの XPROF は外側の求解の終わりに貯めた値を出す (再スタートで 2 回出れば後の方)。従の XPROFW は出すたびに空にするので足す
            if l.startswith("XPROFW"):
                accw[m.group(1)] = accw.get(m.group(1), 0) + float(m.group(2))
            elif cur is not None:
                cur[m.group(1)] = float(m.group(2))
        if l.startswith("{"): res = json.loads(l)
    # スレッドごとの最後の塊を足す (主のスレッドの外側の求解・解探しのスレッドなど)
    for b in blocks.values():
        for k, x in b.items():
            accm[k] = accm.get(k, 0) + x
    acc = {k: accm.get(k, 0) + accw.get(k, 0) for k in set(accm) | set(accw)}
    v = [acc.get(k, 0) / 1000 for k in KEYS]
    print(f"| {Path(f).stem} | {res.get('status')}・{res.get('time', 0):.1f}・{res.get('nodes')} | " + " | ".join(f"{x:.2f}" for x in v) + f" | {sum(v):.2f} | " + " | ".join(f"{acc.get(c, 0):.0f}" for c in CNT) + " |")
