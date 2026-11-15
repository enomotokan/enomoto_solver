"""HILOSS: summ.py の量を 1 問 1 行の表にまとめる (table.txt)。使い方: python table.py > table.txt
最適値は、どちらかが最適と報告した値 (なければ良い方の暫定解)。根で閉じたギャップ = (木に入る直前の下界 - 根の LP) / (最適値 - 根の LP)。
残りの根のギャップの比 = ENOMOTO の (最適値 - 木に入る直前の下界) / HiGHS の同じ量。"""
import json, math
from pathlib import Path
import summ
D = Path(__file__).resolve().parent
order = [l.split()[1] for l in (D / "losses.txt").read_text().splitlines() if len(l.split()) > 5 and l.split()[0] in ("A", "B", "C")]
grp = {l.split()[1]: l.split()[0] for l in (D / "losses.txt").read_text().splitlines() if len(l.split()) > 5 and l.split()[0] in ("A", "B", "C")}
def ok(v): return v is not None and isinstance(v, (int, float)) and math.isfinite(v)
def r(a, b, nd=1):
    return f"{a / b:.{nd}f}" if ok(a) and ok(b) and b > 0 else "-"
def t(v): return f"{v:.0f}" if ok(v) else "-"
print("組 問題                     | 終わり H/E (秒)  | 前処理後 列 H/E (行の比 E/H) | 根LP秒 H/E | 木に入る秒 H/E | 根で閉じた% H/E (残りの比) | 再スタート H(根+木)/E(根+木) | 最初の解 秒 H/E | 最良の解 秒 H/E | ノード H/E | ノード/秒 H/E | ノードの LP 反復 H(/ノード)/E(/LP) | 終わりのギャップ% E")
for p in order:
    h, o = summ.highs(p), summ.ours(p)
    if not h or not o: print(f"{grp[p]}  {p:24} | (ログなし)"); continue
    hs = str(h.get("status", "")).startswith(("Optimal", "Infeasible")); os_ = o.get("json_status") in ("optimal", "infeasible")
    opt = h["end_sol"] if hs else o["end_sol"] if os_ else min(x for x in (h.get("end_sol"), o.get("end_sol")) if ok(x)) if any(ok(x) for x in (h.get("end_sol"), o.get("end_sol"))) else None
    def closed(d):
        if not ok(opt) or not ok(d.get("root_lp")) or not ok(d.get("root_bound")) or abs(opt - d["root_lp"]) < 1e-12: return None
        return 100 * (d["root_bound"] - d["root_lp"]) / (opt - d["root_lp"])
    ch, co = closed(h), closed(o)
    rem = r(opt - o["root_bound"], opt - h["root_bound"], 2) if ok(opt) and ok(o.get("root_bound")) and ok(h.get("root_bound")) else "-"
    def rate(d):
        if not d.get("nodes") or "root_end_t" not in d: return None
        dt = d["end_t"] - d["root_end_t"]; return d["nodes"] / dt if dt > 1 else None
    gap = "-"
    if ok(o.get("end_sol")) and ok(o.get("end_bound")): gap = f"{100 * abs(o['end_sol'] - o['end_bound']) / max(1e-9, abs(o['end_sol'])):.2f}"
    f1 = lambda v: f"{v:.1f}" if ok(v) else "-"
    print(f"{grp[p]}  {p:24} | {t(h.get('end_t')) + ('' if hs else '*'):>5}/{t(o.get('end_t')) + ('' if os_ else '*'):<5} | {h['pre'][1]:>6}/{o['pre'][1]:<6} ({r(o['pre'][0], h['pre'][0], 2)}) | {f1(h.get('root_lp_t')):>6}/{f1(o.get('root_lp_t')):<6} | {t(h.get('root_end_t')):>4}/{t(o.get('root_end_t')):<4} | {f1(ch):>6}/{f1(co):<6} ({rem}) | {h['restarts'] - h.get('tree_restarts', 0)}+{h.get('tree_restarts', 0)}/{o['restarts']}+{o['tree_restarts']} | {f1(h.get('first_sol_t')):>6}/{f1(o.get('first_sol_t')):<6} | {t(h.get('last_sol_t')):>4}/{t(o.get('last_sol_t')):<4} | {t(h.get('nodes')):>7}/{t(o.get('nodes')):<7} | {f1(rate(h)):>6}/{f1(rate(o)):<6} | {f1(h.get('node_iters_per_node')):>6}/{f1(o.get('diag_iters_per_lp')):<6} | {gap}")
print("\n* = 時間切れ。H = HiGHS、E = ENOMOTO (既定、種 1)。")
