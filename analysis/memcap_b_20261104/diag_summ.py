"""runs.jsonl の MEMB_DIAG 行 (各スレッドの最新) を足して、開いたノードの持ち物の種類ごとの合計と RSS を時刻ごとに並べる (2026-11-04)。
使い方: python diag_summ.py <回の名前> [刻み 秒]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
name = sys.argv[1]; step = float(sys.argv[2]) if len(sys.argv) > 2 else 60
rec = [json.loads(l) for l in open(D / "runs.jsonl") if json.loads(l)["name"] == name][-1]
R = re.compile(r"MEMB_DIAG: tid (\d+) t (\d+) nodes (\d+) rows (\d+) \| queue n (\d+) slots (\d+) changes ([\d.]+) basis ([\d.]+) dse ([\d.]+) prot ([\d.]+) sb ([\d.]+) tags ([\d.]+) MB \| pool n (\d+) slots (\d+) changes ([\d.]+) basis ([\d.]+) dse ([\d.]+) cutbasis ([\d.]+) MB \| row_log add (\d+) del (\d+) ([\d.]+) MB")
# 主スレッドの t は起動からの秒に近い (前処理の後から)。各行を出た順に読み、各スレッドの最新を保つ
last = {}; rows = []
for l in rec["diag"]:
    m = R.match(l)
    if not m: continue
    g = m.groups(); tid = int(g[0]); t = int(g[1])
    last[tid] = g
    if tid == 0:
        q = [sum(float(last[k][i]) for k in last) for i in (6, 7, 8, 9, 10, 11)]
        qn = sum(int(last[k][4]) for k in last); nodes = sum(int(last[k][2]) for k in last)
        pn, pc, pb, pd, pcb = int(g[12]), float(g[14]), float(g[15]), float(g[16]), float(g[17])
        rl = sum(float(last[k][20]) for k in last)
        rows.append((t, nodes, qn, pn, q, pc, pb, pd, rl))
ser = rec["series"]
def rss_at(t0):
    # 主スレッドの t は求解の開始から。起動からの時間との差は小さい (数秒) とみなさず、series の最後の時刻と主スレッドの最後の t の差で合わせる
    return None
off = (ser[-1][0] - rows[-1][0]) if rows and ser else 0
print(f"# {name}: 結果 {rec['result'].get('status')} 目的値 {rec['result'].get('objective')} ノード {rec['result'].get('nodes')} 最大 RSS {rec['peak_rss_mb_sampled']} MB (時刻のずれの目安 {off:.0f} s)")
print("| 求解 [s] | 済んだ | 開いた (各スレッド) | 共有 | 境界の変更 | 基底 | DSE | 共有 基底 | 共有 DSE | 共有 変更 | row_log | 合計 | RSS | mi | glibc |")
print("|" + "---:|" * 15)
nxt = 0
for r in rows:
    t = r[0]
    if t < nxt: continue
    nxt = t + step
    s = min(ser, key=lambda x: abs(x[0] - (t + off)))
    tot = r[4][0] + r[4][1] + r[4][2] + r[4][3] + r[5] + r[6] + r[7] + r[8]
    cat = s[2]
    print(f"| {t} | {r[1]} | {r[2]} | {r[3]} | {r[4][0]:.0f} | {r[4][1]:.0f} | {r[4][2]:.0f} | {r[6]:.0f} | {r[7]:.0f} | {r[5]:.0f} | {r[8]:.0f} | {tot:.0f} | {s[1]} | {cat['mi_1g']} | {cat['heap'] + cat['arena64'] + cat['anon_other']} |")
