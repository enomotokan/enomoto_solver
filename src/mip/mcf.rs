//! 多品種フロー (multi-commodity flow, MCF) の構造の検出と cut-set の集約 (SCIP の `sepa_mcf.c`、
//! Achterberg & Raack "The MCF-separator: detecting and exploiting multi-commodity flow structures in MIPs" (2010) の移植)。
//!
//! 次の形の部分を元の行から探す:
//!
//! - (1) 流量保存: `sum_{a in out(v)} f_a^k - sum_{a in in(v)} f_a^k <= -d_v^k` (節点 v、品種 k。係数は全部 ±1 の定数倍)
//! - (2) 弧の容量: `sum_k f_a^k - c_a x_a <= 0` (x は整数。一般の線形式でよい)
//!
//! 検出は SCIP と同じ順: 流量保存の候補の行を点数順に並べ、列を共有する行を順につないで品種 (commodity) を作る →
//! 容量の候補の行を点数順に弧に割り当てる → 品種どうしで同じ弧を共有する行を同じ節点にまとめる → 弧の始点・終点を
//! 多数決で決める → 容量の行のない流量の列から容量なしの弧を作る → 連結成分ごとに網にする。SCIP は双対値を点数の
//! 同点の決め手と等式の向きの決め手に使うが、ここでは使わない (検出は根で一度だけ、元の行だけから行う)。
//!
//! 分離 ([`aggregations`]) は、節点の集合 S について、S に需要のある品種の S の流量保存の行と、T = V \ S から S に入る弧の
//! 容量の行を足した行 (`flow(S -> T) - c x(T -> S) <= -d_S` の形) を作る。S は 1 節点ずつと、弧の余裕と双対値で節点を
//! `ENOMOTO_T_MIP_MCF_CLUSTERS` (5) 個の塊に縮めたときの塊の部分集合 (SCIP と同じ)。集約した行は呼び出し側で CMIR
//! ([`super::cuts::generate_cuts`]) に通す。行の活動量の列 (スラック) を含めた形で集約するので、検出が外れていても
//! できる行はいつでも妥当で、外れは強さにしか効かない。

use std::collections::HashMap;

use super::problem::MipProblem;

const LHSPOSSIBLE: u8 = 1;
const RHSPOSSIBLE: u8 = 2;
const LHSASSIGNED: u8 = 4;
const RHSASSIGNED: u8 = 8;
const INVERTED: u8 = 16;
const DISCARDED: u8 = 32;
const UNDIRECTED: u8 = 64;

/// 流量保存の候補にする行の長さの上限 (列の数に対する割合)
const MAXFLOWCANDDENSITY: f64 = 0.1;
/// 品種に残す節点の数の下限と、最大の品種に対する割合の下限
const MINNODES: usize = 3;
const MINCOMNODESFRACTION: f64 = 0.5;
const MINARCS: usize = 3;
/// 流量の列 / 流量の行の上限 (平均の次数)
const MAXFLOWVARFLOWROWRATIO: f64 = 100.0;
const MAXARCNODERATIO: f64 = 100.0;
/// 弧の始点・終点の食い違いの上限 (全体 (弧の数に対する割合) と弧ごと)。全体の上限は `ENOMOTO_T_MIP_MCF_MAX_INCONS` (0.02、SCIP と同じ)
fn max_inconsistency_ratio() -> f64 {
    tunable!("ENOMOTO_T_MIP_MCF_MAX_INCONS", 0.02, f64)
}
const MAXARCINCONSISTENCYRATIO: f64 = 0.5;
/// 容量なしの弧を作るのに要る品種の割合
const UNCAPACITATEDARCSTRESHOLD: f64 = 0.8;
/// 網の数の上限
const MAXNETWORKS: usize = 4;
/// 集約に使う容量の行の余裕の上限 (最大係数で割った値)
const MAXCAPACITYSLACK: f64 = 0.1;
/// 集約の重みの最大 / 最小の上限
const MAXWEIGHTRANGE: f64 = 1e6;
const FEASTOL: f64 = 1e-6;

fn feq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

/// 検出した網 (連結成分 1 つ)。行の番号は元の行 (LP の行 0..m) の番号。
pub(super) struct McfNetwork {
    pub nnodes: usize,
    pub ncom: usize,
    /// 向きのある網か (反対向きの弧が容量を共有するなら向きなし)
    pub directed: bool,
    /// 節点 v・品種 k の流量保存の行 `node_rows[v * ncom + k]`: (行, 重み (`out - in <= -d` の形にする倍率、反転を含まない), 反転するか)
    node_rows: Vec<Option<(usize, f64, bool)>>,
    /// 弧の容量の行 (行, 重み (流量の係数を正にして `<=` の形にする倍率))。容量なしの弧は None
    arc_cap: Vec<Option<(usize, f64)>>,
    arc_src: Vec<i32>,
    arc_tgt: Vec<i32>,
    /// 流量の列の品種 (-1 なら流量の列でない)。品種の番号はこの網の中のもの
    col_com: Vec<i32>,
    pub nuncap: usize,
}

impl McfNetwork {
    pub fn narcs(&self) -> usize {
        self.arc_cap.len()
    }
}

/// 検出の途中の状態 (SCIP の `MCFDATA`)
struct Data<'a> {
    p: &'a MipProblem,
    flowrowsigns: Vec<u8>,
    flowrowscalars: Vec<f64>,
    flowrowscores: Vec<f64>,
    flowcands: Vec<usize>,
    capacityrowsigns: Vec<u8>,
    capacityrowscores: Vec<f64>,
    capacitycands: Vec<usize>,
    plusflow: Vec<bool>,
    minusflow: Vec<bool>,
    ncom: usize,
    nempty: usize,
    comsigns: Vec<i8>,
    colcom: Vec<i32>,
    rowcom: Vec<i32>,
    colarc: Vec<i32>,
    rownode: Vec<i32>,
    newcols: Vec<usize>,
    nnodes: usize,
    /// 弧の容量の行 (容量なしの弧は None)
    caprows: Vec<Option<usize>>,
    arc_src: Vec<i32>,
    arc_tgt: Vec<i32>,
    directed: bool,
}

/// 検出の結果の数 (ログ用)
#[derive(Default, Debug, Clone)]
pub(super) struct McfStats {
    pub flowcands: usize,
    pub commodities: usize,
    pub flowrows: usize,
    pub capcands: usize,
    pub arcs: usize,
    pub nodes: usize,
    pub inconsistencies: f64,
    pub directed: bool,
    pub fail: &'static str,
}

/// `p` から MCF の網を探す (見つからなければ空)。
pub(super) fn extract(p: &MipProblem) -> (Vec<McfNetwork>, McfStats) {
    let mut st = McfStats::default();
    let (n, m) = (p.n, p.m);
    if n == 0 || m == 0 {
        st.fail = "empty";
        return (Vec::new(), st);
    }
    let colrow = n as f64 / m as f64;
    if !(0.01..=100.0).contains(&colrow) {
        st.fail = "col/row ratio";
        return (Vec::new(), st);
    }
    let mut d = Data {
        p,
        flowrowsigns: vec![0; m],
        flowrowscalars: vec![0.0; m],
        flowrowscores: vec![0.0; m],
        flowcands: Vec::new(),
        capacityrowsigns: vec![0; m],
        capacityrowscores: vec![0.0; m],
        capacitycands: Vec::new(),
        plusflow: vec![false; n],
        minusflow: vec![false; n],
        ncom: 0,
        nempty: 0,
        comsigns: Vec::new(),
        colcom: vec![-1; n],
        rowcom: vec![-1; m],
        colarc: vec![-1; n],
        rownode: vec![-1; m],
        newcols: Vec::new(),
        nnodes: 0,
        caprows: Vec::new(),
        arc_src: Vec::new(),
        arc_tgt: Vec::new(),
        directed: true,
    };
    d.extract_flow_rows();
    st.flowcands = d.flowcands.len();
    if d.flowcands.is_empty() {
        st.fail = "no flow rows";
        return (Vec::new(), st);
    }
    let (ok, nflowrows) = d.extract_flow();
    st.flowrows = nflowrows;
    st.commodities = d.ncom - d.nempty;
    if !ok {
        st.fail = "flow";
        return (Vec::new(), st);
    }
    d.extract_capacity_rows();
    st.capcands = d.capacitycands.len();
    st.directed = d.directed;
    if d.capacitycands.is_empty() {
        st.fail = "no capacity rows";
        return (Vec::new(), st);
    }
    d.extract_capacities();
    d.extract_nodes();
    for s in d.comsigns.iter_mut() {
        if *s == 0 {
            *s = 1;
        }
    }
    d.cleanup_network();
    st.commodities = d.ncom;
    st.arcs = d.caprows.len();
    st.nodes = d.nnodes;
    let (ok, incons) = d.identify_sources_targets();
    st.inconsistencies = incons;
    if !ok {
        st.fail = "inconsistent";
        return (Vec::new(), st);
    }
    d.find_uncapacitated_arcs();
    let nets = d.components();
    if nets.is_empty() {
        st.fail = "no component";
    }
    (nets, st)
}

