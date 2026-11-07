"""6 章の診断を入れる (使い方: python diag6.py ソースの根)。modkvb.rs に解ごとの印 (k = 2・変数上下限・LP のカットの行・CG) と
CG の式そのものの形 (ENOMOTO_MIP_MODKVB_CG)、separation.rs に zerohalf のカットとの平行の数え上げ (ENOMOTO_MIP_MODKVB_ZHCMP) と
zerohalf と平行な候補を出さない形 (ENOMOTO_MIP_MODKVB_SKIPZH) を足す。"""
import sys
root = sys.argv[1]
p = root + "/src/mip/modkvb.rs"
s = open(p).read()
def rep(a, b, cnt=1):
    global s
    assert a in s, a[:80]
    s = s.replace(a, b, cnt)
# 連立の行に変数上下限の印
if "    vbi: u32,\n}" in s:
    rep("""    vbi: u32,
}""", """    vbi: u32,
    /// 変数上下限で置き換えた項があるか (6 章の診断)
    vbf: bool,
}""")
else:
    rep("""    ents: Vec<(usize, i64)>,
    rhs: i64,
}""", """    ents: Vec<(usize, i64)>,
    rhs: i64,
    /// 変数上下限で置き換えた項があるか (6 章の診断)
    vbf: bool,
}""")
import re
m = re.search(r"sys\.push\(SysRow \{ row: i, sg, scale, ents, rhs: irhs([^}]*)\}\);", s)
assert m
s = s.replace(m.group(0), "sys.push(SysRow { row: i, sg, scale, ents, rhs: irhs" + m.group(1) + ", vbf: c1 + c2 > 0 });")
s = re.sub(r"(rhs: (3|-4)(, vbc: 0, vbi: 0)?) \}", r"\1, vbf: false }", s)
# 印の置き場
rep("""/// [`modkvb_cuts`] の記録""", """thread_local! {
    /// 6 章の診断: 直前の [`modkvb_cuts`] の返した組ごとの印 (1 = k が 2、2 = 変数上下限の置き換えを含む、4 = LP のカットの行を含む、8 = CG の式)
    pub(crate) static MODKVB_TAGS: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// [`modkvb_cuts`] の記録""")
# CG モード
rep("""            st.sol_rows += sol.len();
            let mut cs: Vec<RawCut> = Vec::new();""", """            st.sol_rows += sol.len();
            let mut cs: Vec<RawCut> = Vec::new();
            let has_vb = sol.iter().any(|&(q, _)| sys[q].vbf);
            let has_cut = sol.iter().any(|&(q, _)| sys[q].row >= m_orig);
            let mut tag: u8 = (k == 2) as u8 | ((has_vb as u8) << 1) | ((has_cut as u8) << 2);
            // 6 章の診断 `ENOMOTO_MIP_MODKVB_CG` (=1 は CG の式だけ、=both は CG の式と従来の両方): 重み ((k-1) w mod k) / k の非負の乗数で
            // 向きをそろえた行を足し、単純な上下限で x' >= 0 に移して係数と右辺を切り下げる (Chvátal-Gomory)。変数上下限の置き換えを含む解と、
            // 係数が負の連続列 (x' >= 0 で落とせない) が残るものは作らない
            if cg_mode > 0 && !has_vb {
                if let Some(c) = cg_cut(vars, lp_rows, n, &sys, sol, k, &sub) {
                    cs.push(c);
                    tag |= 8;
                }
            }
            if cg_mode == 1 {
                if !cs.is_empty() {
                    out.push(cs);
                    tags.push(tag);
                }
                continue;
            }""")
rep("""            if !cs.is_empty() {
                out.push(cs);
            }
        }""", """            if !cs.is_empty() {
                out.push(cs);
                tags.push(tag);
            }
        }""")
