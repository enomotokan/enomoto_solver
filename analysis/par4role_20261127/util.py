"""prof.py の出力 (logs/<名前>.cpu.jsonl と .log) から、区間ごとの CPU の使い方を出す: python util.py <名前> [<名前> ...]
- 時刻は求解の時計 (ログの秒) に合わせる: 「MIP: solve returned ... after T s」の T を、CPU が最後に動いた時刻に合わせる。
- 区間: 前処理〜根の LP (最初の「root LP」まで)・根 (〜「tree search with」)・木 (〜終わり)。
- 区間ごとに、CPU 秒の合計 / (壁の秒 × スレッド数) = 使用率と、動いていたスレッドの数 (0.25 秒の中で 10% 以上動いた) の平均を出す。
- 1 秒ごとの「動いていたスレッド数」の列も出す (並列がどこで遊んでいるかを見る)。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
def one(name):
    rows = [json.loads(l) for l in open(D / f"{name}.cpu.jsonl")]
    log = (D / f"{name}.log").read_text(errors="replace")
    th = int(re.search(r"_t(\d+)", name).group(1))
    m = re.search(r"solve returned \w+ after ([\d.]+)s", log); T = float(m.group(1)) if m else None
    # 各標本の間の、スレッドごとの CPU の増え
    ivs = []
    for a, b in zip(rows, rows[1:]):
        dt = b["t"] - a["t"]
        d = {k: b["cpu"][k] - a["cpu"].get(k, 0.0) for k in b["cpu"]}
        ivs.append((a["t"], b["t"], dt, d))
    last = max((iv[1] for iv in ivs if sum(iv[3].values()) > 0.05 * iv[2]), default=ivs[-1][1])
    off = last - T if T else 0.0
    rl = re.search(r"MIP: root LP -?[\d.e+-]+ \(\d+ iters, ([\d.]+)s\)", log)
    ts = re.search(r"tree search with \d+ threads \(([\d.]+)s\)", log)
    cut = [("前処理〜根の LP", 0.0), ("根", float(rl.group(1)) if rl else None), ("木", float(ts.group(1)) if ts else None), ("終わり", T)]
    cut = [(n, t) for n, t in cut if t is not None]
    print(f"== {name}: 求解 {T} 秒 (時計のずれ {off:.2f} 秒)")
    for (n, a), (_, b) in zip(cut, cut[1:]):
        cs = sum(sum(d.values()) for s, e, dt, d in ivs if a <= s - off < b)
        w = sum(dt for s, e, dt, d in ivs if a <= s - off < b)
        act = sum(sum(1 for v in d.values() if v > 0.1 * dt) * dt for s, e, dt, d in ivs if a <= s - off < b)
        if w > 0: print(f"  {n:10} {a:7.2f}〜{b:7.2f} 秒: CPU {cs:7.1f} 秒、使用率 {cs / (w * th):5.0%}、動いていたスレッド 平均 {act / w:4.1f}")
    # 1 秒ごとの動いていたスレッド数 (CPU 秒 / 秒) の列
    secs = {}
    for s, e, dt, d in ivs:
        k = int(s - off)
        if 0 <= k: secs.setdefault(k, [0.0, 0.0]); secs[k][0] += sum(d.values()); secs[k][1] += dt
    line = "".join(str(min(9, round(c / w))) if w else "." for k, (c, w) in sorted(secs.items()))
    print("  1 秒ごとの CPU 本数: " + " ".join(line[i:i + 10] for i in range(0, len(line), 10)))
for n in sys.argv[1:]: one(n)