impl Data<'_> {
    fn row(&self, r: usize) -> &[(usize, f64)] {
        &self.p.rows[r]
    }

    /// 流量保存の候補の行 (係数の絶対値が全部同じ) と点数 (SCIP の `extractFlowRows`、双対値の同点の決め手は使わない)
    fn extract_flow_rows(&mut self) {
        let p = self.p;
        for r in 0..p.m {
            let row = &p.rows[r];
            let len = row.len();
            if len == 0 || len as f64 > MAXFLOWCANDDENSITY * p.n as f64 {
                continue;
            }
            let coef = row[0].1.abs();
            let (mut hasp, mut hasn) = (false, false);
            let (mut nbin, mut nint, mut ncont) = (0usize, 0usize, 0usize);
            let mut all = true;
            for &(j, a) in row {
                if !feq(a.abs(), coef) {
                    all = false;
                    break;
                }
                hasp |= a > 0.0;
                hasn |= a < 0.0;
                if !p.is_int[j] {
                    ncont += 1;
                } else if p.col_lo[j] >= 0.0 && p.col_up[j] <= 1.0 {
                    nbin += 1;
                } else {
                    nint += 1;
                }
            }
            if !all {
                continue;
            }
            let mut s = 0u8;
            if p.row_lo[r].is_finite() {
                s |= LHSPOSSIBLE;
            }
            if p.row_up[r].is_finite() {
                s |= RHSPOSSIBLE;
            }
            if s == 0 {
                continue;
            }
            self.flowrowsigns[r] = s;
            self.flowrowscalars[r] = 1.0 / coef;
            self.flowcands.push(r);
            let mut sc = 0.0;
            if feq(1.0 / coef, 1.0) {
                sc += 1000.0;
            }
            if hasp && hasn {
                sc += 500.0;
            }
            if ncont == len {
                sc += 1000.0;
            } else if nint == len {
                sc += 500.0;
            } else if nbin == len {
                sc += 100.0;
            }
            sc += 10.0 * len as f64 / (len as f64 + 10.0);
            if s == LHSPOSSIBLE | RHSPOSSIBLE {
                sc += 50.0;
            }
            self.flowrowscores[r] = sc;
        }
        let sc = &self.flowrowscores;
        self.flowcands.sort_by(|&a, &b| sc[b].total_cmp(&sc[a]));
    }

    /// 行 r が品種 k に入るか、入るならその向き (SCIP の `getFlowrowFit`)
    fn flowrow_fit(&mut self, r: usize, k: usize) -> (u8, bool) {
        if self.rowcom[r] != -1 {
            return (0, false);
        }
        let fs = self.flowrowsigns[r];
        if fs & DISCARDED != 0 || fs & (LHSPOSSIBLE | RHSPOSSIBLE) == 0 {
            return (0, false);
        }
        let mut f = fs & (LHSPOSSIBLE | RHSPOSSIBLE);
        let mut inv = f;
        for &(c, a) in &self.p.rows[r] {
            if f == 0 && inv == 0 {
                break;
            }
            if self.colcom[c] == k as i32 {
                // 同じ品種の列は、同じ符号でまだ入っていないときだけ合う
                if self.plusflow[c] {
                    if a > 0.0 {
                        f &= !RHSPOSSIBLE;
                        inv &= !LHSPOSSIBLE;
                    } else {
                        f &= !LHSPOSSIBLE;
                        inv &= !RHSPOSSIBLE;
                    }
                }
                if self.minusflow[c] {
                    if a > 0.0 {
                        f &= !LHSPOSSIBLE;
                        inv &= !RHSPOSSIBLE;
                    } else {
                        f &= !RHSPOSSIBLE;
                        inv &= !LHSPOSSIBLE;
                    }
                }
            } else if self.colcom[c] != -1 {
                f = 0;
                inv = 0;
            }
        }
        if f != 0 {
            (f, false)
        } else if inv != 0 {
            if self.comsigns.get(k).copied().unwrap_or(0) == 0 {
                (inv, true)
            } else {
                (inv | INVERTED, false)
            }
        } else {
            // ほかの品種にも入らない
            self.flowrowsigns[r] |= DISCARDED;
            (0, false)
        }
    }

    /// 行を最後の品種に加える (SCIP の `addFlowrowToCommodity`)
    fn add_flowrow(&mut self, r: usize, rowsign: u8, comcols: &mut Vec<usize>) {
        let k = self.ncom - 1;
        let invert = rowsign & INVERTED != 0;
        let rowsign = rowsign & !INVERTED;
        if self.flowrowsigns[r] & (LHSPOSSIBLE | RHSPOSSIBLE) != LHSPOSSIBLE | RHSPOSSIBLE {
            self.comsigns[k] = 1;
        }
        let mut assigned = if rowsign == LHSPOSSIBLE {
            LHSASSIGNED
        } else if rowsign == RHSPOSSIBLE {
            RHSASSIGNED
        } else if self.p.row_up[r] < 0.0 {
            // 両側にとれるなら a x <= -d (d >= 0) になる側
            RHSASSIGNED
        } else if self.p.row_lo[r] > 0.0 {
            LHSASSIGNED
        } else {
            RHSASSIGNED
        };
        let mut rowscale = if assigned == RHSASSIGNED { 1.0 } else { -1.0 };
        if invert {
            assigned |= INVERTED;
            rowscale = -rowscale;
        }
        self.flowrowsigns[r] |= assigned;
        self.rowcom[r] = k as i32;
        for i in 0..self.p.rows[r].len() {
            let (c, a) = self.p.rows[r][i];
            if self.colcom[c] == -1 {
                self.colcom[c] = k as i32;
                self.newcols.push(c);
                comcols.push(c);
            }
            if rowscale * a > 0.0 {
                self.plusflow[c] = true;
            } else {
                self.minusflow[c] = true;
            }
        }
    }

    fn invert_commodity(&mut self, comrows: &[usize], comcols: &[usize]) {
        for &r in comrows {
            let s = self.flowrowsigns[r];
            self.flowrowsigns[r] &= !(LHSASSIGNED | RHSASSIGNED);
            self.flowrowsigns[r] |= if s & LHSASSIGNED != 0 { RHSASSIGNED } else { LHSASSIGNED };
        }
        for &c in comcols {
            std::mem::swap(&mut self.plusflow[c], &mut self.minusflow[c]);
        }
    }

    /// 品種 k を消す (SCIP の `deleteCommodity`)。消した (行, 列) の数を返す
    fn delete_commodity(&mut self, k: usize, comrows: &[usize]) -> (usize, usize) {
        let mut nvars = 0;
        for &r in comrows {
            self.flowrowsigns[r] &= !(LHSASSIGNED | RHSASSIGNED | INVERTED);
            self.rowcom[r] = -1;
            for &(c, _) in &self.p.rows[r] {
                if self.colcom[c] == k as i32 {
                    nvars += 1;
                }
                self.colcom[c] = -1;
                self.plusflow[c] = false;
                self.minusflow[c] = false;
            }
        }
        if k == self.ncom - 1 {
            self.ncom -= 1;
            self.comsigns.pop();
        } else {
            self.nempty += 1;
        }
        (comrows.len(), nvars)
    }

    /// 最後の品種に入る次の行 (SCIP の `getNextFlowrow`: 新しく入った列を後ろから見て、最初に合う行のある列の最良の行)
    fn next_flowrow(&mut self) -> Option<(usize, u8, bool)> {
        let k = self.ncom - 1;
        while let Some(c) = self.newcols.pop() {
            if self.plusflow[c] && self.minusflow[c] {
                continue;
            }
            let mut best: Option<(usize, u8, bool, f64)> = None;
            for i in 0..self.p.cols[c].len() {
                let r = self.p.cols[c][i].0;
                let (s, inv) = self.flowrow_fit(r, k);
                if s != 0 {
                    let sc = self.flowrowscores[r] * if s & INVERTED != 0 { 0.75 } else { 1.0 };
                    if best.is_none_or(|b| sc > b.3) {
                        best = Some((r, s, inv, sc));
                    }
                }
            }
            if let Some((r, s, inv, _)) = best {
                return Some((r, s, inv));
            }
        }
        None
    }

    /// 流量保存の行を品種にまとめる (SCIP の `extractFlow`)。(失敗しなかったか, 流量の行の数)
    fn extract_flow(&mut self) -> (bool, usize) {
        let mut ncomnodes: Vec<usize> = Vec::new();
        let mut deleted: Vec<bool> = Vec::new();
        let (mut maxnnodes, mut nflowrows, mut nflowvars) = (0usize, 0usize, 0usize);
        let mut comrows: Vec<usize> = Vec::new();
        let mut comcols: Vec<usize> = Vec::new();
        for i in 0..self.flowcands.len() {
            let r = self.flowcands[i];
            let (s, inv) = self.flowrow_fit(r, self.ncom);
            if s == 0 {
                continue;
            }
            self.ncom += 1;
            self.comsigns.push(0);
            let k = self.ncom - 1;
            if ncomnodes.len() < self.ncom {
                ncomnodes.push(0);
                deleted.push(false);
            }
            deleted[k] = false;
            comrows.clear();
            comcols.clear();
            self.newcols.clear();
            let (mut nr, mut ns, mut ninv) = (r, s, inv);
            loop {
                if ninv {
                    let (cr, cc) = (std::mem::take(&mut comrows), std::mem::take(&mut comcols));
                    self.invert_commodity(&cr, &cc);
                    comrows = cr;
                    comcols = cc;
                }
                self.add_flowrow(nr, ns, &mut comcols);
                comrows.push(nr);
                nflowrows += 1;
                match self.next_flowrow() {
                    Some((r2, s2, i2)) => {
                        nr = r2;
                        ns = s2;
                        ninv = i2;
                    }
                    None => break,
                }
            }
            let nnodes = comrows.len();
            ncomnodes[k] = nnodes;
            maxnnodes = maxnnodes.max(nnodes);
            nflowvars += comcols.len();
            if nnodes < MINNODES || (nnodes as f64) < MINCOMNODESFRACTION * maxnnodes as f64 {
                let cr = std::mem::take(&mut comrows);
                let (dr, dv) = self.delete_commodity(k, &cr);
                comrows = cr;
                nflowrows -= dr;
                nflowvars -= dv;
                ncomnodes[k] = 0;
                deleted[k] = true;
            }
        }
        // 最大の品種に比べて小さい品種を消す
        let mut k = 0;
        while k < self.ncom {
            k += 1;
            let k = k - 1;
            if deleted[k] || ncomnodes[k] as f64 >= MINCOMNODESFRACTION * maxnnodes as f64 {
                continue;
            }
            let rows: Vec<usize> = self.flowcands.iter().copied().filter(|&r| self.rowcom[r] == k as i32).collect();
            let (dr, dv) = self.delete_commodity(k, &rows);
            nflowrows -= dr;
            nflowvars -= dv;
            deleted[k] = true;
        }
        let ok = nflowrows > 0 && (nflowvars as f64 / nflowrows as f64) <= MAXFLOWVARFLOWROWRATIO;
        (ok, nflowrows)
    }

    /// 容量の候補の行と点数、向きのある網か (SCIP の `extractCapacityRows`)
    fn extract_capacity_rows(&mut self) {
        let p = self.p;
        let ncom = self.ncom;
        let nactive = (self.ncom - self.nempty) as f64;
        let limit = 2.0; // 向きがわかるまでは 2 (向きのある網なら後で 1 として点数を直す)
        let mut ncolspercom = vec![0usize; ncom];
        let mut touched: Vec<usize> = Vec::new();
        let mut maxcolspercom = vec![0usize; p.m];
        let (mut dirsc, mut undirsc) = (0.0f64, 0.0f64);
        for r in 0..p.m {
            let row = &p.rows[r];
            if row.is_empty() || self.flowrowsigns[r] & (LHSASSIGNED | RHSASSIGNED) != 0 {
                continue;
            }
            for &k in &touched {
                ncolspercom[k] = 0;
            }
            touched.clear();
            let (mut npf, mut nnf, mut npc, mut nnc) = (0usize, 0usize, 0usize, 0usize);
            let mut ncovered = 0usize;
            let (mut same, mut sameabs, mut maxcap) = (0.0f64, 0.0f64, 0.0f64);
            let mut s = 0u8;
            if p.row_lo[r].is_finite() {
                s |= LHSPOSSIBLE;
            }
            if p.row_up[r].is_finite() {
                s |= RHSPOSSIBLE;
            }
            let mut sign = 0u8;
            for &(c, a) in row {
                let k = self.colcom[c];
                if k >= 0 {
                    let k = k as usize;
                    if same == 0.0 {
                        same = a;
                    } else if !feq(same, a) {
                        same = f64::MAX;
                    }
                    if sameabs == 0.0 {
                        sameabs = a.abs();
                    } else if !feq(sameabs, a.abs()) {
                        sameabs = f64::MAX;
                    }
                    if a > 0.0 {
                        npf += 1;
                    } else {
                        nnf += 1;
                    }
                    if ncolspercom[k] == 0 {
                        ncovered += 1;
                        touched.push(k);
                    }
                    ncolspercom[k] += 1;
                    maxcolspercom[r] = maxcolspercom[r].max(ncolspercom[k]);
                    if ncolspercom[k] >= 2 {
                        sign |= UNDIRECTED;
                    }
                } else {
                    maxcap = maxcap.max(a.abs());
                    if a > 0.0 {
                        npc += 1;
                    } else {
                        nnc += 1;
                    }
                }
            }
            if s == 0 || npf + nnf == 0 {
                continue;
            }
            self.capacityrowsigns[r] = s | sign;
            self.capacitycands.push(r);
            let nf = (npf + nnf) as f64;
            let excess = (nf / ncovered as f64 - limit).abs();
            let mut sc = 1.0 + 1000.0 * (2.0 - excess).max(0.0);
            if s & RHSPOSSIBLE != 0 && nnf == 0 && npc == 0 && nnc > 0 {
                sc += 1000.0;
            }
            if s & LHSPOSSIBLE != 0 && npf == 0 && npc > 0 && nnc == 0 {
                sc += 1000.0;
            }
            sc += 2000.0 * ncovered as f64 / (nactive + 3.0);
            if feq(same.abs(), 1.0) {
                sc += 500.0;
            }
            if same != 0.0 && same != f64::MAX {
                sc += 250.0;
            }
            if feq(sameabs, 1.0) {
                sc += 100.0;
            }
            if maxcap > 0.0 && !feq(maxcap, 1.0) {
                sc += 100.0;
            }
            sc += 20.0 * npf.max(nnf) as f64 / nf.max(1.0);
            sc += 10.0 * npc.max(nnc) as f64 / ((npc + nnc) as f64 + 1.0);
            if s & RHSPOSSIBLE != 0 && p.row_up[r] >= 0.0 {
                sc += 10.0;
            }
            if p.row_lo[r].is_finite() != p.row_up[r].is_finite() {
                sc += 10.0;
            }
            self.capacityrowscores[r] = sc;
            if sign & UNDIRECTED != 0 {
                undirsc += sc;
            } else {
                dirsc += sc;
            }
        }
        // 向きのある網か (点数の多数決)
        self.directed = dirsc > undirsc;
        if self.directed {
            for &r in &self.capacitycands {
                if self.capacityrowsigns[r] & UNDIRECTED != 0 && maxcolspercom[r] <= 2 {
                    self.capacityrowscores[r] -= 1000.0;
                }
            }
        }
        let sc = &self.capacityrowscores;
        self.capacitycands.sort_by(|&a, &b| sc[b].total_cmp(&sc[a]));
    }

    /// 容量の行を点数順に弧にして、流量の列を弧に割り当てる (SCIP の `extractCapacities`)
    fn extract_capacities(&mut self) {
        for i in 0..self.capacitycands.len() {
            let r = self.capacitycands[i];
            let (mut nas, mut nun) = (0usize, 0usize);
            let (mut npf, mut nnf) = (0usize, 0usize);
            for &(c, a) in self.row(r) {
                if self.colcom[c] == -1 {
                    continue;
                }
                if self.colarc[c] >= 0 {
                    nas += 1;
                } else {
                    nun += 1;
                }
                if a > 0.0 {
                    npf += 1;
                } else {
                    nnf += 1;
                }
            }
            // 流量の列の 1/3 以上がまだ弧に割り当てられていないときだけ使う
            if nun == 0 || nas >= 2 * nun {
                self.capacityrowsigns[r] |= DISCARDED;
                continue;
            }
            let a = self.caprows.len() as i32;
            self.caprows.push(Some(r));
            // SCIP は右辺がとれれば右辺を使う。両側がとれる行 (等式) では、流量の係数が正になる側を使う
            // (`c x - sum f = 0` を右辺で使うと流量が打ち消されない)
            let s = self.capacityrowsigns[r];
            let use_rhs = s & RHSPOSSIBLE != 0 && (s & LHSPOSSIBLE == 0 || npf >= nnf);
            self.capacityrowsigns[r] |= if use_rhs { RHSASSIGNED } else { LHSASSIGNED };
            for &(c, _) in &self.p.rows[r] {
                if self.colcom[c] >= 0 && self.colarc[c] == -1 {
                    self.colarc[c] = a;
                }
            }
        }
    }

    /// 流量の行の向きの倍率 (右辺側 +1、左辺側 -1、反転と品種の向きを含む)
    fn rowscale(&self, r: usize, with_com: bool) -> f64 {
        let mut s = if self.flowrowsigns[r] & RHSASSIGNED != 0 { 1.0 } else { -1.0 };
        if self.flowrowsigns[r] & INVERTED != 0 {
            s = -s;
        }
        if with_com && self.comsigns[self.rowcom[r] as usize] == -1 {
            s = -s;
        }
        s
    }

    /// 基準の行と行 r の弧の重なりの点数 (SCIP の `getNodeSimilarityScore`)。(点数, 品種を反転するか)
    #[allow(clippy::too_many_arguments)]
    fn similarity(&self, baselen: usize, basepat: &[i32], bpu: usize, bnu: usize, r: usize, pat: &mut [i32], ov: &mut Vec<usize>) -> (f64, bool) {
        let rowcom = self.rowcom[r] as usize;
        let mut rowcomsign = self.comsigns[rowcom];
        let row = self.row(r);
        let rs = self.rowscale(r, false);
        let (mut npu, mut nnu) = (0usize, 0usize);
        let mut incompatible = false;
        ov.clear();
        for &(c, a) in row {
            let mut valsign: i32 = if rs * a > 0.0 { 1 } else { -1 };
            let arc = self.colarc[c];
            if arc == -1 {
                if valsign > 0 {
                    npu += 1;
                } else {
                    nnu += 1;
                }
                continue;
            }
            let arc = arc as usize;
            if basepat[arc] != 0 {
                if self.directed {
                    let valid: i8 = if valsign * basepat[arc] > 0 { 1 } else { -1 };
                    if rowcomsign == 0 {
                        rowcomsign = valid;
                    } else if rowcomsign != valid {
                        incompatible = true;
                        break;
                    }
                } else {
                    valsign = 1;
                }
                if pat[arc] == 0 {
                    ov.push(arc);
                }
                pat[arc] += valsign;
            }
        }
        let mut overlap = 0.0;
        for &arc in ov.iter() {
            let (b, x) = (basepat[arc].abs() as f64, pat[arc].abs() as f64);
            overlap += if b > x { x / b } else { b / x };
            pat[arc] = 0;
        }
        if incompatible || overlap <= 0.0 {
            return (0.0, false);
        }
        let ncols = self.p.n as f64;
        let rowarcs = (row.len() - npu - nnu) as f64;
        let basearcs = (baselen - bpu - bnu) as f64;
        let inv = rowcomsign == -1;
        // (SCIP は重なる弧が 2 本以上なら +1000 するが、次の代入で上書きされて効いていない。同じにする)
        let mut sc = if self.directed { overlap - (rowarcs + basearcs - 2.0 * overlap) / (2.0 * ncols + 1.0) } else { overlap - (rowarcs + basearcs - 4.0 * overlap) / (2.0 * ncols + 1.0) };
        let (npu, nnu, bpu, bnu) = (npu as f64, nnu as f64, bpu as f64, bnu as f64);
        sc += if inv { 1.0 - ((nnu - bpu).abs() + (npu - bnu).abs()) / (2.0 * ncols + 1.0) } else { 1.0 - ((npu - bpu).abs() + (nnu - bnu).abs()) / (2.0 * ncols + 1.0) };
        (sc.max(1e-6), inv)
    }

    /// 流量保存の行に節点の番号を付ける (SCIP の `extractNodes`)
    fn extract_nodes(&mut self) {
        let narcs = self.caprows.len();
        let ncom = self.ncom;
        let mut basepat = vec![0i32; narcs];
        let mut pat = vec![0i32; narcs];
        let mut ov: Vec<usize> = Vec::new();
        let mut incident = vec![false; self.p.n];
        let mut processed = vec![false; self.p.m];
        let mut processed_list: Vec<usize> = Vec::new();
        for ni in 0..self.flowcands.len() {
            let r = self.flowcands[ni];
            let basecom = self.rowcom[r];
            if basecom == -1 || self.rownode[r] >= 0 {
                continue;
            }
            self.rownode[r] = self.nnodes as i32;
            self.nnodes += 1;
            if ncom == 1 {
                continue;
            }
            let rs = self.rowscale(r, true);
            let (mut bpu, mut bnu) = (0usize, 0usize);
            let mut basearcs: Vec<usize> = Vec::new();
            for &(c, a) in self.row(r) {
                let arc = self.colarc[c];
                let pos = !self.directed || rs * a > 0.0;
                if arc >= 0 {
                    if basepat[arc as usize] == 0 {
                        basearcs.push(arc as usize);
                    }
                    basepat[arc as usize] += if pos { 1 } else { -1 };
                } else if pos {
                    bpu += 1;
                } else {
                    bnu += 1;
                }
            }
            let mut best: Vec<Option<(usize, f64, bool)>> = vec![None; ncom];
            // 弧の容量の行を通じてつながる、ほかの品種の流量の列
            let mut newcols: Vec<usize> = Vec::new();
            for &(c, _) in self.row(r) {
                let arc = self.colarc[c];
                if arc < 0 {
                    continue;
                }
                let Some(cr) = self.caprows[arc as usize] else { continue };
                for &(c2, _) in self.row(cr) {
                    let k2 = self.colcom[c2];
                    if k2 == -1 || k2 == basecom || incident[c2] {
                        continue;
                    }
                    incident[c2] = true;
                    newcols.push(c2);
                }
            }
            let baselen = self.row(r).len();
            for &c in &newcols {
                incident[c] = false;
                for &(r2, _) in &self.p.cols[c] {
                    if processed[r2] {
                        continue;
                    }
                    processed[r2] = true;
                    processed_list.push(r2);
                    let rc = self.rowcom[r2];
                    if rc == -1 || self.rownode[r2] >= 0 {
                        continue;
                    }
                    let (sc, inv) = self.similarity(baselen, &basepat, bpu, bnu, r2, &mut pat, &mut ov);
                    if sc > best[rc as usize].map_or(0.0, |b| b.1) {
                        best[rc as usize] = Some((r2, sc, inv));
                    }
                }
            }
            for &r2 in &processed_list {
                processed[r2] = false;
            }
            processed_list.clear();
            for &a in &basearcs {
                basepat[a] = 0;
            }
            for (k, b) in best.iter().enumerate() {
                if let Some((r2, _, inv)) = *b {
                    self.rownode[r2] = self.nnodes as i32 - 1;
                    self.comsigns[k] = if inv { -1 } else { 1 };
                }
            }
        }
    }

    /// 弧のない品種と小さい品種を消し、使われない弧と節点を詰める (SCIP の `cleanupNetwork`)
    fn cleanup_network(&mut self) {
        let ncom = self.ncom;
        let narcs = self.caprows.len();
        let mut nnodespercom = vec![0usize; ncom];
        let mut narcspercom = vec![0usize; ncom];
        for &r in &self.flowcands {
            if self.rowcom[r] >= 0 {
                nnodespercom[self.rowcom[r] as usize] += 1;
            }
        }
        let mut inc = vec![false; ncom];
        for a in 0..narcs {
            let Some(r) = self.caprows[a] else { continue };
            for &(c, _) in self.row(r) {
                if self.colcom[c] >= 0 && self.colarc[c] == a as i32 {
                    inc[self.colcom[c] as usize] = true;
                }
            }
            for k in 0..ncom {
                if inc[k] {
                    narcspercom[k] += 1;
                    inc[k] = false;
                }
            }
        }
        let maxn = nnodespercom.iter().copied().max().unwrap_or(0);
        let thr = ((MINCOMNODESFRACTION * maxn as f64) as usize).max(MINNODES);
        let mut perm = vec![-1i32; ncom];
        let mut newn = 0usize;
        for k in 0..ncom {
            if nnodespercom[k] >= thr && narcspercom[k] >= 1 {
                perm[k] = newn as i32;
                self.comsigns[newn] = self.comsigns[k];
                newn += 1;
            }
        }
        if newn < ncom {
            let mut arcused = vec![false; narcs];
            let mut nodeused = vec![false; self.nnodes];
            for c in 0..self.p.n {
                if self.colcom[c] >= 0 {
                    self.colcom[c] = perm[self.colcom[c] as usize];
                    if self.colcom[c] == -1 {
                        self.colarc[c] = -1;
                    } else if self.colarc[c] >= 0 {
                        arcused[self.colarc[c] as usize] = true;
                    }
                }
            }
            for i in 0..self.flowcands.len() {
                let r = self.flowcands[i];
                if self.rowcom[r] >= 0 {
                    self.rowcom[r] = perm[self.rowcom[r] as usize];
                    if self.rowcom[r] == -1 {
                        self.rownode[r] = -1;
                    } else if self.rownode[r] >= 0 {
                        nodeused[self.rownode[r] as usize] = true;
                    }
                }
            }
            self.ncom = newn;
            self.comsigns.truncate(newn);
            let mut aperm = vec![-1i32; narcs];
            let mut newcap = Vec::new();
            for a in 0..narcs {
                if arcused[a] {
                    aperm[a] = newcap.len() as i32;
                    newcap.push(self.caprows[a]);
                }
            }
            self.caprows = newcap;
            for c in 0..self.p.n {
                if self.colarc[c] >= 0 {
                    self.colarc[c] = aperm[self.colarc[c] as usize];
                }
            }
            let mut vperm = vec![-1i32; self.nnodes];
            let mut nv = 0usize;
            for v in 0..self.nnodes {
                if nodeused[v] {
                    vperm[v] = nv as i32;
                    nv += 1;
                }
            }
            for &r in &self.flowcands {
                if self.rowcom[r] >= 0 && self.rownode[r] >= 0 {
                    self.rownode[r] = vperm[self.rownode[r] as usize];
                }
            }
            self.nnodes = nv;
        }
        self.nempty = 0;
    }

    /// 流量の列の始点・終点 (列の流量の行のうち、`out - in` の形で +1 の行の節点が始点、-1 が終点)
    fn incident_nodes(&self, c: usize) -> (i32, i32) {
        let (mut s, mut t) = (-1i32, -1i32);
        for &(r, a) in &self.p.cols[c] {
            if self.rownode[r] < 0 || self.rowcom[r] < 0 {
                continue;
            }
            let v = self.rownode[r];
            if self.rowscale(r, true) * a > 0.0 {
                s = v;
                if t >= 0 {
                    break;
                }
            } else {
                t = v;
                if s >= 0 {
                    break;
                }
            }
        }
        (s, t)
    }

    /// 弧の始点・終点を多数決で決める (SCIP の `identifySourcesTargets`)。(食い違いが上限以下か, 食い違いの数)
    fn identify_sources_targets(&mut self) -> (bool, f64) {
        let narcs = self.caprows.len();
        let nnodes = self.nnodes;
        let ncom = self.ncom;
        self.arc_src = vec![-1; narcs];
        self.arc_tgt = vec![-1; narcs];
        let mut srccnt = vec![0.0f64; nnodes];
        let mut tgtcnt = vec![0.0f64; nnodes];
        let mut percom = vec![0usize; ncom];
        let mut touched: Vec<usize> = Vec::new();
        let mut incons = 0.0f64;
        let maxincons = max_inconsistency_ratio() * narcs as f64;
        for a in 0..narcs {
            let Some(r) = self.caprows[a] else { continue };
            let row = self.row(r);
            let mut ntouchedcoms = 0usize;
            for &(c, _) in row {
                if self.colarc[c] >= 0 {
                    let k = self.colcom[c] as usize;
                    if percom[k] == 0 {
                        ntouchedcoms += 1;
                    }
                    percom[k] += 1;
                }
            }
            if ntouchedcoms == 0 {
                self.caprows[a] = None;
                continue;
            }
            touched.clear();
            let (mut tsrc, mut ttgt, mut tnode) = (0.0f64, 0.0f64, 0.0f64);
            for &(c, _) in row {
                if self.colarc[c] < 0 {
                    continue;
                }
                let k = self.colcom[c] as usize;
                let (s, t) = self.incident_nodes(c);
                let w = 1.0 / percom[k] as f64;
                if s >= 0 {
                    let s = s as usize;
                    if srccnt[s] == 0.0 && tgtcnt[s] == 0.0 {
                        touched.push(s);
                    }
                    srccnt[s] += w;
                    tsrc += w;
                }
                if t >= 0 {
                    let t = t as usize;
                    if srccnt[t] == 0.0 && tgtcnt[t] == 0.0 {
                        touched.push(t);
                    }
                    tgtcnt[t] += w;
                    ttgt += w;
                }
                if s >= 0 || t >= 0 {
                    tnode += w;
                }
            }
            for &(c, _) in row {
                if self.colarc[c] >= 0 {
                    percom[self.colcom[c] as usize] = 0;
                }
            }
            let (mut bs, mut bt) = (-1i32, -1i32);
            let (mut bsc, mut btc) = (0.0f64, 0.0f64);
            for &v in &touched {
                if self.directed {
                    if srccnt[v] >= tgtcnt[v] {
                        if srccnt[v] > bsc {
                            bs = v as i32;
                            bsc = srccnt[v];
                        }
                    } else if tgtcnt[v] > btc {
                        bt = v as i32;
                        btc = tgtcnt[v];
                    }
                } else {
                    let cnt = srccnt[v] + tgtcnt[v];
                    if cnt > bsc {
                        bt = bs;
                        btc = bsc;
                        bs = v as i32;
                        bsc = cnt;
                    } else if cnt > btc {
                        bt = v as i32;
                        btc = cnt;
                    }
                }
                srccnt[v] = 0.0;
                tgtcnt[v] = 0.0;
            }
            if !self.directed {
                tsrc = tnode;
                ttgt = tnode;
            }
            let sinc = (tsrc - bsc) / ntouchedcoms as f64;
            let tinc = (ttgt - btc) / ntouchedcoms as f64;
            if sinc > MAXARCINCONSISTENCYRATIO {
                bs = -1;
            }
            if tinc > MAXARCINCONSISTENCYRATIO {
                bt = -1;
            }
            self.arc_src[a] = bs;
            self.arc_tgt[a] = bt;
            incons += 0.5 * (sinc + tinc);
            if incons > maxincons {
                return (false, incons);
            }
        }
        (narcs > 0 && ncom > 0, incons)
    }

    /// 容量の行のない流量の列から容量なしの弧を作る (SCIP の `findUncapacitatedArcs` を簡単にしたもの: 節点の組ごとに数え、
    /// 品種の 8 割 (向きなしなら 2 倍) 以上の列がある組を弧にする)
    fn find_uncapacitated_arcs(&mut self) {
        let mut thr = (self.ncom as f64 * UNCAPACITATEDARCSTRESHOLD).ceil() as usize;
        if !self.directed {
            thr *= 2;
        }
        let mut cnt: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for c in 0..self.p.n {
            if self.colcom[c] < 0 || self.colarc[c] >= 0 {
                continue;
            }
            let (s, t) = self.incident_nodes(c);
            if s < 0 || t < 0 || s == t {
                continue;
            }
            let key = if self.directed || s < t { (s, t) } else { (t, s) };
            cnt.entry(key).or_default().push(c);
        }
        let mut keys: Vec<(i32, i32)> = cnt.keys().copied().collect();
        keys.sort_unstable();
        for key in keys {
            let cols = &cnt[&key];
            if cols.len() < thr.max(1) {
                continue;
            }
            let a = self.caprows.len() as i32;
            self.caprows.push(None);
            self.arc_src.push(key.0);
            self.arc_tgt.push(key.1);
            for &c in cols {
                self.colarc[c] = a;
            }
        }
    }

    /// 連結成分ごとに網を作る (節点の多い順に MAXNETWORKS 個まで、SCIP の `mcfnetworkExtract` の後半と `mcfnetworkFill`)
    fn components(&self) -> Vec<McfNetwork> {
        let nnodes = self.nnodes;
        let narcs = self.caprows.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nnodes];
        for a in 0..narcs {
            if self.arc_src[a] >= 0 {
                adj[self.arc_src[a] as usize].push(a);
            }
            if self.arc_tgt[a] >= 0 {
                adj[self.arc_tgt[a] as usize].push(a);
            }
        }
        let mut comp = vec![-1i32; nnodes];
        let mut comps: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
        for v0 in 0..nnodes {
            if comp[v0] >= 0 {
                continue;
            }
            let id = comps.len() as i32;
            let (mut nodes, mut arcs) = (vec![v0], Vec::new());
            comp[v0] = id;
            let mut i = 0;
            let mut arcseen: Vec<usize> = Vec::new();
            while i < nodes.len() {
                let v = nodes[i];
                i += 1;
                for &a in &adj[v] {
                    arcseen.push(a);
                    for u in [self.arc_src[a], self.arc_tgt[a]] {
                        if u >= 0 && comp[u as usize] < 0 {
                            comp[u as usize] = id;
                            nodes.push(u as usize);
                        }
                    }
                }
            }
            arcseen.sort_unstable();
            arcseen.dedup();
            arcs.extend(arcseen);
            comps.push((nodes, arcs));
        }
        let mut keep: Vec<usize> = (0..comps.len()).filter(|&i| comps[i].0.len() >= MINNODES && comps[i].1.len() >= MINARCS).collect();
        keep.sort_by_key(|&i| std::cmp::Reverse(comps[i].0.len()));
        keep.truncate(MAXNETWORKS);
        let mut nets = Vec::new();
        // 行 → 節点・品種 (流量の行)
        let mut node_rows_of: Vec<Vec<usize>> = vec![Vec::new(); nnodes];
        for &r in &self.flowcands {
            if self.rowcom[r] >= 0 && self.rownode[r] >= 0 {
                node_rows_of[self.rownode[r] as usize].push(r);
            }
        }
        for ci in keep {
            let (nodes, arcs) = &comps[ci];
            let mut vid = vec![-1i32; nnodes];
            for (i, &v) in nodes.iter().enumerate() {
                vid[v] = i as i32;
            }
            // この成分の品種
            let mut kid = vec![-1i32; self.ncom];
            let mut ncom = 0usize;
            for &v in nodes {
                for &r in &node_rows_of[v] {
                    let k = self.rowcom[r] as usize;
                    if kid[k] < 0 {
                        kid[k] = ncom as i32;
                        ncom += 1;
                    }
                }
            }
            let mut node_rows = vec![None; nodes.len() * ncom];
            for (i, &v) in nodes.iter().enumerate() {
                for &r in &node_rows_of[v] {
                    let k = kid[self.rowcom[r] as usize] as usize;
                    let mut sc = self.flowrowscalars[r];
                    if self.flowrowsigns[r] & LHSASSIGNED != 0 {
                        sc = -sc;
                    }
                    if self.comsigns[self.rowcom[r] as usize] == -1 {
                        sc = -sc;
                    }
                    node_rows[i * ncom + k] = Some((r, sc, self.flowrowsigns[r] & INVERTED != 0));
                }
            }
            let mut col_com = vec![-1i32; self.p.n];
            for c in 0..self.p.n {
                if self.colcom[c] >= 0 {
                    col_com[c] = kid[self.colcom[c] as usize];
                }
            }
            // 容量の行の倍率: 同じ品種の流量の係数がどの容量の行でもそろうように (2 値の流量の列で需要が係数のとき)
            let mut comdem = vec![0.0f64; ncom];
            let mut arc_cap = Vec::with_capacity(arcs.len());
            let (mut arc_src, mut arc_tgt) = (Vec::with_capacity(arcs.len()), Vec::with_capacity(arcs.len()));
            let mut nuncap = 0;
            for &a in arcs {
                if let Some(r) = self.caprows[a] {
                    let row = self.row(r);
                    let mut scale = 1.0;
                    for &(c, v) in row {
                        let k = col_com[c];
                        if k >= 0 && comdem[k as usize] != 0.0 {
                            scale = comdem[k as usize] / v;
                            break;
                        }
                    }
                    let mut scale = scale.abs();
                    if self.capacityrowsigns[r] & LHSASSIGNED != 0 {
                        scale = -scale;
                    }
                    for &(c, v) in row {
                        let k = col_com[c];
                        if k >= 0 && comdem[k as usize] == 0.0 {
                            comdem[k as usize] = scale * v;
                        }
                    }
                    arc_cap.push(Some((r, scale)));
                } else {
                    arc_cap.push(None);
                    nuncap += 1;
                }
                arc_src.push(if self.arc_src[a] >= 0 { vid[self.arc_src[a] as usize] } else { -1 });
                arc_tgt.push(if self.arc_tgt[a] >= 0 { vid[self.arc_tgt[a] as usize] } else { -1 });
            }
            let net = McfNetwork { nnodes: nodes.len(), ncom, directed: self.directed, node_rows, arc_cap, arc_src, arc_tgt, col_com, nuncap };
            if net.narcs() as f64 / net.nnodes as f64 <= MAXARCNODERATIO {
                nets.push(net);
            }
        }
        nets
    }
}