rep("""    let mut out: Vec<Vec<RawCut>> = Vec::new();
    let mut aggv""", """    let mut out: Vec<Vec<RawCut>> = Vec::new();
    let mut tags: Vec<u8> = Vec::new();
    let cg_mode: u8 = match env_str!("ENOMOTO_MIP_MODKVB_CG") { None | Some("0") => 0, Some("both") => 2, Some(_) => 1 };
    let mut aggv""")
# 返す直前に印を置く (関数の最後の (out, st))
idx = s.index("pub(crate) fn modkvb_cuts(")
tail = s[idx:]
j = tail.index("\n    (out, st)\n}")
tail = tail[:j] + "\n    MODKVB_TAGS.with(|t| *t.borrow_mut() = tags);" + tail[j:]
s = s[:idx] + tail
# 早い return でも印を空に
s = s.replace("""    if sys.is_empty() || sys.iter().all(|r| r.rhs == 0) {
        return (Vec::new(), st);
    }""", """    if sys.is_empty() || sys.iter().all(|r| r.rhs == 0) {
        MODKVB_TAGS.with(|t| t.borrow_mut().clear());
        return (Vec::new(), st);
    }""")
# cg_cut 関数
rep("""#[cfg(test)]
mod tests {""", """/// 6 章の診断: 解 `sol` の mod-k の CG カット (非負の乗数 ((k-1) w mod k) / k で向きをそろえた LP の行を足し、単純な上下限で
/// `x' >= 0` に移して係数と右辺を切り下げる)。構造変数だけの式で返す。置き換えが使えない列があれば `None`
fn cg_cut(vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], n: usize, sys: &[SysRow], sol: &[(usize, u32)], k: u32, sub: &[Sub]) -> Option<RawCut> {
    let mut acc: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
    let mut beta = 0.0f64;
    for &(q, w) in sol {
        let r = &sys[q];
        let u = (((k - 1) * w) % k) as f64 / k as f64 * r.scale;
        if u == 0.0 {
            continue;
        }
        let b = if r.sg > 0.0 { vars.up[n + r.row] } else { -vars.lo[n + r.row] };
        if !b.is_finite() {
            return None;
        }
        beta += u * b;
        for &(j, a) in &lp_rows[r.row] {
            *acc.entry(j).or_insert(0.0) += u * r.sg * a;
        }
    }
    let fl = |v: f64| (v + 1e-9 * v.abs().max(1.0)).floor();
    let mut coefs: Vec<(usize, f64)> = Vec::new();
    let mut rhs_const = 0.0f64;
    let mut terms: Vec<(usize, bool, f64)> = Vec::new();
    for (&j, &c) in &acc {
        if c.abs() <= 1e-12 {
            continue;
        }
        match sub[j] {
            Sub::Fixed(v) => beta -= c * v,
            Sub::ContAt(v) => {
                let lower = v == vars.lo[j];
                // x = l + x' (係数 c) か x = u - x' (係数 -c)。係数が負なら x' >= 0 で落とせない
                let cc = if lower { c } else { -c };
                if cc < -1e-9 {
                    return None;
                }
                beta -= c * v;
            }
            Sub::Int(lower, _) => {
                let (cc, b) = if lower { (c, vars.lo[j]) } else { (-c, vars.up[j]) };
                if !b.is_finite() {
                    return None;
                }
                beta -= c * b;
                terms.push((j, lower, cc));
            }
            Sub::Vb(..) | Sub::Off => return None,
        }
    }
    let fb = fl(beta);
    for (j, lower, cc) in terms {
        let f = fl(cc);
        if f == 0.0 {
            continue;
        }
        // f x' : 下限なら f (x - l)、上限なら f (u - x)
        if lower {
            coefs.push((j, f));
            rhs_const += f * vars.lo[j];
        } else {
            coefs.push((j, -f));
            rhs_const -= f * vars.up[j];
        }
    }
    if coefs.is_empty() {
        return None;
    }
    Some(RawCut { coefs, rhs: fb + rhs_const })
}

#[cfg(test)]
mod tests {""")
open(p, "w").write(s)

