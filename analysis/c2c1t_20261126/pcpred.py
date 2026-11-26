"""擬似費用の予測と、木の中で実際に分枝したときの伸び (単位あたり) の比べ (README 4 節、親の依頼: 利用者の問い)。
python pcpred.py pclog/n85_{def,sbpb,sbpbk}_s{1,2,3}.log.gz
PCO の行 (ENOMOTO_MIP_PCLOG、src/mip/solver.rs の pclog_emit): 子の LP を解いたときの、観測を足す前の擬似費用 (単位あたり、予測) と、
子の LP の値 obj0 − 親の値 を実際の移動量 dact で割った実際の伸び (単位あたり)。観測のない列 (nobs = 0) は除く。
PSB の行: 強分枝の観測 (伝播の境界つきか、擬似費用に入れたか)。行の順に読み、その分枝の時点で列 (と向き) の擬似費用に伝播の境界つきの観測が入っていたかで分ける。
比は log10((予測 + e) / (実際 + e))、e = 1e-6 × max(1, |親の値|)。正なら予測が過大。"""
import collections, gzip, math, statistics, sys
def bins(d): return "0-5" if d <= 5 else "6-10" if d <= 10 else "11-20" if d <= 20 else "21+"
for f in sys.argv[1:]:
    prop = collections.Counter()   # (j, up) -> 擬似費用に入った伝播の境界つきの観測の数
    rows = []
    for l in gzip.open(f, "rt"):
        t = l.split()
        if t[0] == "PSB":
            _, nodes, depth, j, up, unit, pl, counted = t
            if pl == "1" and counted == "1":
                prop[(int(j), up == "1")] += 1
        elif t[0] == "PCO" and t[12] == "1":
            depth, j, up = int(t[4]), int(t[5]), t[6] == "1"
            po, unit, nobs, dact, obj0 = float(t[8]), float(t[10]), int(t[11]), float(t[13]), float(t[14])
            if nobs == 0 or not (dact > 1e-9) or not math.isfinite(obj0):
                continue
            act = max(obj0 - po, 0.0) / dact
            e = 1e-6 * max(1.0, abs(po))
            rows.append((depth, prop[(j, up)] > 0, math.log10((unit + e) / (act + e)), act <= e))
    def summ(rs):
        if not rs: return "—"
        lr = [r[2] for r in rs]
        return f"n {len(rs)}, 中央 {statistics.median(lr):+.2f}, 平均 {statistics.mean(lr):+.2f}, 予測 > 10 倍 {sum(x > 1 for x in lr) / len(lr):.0%}, 実際 0 {sum(r[3] for r in rs) / len(rs):.0%}"
    print(f"== {f.split('/')[-1]}: 全体 {summ(rows)}")
    for has in (True, False):
        print(f"   伝播の観測あり={has}: {summ([r for r in rows if r[1] == has])}")
    for b in ("0-5", "6-10", "11-20", "21+"):
        print(f"   深さ {b}: {summ([r for r in rows if bins(r[0]) == b])}")