/// 節点の塊への分け方 (SCIP の `NODEPARTITION`)
struct Partition {
    cluster: Vec<usize>,
    nclusters: usize,
}

fn uf_find(rep: &mut [usize], mut v: usize) -> usize {
    while rep[v] != v {
        rep[v] = rep[rep[v]];
        v = rep[v];
    }
    v
}

/// 行 r の余裕 (`min(右辺 - 活動量, 活動量 - 左辺)`)
fn row_slack(act: &[f64], rlo: &[f64], rup: &[f64], r: usize) -> f64 {
    (rup[r] - act[r]).min(act[r] - rlo[r])
}

/// 節点の側の値 (`out - in <= -d` の形の右辺) を、行の倍率の符号で選んだ側から読む
fn node_rhs(rlo: &[f64], rup: &[f64], r: usize, sc: f64, inv: bool) -> f64 {
    let side = if sc > 0.0 { rup[r] } else { rlo[r] };
    let v = if inv { -side } else { side };
    v * sc
}

impl McfNetwork {
    /// 弧の余裕と双対値で節点の組を縮めて `ncl` 個の塊にする (SCIP の `nodepartitionCreate` と `nodepairqueueCreate`)
    fn partition(&self, lp_rows: &[Vec<(usize, f64)>], act: &[f64], rlo: &[f64], rup: &[f64], duals: &[f64], ncl: usize) -> Partition {
        let nn = self.nnodes;
        let mut pairs: HashMap<(usize, usize), f64> = HashMap::new();
        for a in 0..self.narcs() {
            let (s, t) = (self.arc_src[a], self.arc_tgt[a]);
            if s < 0 || t < 0 {
                continue;
            }
            let key = if s <= t { (s as usize, t as usize) } else { (t as usize, s as usize) };
            let w = match self.arc_cap[a] {
                Some((r, sc)) => {
                    let slack = row_slack(act, rlo, rup, r).max(0.0);
                    let maxv = lp_rows[r].iter().fold(0.0f64, |mx, &(_, v)| mx.max(v.abs())).max(1e-12);
                    let scale = sc.abs() / maxv;
                    scale * slack - duals.get(r).copied().unwrap_or(0.0).abs() / scale
                }
                None => f64::INFINITY,
            };
            let e = pairs.entry(key).or_insert(w);
            *e = e.min(w);
        }
        // 需要のある節点どうしの組は先に縮めない (SCIP の BETTERWEIGHTFORDEMANDNODES)
        let has_dem: Vec<bool> = (0..nn)
            .map(|v| {
                (0..self.ncom).any(|k| {
                    self.node_rows[v * self.ncom + k].is_some_and(|(r, sc, _)| {
                        let side = if sc > 0.0 { rup[r] } else { rlo[r] };
                        side.abs() > 1e-9
                    })
                })
            })
            .collect();
        let (mut maxw, mut minw) = (1.0f64, -1.0f64);
        for &w in pairs.values() {
            if w.is_finite() {
                maxw = maxw.max(w);
            }
            minw = minw.min(w);
        }
        let mut list: Vec<((usize, usize), f64)> = pairs.into_iter().collect();
        for ((u, v), w) in list.iter_mut() {
            if *w > 1e-9 {
                if !has_dem[*u] || !has_dem[*v] {
                    *w += maxw;
                }
            } else if has_dem[*u] && has_dem[*v] {
                *w += minw;
            }
        }
        list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut rep: Vec<usize> = (0..nn).collect();
        let mut left = nn;
        for ((u, v), _) in list {
            if left <= ncl {
                break;
            }
            let (ru, rv) = (uf_find(&mut rep, u), uf_find(&mut rep, v));
            if ru == rv {
                continue;
            }
            // 小さい番号が代表 (節点 0 はいつも代表)
            if ru < rv {
                rep[rv] = ru;
            } else {
                rep[ru] = rv;
            }
            left -= 1;
        }
        // 弧が足りずに縮めきれなければ、残りを節点 0 の塊にまとめる
        let mut v = 1;
        while left > ncl && v < nn {
            let r = uf_find(&mut rep, v);
            if r != 0 {
                rep[r] = 0;
                left -= 1;
            }
            v += 1;
        }
        let mut cluster = vec![0usize; nn];
        let mut cid = vec![usize::MAX; nn];
        let mut nc = 0;
        for v in 0..nn {
            let r = uf_find(&mut rep, v);
            if cid[r] == usize::MAX {
                cid[r] = nc;
                nc += 1;
            }
            cluster[v] = cid[r];
        }
        Partition { cluster, nclusters: nc }
    }