# separation.rs: 平行の数え上げと SKIPZH
p = root + "/src/mip/separation.rs"
s = open(p).read()
old = """                let (groups, st) = super::modkvb::modkvb_cuts("""
assert old in s
a = s.index(old)
b = s.index("keep_best(&mut cands, n0, self.p);\n                }", a)
b = s.index("}", b + len("keep_best(&mut cands, n0, self.p);\n                ")) + 1
seg = s[a:b]
m = re.search(r"let \(groups, st\) = super::modkvb::modkvb_cuts\((.*?)\);", seg)
call = m.group(0)
new = call + """
                let mk_tags = super::modkvb::MODKVB_TAGS.with(|t| std::mem::take(&mut *t.borrow_mut()));
                // 6 章の診断 `ENOMOTO_MIP_MODKVB_ZHCMP` (数えて出す) / `ENOMOTO_MIP_MODKVB_SKIPZH` (平行なものを除く): 同じ LP 点の zerohalf の候補と
                // 係数の向きが平行な試作の候補 (解ごとに残した 1 本) を数える。向きは係数を最大の絶対値で割り 1e-6 に丸めたもの
                let zhcmp = env_str!("ENOMOTO_MIP_MODKVB_ZHCMP").is_some();
                let skipzh = env_str!("ENOMOTO_MIP_MODKVB_SKIPZH").is_some();
                let dirkey = |c: &Candidate| -> (Vec<(usize, i64)>, i64) {
                    let mx = c.coefs.iter().map(|t| t.1.abs()).fold(0.0f64, f64::max).max(1e-300);
                    let mut v: Vec<(usize, i64)> = c.coefs.iter().filter(|t| t.1.abs() > 1e-12 * mx).map(|&(j, a)| (j, (a / mx * 1e6).round() as i64)).collect();
                    v.sort_unstable();
                    (v, (c.rhs / mx * 1e6).round() as i64)
                };
                let zh_keys: std::collections::HashMap<Vec<(usize, i64)>, i64> = if zhcmp || skipzh {
                    cands.iter().filter(|c| c.src == sepprof::SRC_ZH + src_off).map(|c| dirkey(c)).collect()
                } else {
                    std::collections::HashMap::new()
                };
                // [k2 で置き換えもカットの行もない, ほか] x [残した数, 平行, 右辺も同じ]
                let mut zc = [[0usize; 3]; 2];
                let mut ncg = [0usize; 2];"""
loop_old = seg[seg.index("for g in groups {"):]
loop_new = """for (gi, g) in groups.into_iter().enumerate() {
                    let n0 = cands.len();
                    for r in g {
                        push(Some(r), &mut cands, self);
                    }
                    keep_best(&mut cands, n0, self.p);
                    if (zhcmp || skipzh) && cands.len() > n0 {
                        let tg = mk_tags.get(gi).copied().unwrap_or(0);
                        let cls = if tg & 7 == 1 { 0 } else { 1 };
                        zc[cls][0] += 1;
                        if tg & 8 != 0 {
                            ncg[cls] += 1;
                        }
                        let (kv, kr) = dirkey(&cands[n0]);
                        if let Some(&zr) = zh_keys.get(&kv) {
                            zc[cls][1] += 1;
                            if zr == kr {
                                zc[cls][2] += 1;
                            }
                            if skipzh {
                                cands.truncate(n0);
                            }
                        }
                    }
                }
                if zhcmp {
                    eprintln!("MODKVB_ZHCMP zh {} k2plain kept {} par {} same {} cg {} | other kept {} par {} same {} cg {}", zh_keys.len(), zc[0][0], zc[0][1], zc[0][2], ncg[0], zc[1][0], zc[1][1], zc[1][2], ncg[1]);
                }"""
seg2 = seg[:seg.index(call)] + new + seg[seg.index(call) + len(call):seg.index("for g in groups {")] + loop_new
s = s[:a] + seg2 + s[b:]
open(p, "w").write(s)
print("ok")
