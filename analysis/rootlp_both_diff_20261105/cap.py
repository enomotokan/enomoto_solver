"""待ちの上限 (勝った方の時間 t の r 倍の時点まで、ただし少なくとも a 秒の時点まで、勝った後に待つのは最大 B 秒) の規則ごとに、両方の頂点が揃う問題の数と、増える秒を数える。
使い方: python cap.py ファイル.jsonl ..."""
import json, sys
R = [json.loads(l) for f in sys.argv[1:] for l in open(f)]
R = [r for r in R if "error" not in r and r["winner"] != "none"]
def wl(r):
    ok_s, ok_i = r["st_s"] == "Optimal", r["st_i"] == "Some(Optimal)"
    w = r["t_simplex"] if r["winner"] == "simplex" else r["t_xo"]
    l = (r["t_i"] if r["winner"] == "simplex" else r["t_simplex"])
    return w, l, ok_s and ok_i
print(f"{len(R)} 問 (片方だけ最適も含む。負けた方が失敗する問題は、失敗するか上限に当たるまで待って何も得ない)")
print("| 規則 (待ちの上限) | 両方そろう問題 | 両方が最適の問題のうち | 増える秒の合計 | 増える秒の最大 | 勝った方の時間の合計に対する増え |")
print("|---|---:|---:|---:|---:|---:|")
tw = sum(wl(r)[0] for r in R)
nboth = sum(wl(r)[2] for r in R)
for r_, a, B in [(1.5, 0, 1e9), (2, 0, 1e9), (3, 0, 1e9), (2, 0.2, 1e9), (2, 0.5, 1e9), (3, 0.5, 1e9), (5, 1.0, 1e9), (2, 0.5, 5), (3, 0.5, 5), (3, 0.5, 10), (5, 0.5, 10), (10, 0.5, 5), (1e9, 0.5, 5)]:
    got = add = mx = 0
    for r in R:
        w, l, both = wl(r)
        cap = max(a, r_ * w) - w  # 勝った後に待てる秒 (上限は勝った方の時間の r 倍、ただし少なくとも a 秒の時点まで)
        cap = min(max(cap, 0), B)
        if both and l - w <= cap:
            got += 1
            e = l - w
        else:
            e = min(cap, max(l - w, 0))
        add += e
        mx = max(mx, e)
    print(f"| {r_:g} 倍 (最低 {a} 秒、待ちは最大 {B:g} 秒) | {got} / {len(R)} | {got} / {nboth} | {add:.1f} | {mx:.1f} | +{100*add/tw:.0f}% |")