    /// 塊の部分集合 S (ビット) と T の両方がつながっているか (SCIP の `nodepartitionIsConnected`)
    fn partition_connected(&self, part: &Partition, bits: u32) -> bool {
        let nc = part.nclusters;
        let mut rep: Vec<usize> = (0..nc).collect();
        for a in 0..self.narcs() {
            let (s, t) = (self.arc_src[a], self.arc_tgt[a]);
            if s < 0 || t < 0 {
                continue;
            }
            let (cs, ct) = (part.cluster[s as usize], part.cluster[t as usize]);
            if cs == ct || ((bits >> cs) & 1) != ((bits >> ct) & 1) {
                continue;
            }
            let (r1, r2) = (uf_find(&mut rep, cs), uf_find(&mut rep, ct));
            if r1 != r2 {
                rep[r1.max(r2)] = r1.min(r2);
            }
        }
        let (mut rs, mut rt) = (None, None);
        for c in 0..nc {
            let r = uf_find(&mut rep, c);
            let side = if (bits >> c) & 1 == 1 { &mut rs } else { &mut rt };
            match *side {
                None => *side = Some(r),
                Some(x) if x != r => return false,
                _ => {}
            }
        }
        true
    }

    /// 節点の集合 S に対する cut-set の集約の重み (行, 重み) を `out` に足す (SCIP の `generateClusterCuts` の集約の部分)。
    /// `in_s(v)` は節点 v が S に入るか。集約がなければ何もしない
    fn aggregate(&self, in_s: &dyn Fn(usize) -> bool, lp_rows: &[Vec<(usize, f64)>], act: &[f64], rlo: &[f64], rup: &[f64], w: &mut Vec<(usize, f64)>) -> bool {
        let ncom = self.ncom;
        let ins = |v: i32| v >= 0 && in_s(v as usize);
        let mut ccd = vec![0.0f64; ncom];
        for v in 0..self.nnodes {
            if !in_s(v) {
                continue;
            }
            for k in 0..ncom {
                if let Some((r, sc, inv)) = self.node_rows[v * ncom + k] {
                    ccd[k] += node_rhs(rlo, rup, r, sc, inv);
                }
            }
        }
        let useful = if self.directed { ccd.iter().any(|&d| d < -1e-9) } else { ccd.iter().any(|&d| d.abs() > 1e-9) };
        if !useful {
            return false;
        }
        w.clear();
        let mut comdem = vec![0.0f64; ncom];
        for a in 0..self.narcs() {
            let (s, t) = (self.arc_src[a], self.arc_tgt[a]);
            if self.directed {
                // T から S に入る弧
                if ins(s) || !ins(t) {
                    continue;
                }
            } else if ins(s) == ins(t) {
                continue;
            }
            let Some((r, sc)) = self.arc_cap[a] else { continue };
            let slack = row_slack(act, rlo, rup, r);
            if s < 0 || t < 0 {
                if slack > FEASTOL {
                    continue;
                }
            } else {
                let maxv = lp_rows[r].iter().fold(0.0f64, |mx, &(_, v)| mx.max(v.abs())).max(1e-12);
                if slack / maxv > MAXCAPACITYSLACK + FEASTOL {
                    continue;
                }
            }
            w.push((r, sc));
            for &(c, v) in &lp_rows[r] {
                let k = self.col_com[c];
                if k >= 0 {
                    comdem[k as usize] = (v * sc).abs();
                }
            }
        }
        if w.is_empty() {
            return false;
        }
        let ncap = w.len();
        for v in 0..self.nnodes {
            for k in 0..ncom {
                if comdem[k] == 0.0 {
                    continue;
                }
                if self.directed {
                    if ccd[k] >= -1e-9 || !in_s(v) {
                        continue;
                    }
                } else {
                    if ccd[k].abs() <= 1e-9 {
                        continue;
                    }
                    // 需要の向きが逆なら T の側で足す
                    if (ccd[k] > 0.0) == in_s(v) {
                        continue;
                    }
                }
                let Some((r, sc, inv)) = self.node_rows[v * ncom + k] else { continue };
                // 余裕のある行は使わない
                if row_slack(act, rlo, rup, r) > FEASTOL {
                    continue;
                }
                let wt = comdem[k] * sc * if inv { -1.0 } else { 1.0 };
                w.push((r, wt));
            }
        }
        if w.len() == ncap {
            return false;
        }
        let (mut wmax, mut wmin) = (0.0f64, f64::INFINITY);
        for &(_, x) in w.iter() {
            wmax = wmax.max(x.abs());
            wmin = wmin.min(x.abs());
        }
        wmin > 0.0 && wmax / wmin <= MAXWEIGHTRANGE
    }
}

