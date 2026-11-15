"""HILOSS: ENOMOTO のログ (logs/<問題>.ours.log.gz) から、根の段の時間の使い道を短い時系列にする。
使い方: python phases.py 問題 [...]   (書かなければ logs/ の全部 -> phases.txt に向ける)
出す行: 前処理、LP プロービング、対称性、根の LP、カットのループ (ラウンド数・時間・値)、1 秒以上かかった根のヒューリスティクス、裏のカット、
再スタート、木に入る時刻、木の中の再スタート、終わりの統計 (ノードの LP・強分岐・カット)。"""
import gzip, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
def run(p):
    f = D / "logs" / f"{p}.ours.log.gz"
    if not f.exists(): return
    L = []
    for l in gzip.open(f, "rt", errors="replace"):
        try: L.append((float(l[:8]), l[9:].rstrip("\n")))
        except ValueError: pass
    if not L: return
    t0 = L[0][0]
    print(f"\n=== {p} (ENOMOTO、時刻は最初の行からの秒) ===")
    cut = None  # (始まりの時刻, ラウンド数, 最初の値, 最後の値, LP の反復)
    def flush_cut(t, s=""):
        nonlocal cut
        if cut: print(f"  {cut[0]:7.1f}-{t:7.1f} カットのループ: {cut[1]} ラウンド、{cut[2]} -> {cut[3]}、LP {cut[4]} 反復 {s}"); cut = None
    for t, s in L:
        t -= t0
        m = re.match(r"MIP: presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+), nonzeros (\d+) -> (\d+)", s)
        if m: print(f"  {t:7.1f} 前処理: 行 {m.group(1)} -> {m.group(2)}、列 {m.group(3)} -> {m.group(4)}、非零 {m.group(5)} -> {m.group(6)}"); continue
        m = re.match(r"MIP: LP probing: (.*)", s)
        if m and ("root LP" in s or "stopped" in s): print(f"  {t:7.1f} LP プロービング: {m.group(1)[:150]}"); continue
        m = re.match(r"MIP: symmetry: (\d+ generators.*|none.*)", s)
        if m: print(f"  {t:7.1f} 対称性: {m.group(1)[:110]}"); continue
        m = re.match(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", s)
        if m: print(f"  {t:7.1f} 根の LP {m.group(1)} ({m.group(2)} 反復、{m.group(3)} 秒)"); continue
        m = re.match(r"MIP: cut round (\d+): (\d+) cuts \(of (\d+), sep ([\d.]+)s, LP (\d+) iters\), LP rows (\d+) \(cut rows mean nnz (\d+)\), obj (\S+)", s)
        if m:
            if cut is None: cut = [t, 0, m.group(8), m.group(8), 0]
            cut[1] += 1; cut[3] = m.group(8); cut[4] += int(m.group(5)); continue
        if "root cut time cap check" in s and "-> stop" in s: print(f"  {t:7.1f} カットの時間の上限で停止: {s[s.find('clock'):]}"); continue
        m = re.match(r"MIP: cut loop done: (.*)", s)
        if m: flush_cut(t, "| " + m.group(1)); continue
        m = re.match(r"MIP: root heuristic (.*): (\S+) \(([\d.]+)s\)", s)
        if m and float(m.group(3)) >= 1.0: print(f"  {t:7.1f} 根のヒューリスティクス {m.group(1)}: {m.group(2)} ({m.group(3)} 秒)"); continue
        m = re.match(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*)\]", s)
        if m: print(f"  {t:7.1f} 暫定解 {m.group(1)} [{m.group(4)}] (ノード {m.group(2)})"); continue
        m = re.match(r"MIP: after root heuristics: (.*)", s)
        if m: print(f"  {t:7.1f} 根のヒューリスティクスの終わり: {m.group(1)}"); continue
        m = re.match(r"MIP: background cuts: (\d+ rounds, .*)", s)
        if m: print(f"  {t:7.1f} 裏のカット: {m.group(1)[:140]}"); continue
        m = re.match(r"MIP: restart check: (.*)", s)
        if m: print(f"  {t:7.1f} 再スタートの判定: {m.group(1)}"); continue
        m = re.match(r"MIP: (early restart: .*|restart \d+ \(.*|tree restart: .*)", s)
        if m: print(f"  {t:7.1f} {m.group(1)}"); continue
        m = re.match(r"TRSTAT: (.*)", s)
        if m: print(f"  {t:7.1f} TRSTAT {m.group(1)[:200]}"); continue
        m = re.match(r"MIP: (node diag: .*|branching: .*|basis nb diag: .*|dual proofs and conflicts .*|solve returned .*|adaptive node cuts: .*|SUBCAP.*|WARNING.*|root objective probing: .*)", s)
        if m: print(f"  {t:7.1f} {m.group(1)[:330]}"); continue
    # 木の経過の行を 6 本ほど
    pr = [(t - t0, s) for t, s in L if re.match(r"MIP:\s+[\d.]+s nodes", s)]
    step = max(1, len(pr) // 6)
    for t, s in pr[::step][:7] + pr[-1:]:
        print(f"  {t:7.1f} {re.sub(r' +', ' ', s)[:250]}")
if __name__ == "__main__":
    for p in sys.argv[1:] or sorted({x.name.split(".")[0] for x in (D / "logs").glob("*.ours.log.gz")}): run(p)