/// 網の cut-set の集約 (行の重み) を列挙する: 1 節点ずつ (向きのある網では S = {v} と T = {v} の両方)、続いて塊の部分集合。
/// `max_aggs` 本まで
#[allow(clippy::too_many_arguments)]
pub(super) fn aggregations(nets: &[McfNetwork], lp_rows: &[Vec<(usize, f64)>], act: &[f64], rlo: &[f64], rup: &[f64], duals: &[f64], ncl: usize, max_aggs: usize, single: bool) -> Vec<Vec<(usize, f64)>> {
    let mut out: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut w: Vec<(usize, f64)> = Vec::new();
    for net in nets {
        // 行の番号が LP の外なら (行の削除などで) 使わない
        if net.node_rows.iter().flatten().any(|&(r, _, _)| r >= lp_rows.len()) {
            continue;
        }
        let invs: &[bool] = if net.directed { &[false, true] } else { &[false] };
        if single {
            for v in 0..net.nnodes {
                for &inv in invs {
                    if out.len() >= max_aggs {
                        return out;
                    }
                    if net.aggregate(&|u| (u == v) != inv, lp_rows, act, rlo, rup, &mut w) {
                        out.push(w.clone());
                    }
                }
            }
        }
        let part = net.partition(lp_rows, act, rlo, rup, duals, ncl.max(2));
        if part.nclusters < 2 {
            continue;
        }
        for bits in 1u32..(1u32 << (part.nclusters - 1)) {
            if !net.partition_connected(&part, bits) {
                continue;
            }
            let ns = (0..net.nnodes).filter(|&v| (bits >> part.cluster[v]) & 1 == 1).count();
            // 1 節点だけの S・T は 1 節点のカットで試した
            if single && (ns == 1 || ns + 1 == net.nnodes) {
                continue;
            }
            for &inv in invs {
                if out.len() >= max_aggs {
                    return out;
                }
                if net.aggregate(&|u| ((bits >> part.cluster[u]) & 1 == 1) != inv, lp_rows, act, rlo, rup, &mut w) {
                    out.push(w.clone());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 節点 12 個・弧 (i -> i+1, i -> i+2, i+1 -> i) の固定費ネットワーク設計、品種 3 個 (k: 節点 k から最後の節点へ需要 k + 1)。
    /// 流量 f_a^k は連続、弧を開く y_a は 0-1、容量 `sum_k f_a^k - 4 y_a <= 0`、流量保存は `out - in = 供給`
    fn network_design() -> (MipProblem, usize, usize) {
        let nv = 12;
        let mut arcs = Vec::new();
        for i in 0..nv - 1 {
            arcs.push((i, i + 1));
            arcs.push((i + 1, i));
            if i + 2 < nv {
                arcs.push((i, i + 2));
            }
        }
        let (na, nk) = (arcs.len(), 3);
        let n = na * nk + na;
        let f = |a: usize, k: usize| a * nk + k;
        let y = |a: usize| na * nk + a;
        let mut rows = Vec::new();
        let (mut lo, mut up) = (Vec::new(), Vec::new());
        for k in 0..nk {
            for v in 0..nv {
                let mut r: Vec<(usize, f64)> = Vec::new();
                for (a, &(s, t)) in arcs.iter().enumerate() {
                    if s == v {
                        r.push((f(a, k), 1.0));
                    }
                    if t == v {
                        r.push((f(a, k), -1.0));
                    }
                }
                r.sort_by_key(|e| e.0);
                let d = (k + 1) as f64;
                let b = if v == k { d } else if v == nv - 1 { -d } else { 0.0 };
                rows.push(r);
                lo.push(b);
                up.push(b);
            }
        }
        for a in 0..na {
            let mut r: Vec<(usize, f64)> = (0..nk).map(|k| (f(a, k), 1.0)).collect();
            r.push((y(a), -4.0));
            rows.push(r);
            lo.push(f64::NEG_INFINITY);
            up.push(0.0);
        }
        let mut is_int = vec![false; n];
        let mut col_up = vec![f64::INFINITY; n];
        for a in 0..na {
            is_int[y(a)] = true;
            col_up[y(a)] = 1.0;
        }
        let cost: Vec<f64> = (0..n).map(|j| if j >= na * nk { 10.0 } else { 1.0 }).collect();
        (MipProblem::from_rows(vec![0.0; n], col_up, cost, 0.0, 1.0, is_int, rows, lo, up), nv, na)
    }

    #[test]
    fn detects_network_design() {
        let (p, nv, na) = network_design();
        let (nets, st) = extract(&p);
        assert_eq!(st.fail, "", "{st:?}");
        assert_eq!(nets.len(), 1, "{st:?}");
        let g = &nets[0];
        assert_eq!(g.nnodes, nv);
        assert_eq!(g.ncom, 3);
        assert_eq!(g.narcs(), na);
        assert!(g.directed);
        // 弧の始点・終点が元の弧の向きと同じ並びになる (節点の番号は付け替わるので、始点・終点がすべて決まっていることだけを見る)
        assert!(g.arc_src.iter().all(|&s| s >= 0) && g.arc_tgt.iter().all(|&t| t >= 0));
        // LP の点: 流量は 0、y は 0 (容量の行に余裕なし)。集約が出る
        let lp_rows = p.rows.clone();
        let act = vec![0.0; p.m];
        let aggs = aggregations(&nets, &lp_rows, &act, &p.row_lo, &p.row_up, &vec![0.0; p.m], 5, 1000, true);
        assert!(!aggs.is_empty());
        // 需要の終点を S にした集約は、5 に入る弧の容量の行を含む
        assert!(aggs.iter().any(|w| w.iter().filter(|&&(r, _)| r >= 3 * nv).count() >= 2));
    }
}
