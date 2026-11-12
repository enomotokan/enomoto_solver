//! ネットワーク行列の判定 (analysis/drcrtu_20261111.md)。0/±1 の行列 M (行 = 木の弧、列 = 木でない弧) が、ある有向グラフの全域木に
//! ついての基本閉路の行列 (ネットワーク行列) かを判定する。ネットワーク行列とその転置は TU。
//!
//! - 判定の本体は van der Hulst の MATREC (github の rolfvdhulst/matrec、MIT ライセンス、Copyright (c) 2024 Rolf van der Hulst) の
//!   src/Network.c の「列の追加」(Bixby-Wagner の方法: SPQR 木 (t 分解) を保ち、新しい列の行が木の道になる実現があるかを確かめて
//!   木を更新する) を関数ごとにそのまま Rust に移したもの。関数名は C の名前を snake_case にした。DRCR の論文 (van der Hulst 2026) の
//!   著者の実装 (rolfvdhulst/folding の AnalyzePartition.cpp) もこの部品で整数の組の TU を確かめている。
//! - 移植の誤りで TU でない行列を TU と言わないように、列を全部足し終えたら分解から実際のグラフ (木 + 木でない弧) を組み立て、
//!   全部の列について「列の非零の行 = 列の両端を結ぶ木の道の弧」と「符号が行・列の ±1 倍を除いて道の向きと一致する」(2 色塗り) を
//!   確かめる ([`verify`]、Camion の定理で、TU な符号づけは行・列の ±1 倍を除いて一つなので、この検査は完全でもある)。
//!   通らなければ偽 (安全側)。2-和の貼り合わせの向きや直列の弧の並びは、グラフの閉路マトロイドを変えないので、検査は向きの情報に頼らない。
//! - 移植の誤りで添字が外れるなどして panic したら偽を返す ([`is_network_cols`] の `catch_unwind`)。

use std::cell::Cell;

const INV: i32 = -1;
const MARKER_ROW: i32 = i32::MIN;
const MARKER_COL: i32 = i32::MAX;

thread_local! {
    /// 診断: panic で偽にした回数・証明書の検査で偽にした回数 (単体テストと数えるモードで見る)
    pub static NET_PANICS: Cell<usize> = const { Cell::new(0) };
    pub static NET_VERIFY_FAIL: Cell<usize> = const { Cell::new(0) };
}

#[derive(Clone, Copy, Default)]
struct Ln {
    prev: i32,
    next: i32,
}

#[derive(Clone, Copy)]
struct Arc {
    head: i32,
    tail: i32,
    member: i32,
    child_member: i32,
    head_ln: Ln,
    tail_ln: Ln,
    ln: Ln,
    element: i32,
    representative: i32,
    reversed: bool,
}

#[derive(Clone, Copy)]
struct Node {
    rep: i32,
    first_arc: i32,
    num_arcs: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mt {
    Rigid,
    Parallel,
    Series,
    Loop,
    Unassigned,
}

#[derive(Clone, Copy)]
struct Member {
    rep: i32,
    ty: Mt,
    parent: i32,
    marker_to_parent: i32,
    marker_of_parent: i32,
    first_arc: i32,
    num_arcs: i32,
}

fn element_is_row(e: i32) -> bool {
    e < 0
}
fn row_to_element(r: i32) -> i32 {
    -r - 1
}
fn element_to_row(e: i32) -> i32 {
    -e - 1
}
fn valid(x: i32) -> bool {
    x >= 0
}

/// SPQR 木 (MATREC の `MATRECNetworkDecomposition`)。
struct Dec {
    arcs: Vec<Arc>,
    members: Vec<Member>,
    nodes: Vec<Node>,
    row_arcs: Vec<i32>,
    col_arcs: Vec<i32>,
    num_cc: i32,
}

impl Dec {
    fn new(nrows: usize, ncols: usize) -> Dec {
        Dec { arcs: Vec::new(), members: Vec::new(), nodes: Vec::new(), row_arcs: vec![INV; nrows], col_arcs: vec![INV; ncols], num_cc: 0 }
    }

    // ---- 節点の union-find ----
    fn find_node(&mut self, node: i32) -> i32 {
        let mut cur = node;
        loop {
            let nx = self.nodes[cur as usize].rep;
            if !valid(nx) {
                break;
            }
            cur = nx;
        }
        let root = cur;
        cur = node;
        loop {
            let nx = self.nodes[cur as usize].rep;
            if !valid(nx) {
                break;
            }
            self.nodes[cur as usize].rep = root;
            cur = nx;
        }
        root
    }
    fn find_arc_tail(&mut self, arc: i32) -> i32 {
        let r = self.find_node(self.arcs[arc as usize].tail);
        self.arcs[arc as usize].tail = r;
        r
    }
    fn find_arc_head(&mut self, arc: i32) -> i32 {
        let r = self.find_node(self.arcs[arc as usize].head);
        self.arcs[arc as usize].head = r;
        r
    }
    fn get_first_node_arc(&self, node: i32) -> i32 {
        self.nodes[node as usize].first_arc
    }
    fn get_next_node_arc(&mut self, arc: i32, node: i32) -> i32 {
        if self.find_arc_head(arc) == node {
            self.arcs[arc as usize].head_ln.next
        } else {
            self.arcs[arc as usize].tail = node;
            self.arcs[arc as usize].tail_ln.next
        }
    }
    fn get_previous_node_arc(&mut self, arc: i32, node: i32) -> i32 {
        if self.find_arc_head(arc) == node {
            self.arcs[arc as usize].head_ln.prev
        } else {
            self.arcs[arc as usize].tail = node;
            self.arcs[arc as usize].tail_ln.prev
        }
    }
    fn node_ln(&mut self, arc: i32, is_head: bool) -> &mut Ln {
        if is_head {
            &mut self.arcs[arc as usize].head_ln
        } else {
            &mut self.arcs[arc as usize].tail_ln
        }
    }
    fn merge_node_arc_list(&mut self, into: i32, from: i32) {
        let first_into = self.get_first_node_arc(into);
        let first_from = self.get_first_node_arc(from);
        if !valid(first_into) {
            self.nodes[into as usize].num_arcs += self.nodes[from as usize].num_arcs;
            self.nodes[from as usize].num_arcs = 0;
            self.nodes[into as usize].first_arc = self.nodes[from as usize].first_arc;
            self.nodes[from as usize].first_arc = INV;
            return;
        } else if !valid(first_from) {
            return;
        }
        let last_into = self.get_previous_node_arc(first_into, into);
        let last_from = self.get_previous_node_arc(first_from, from);
        let fi_h = self.find_arc_head(first_into) == into;
        let li_h = self.find_arc_head(last_into) == into;
        let ff_h = self.find_arc_head(first_from) == from;
        let lf_h = self.find_arc_head(last_from) == from;
        self.node_ln(first_into, fi_h).prev = last_from;
        self.node_ln(last_into, li_h).next = first_from;
        self.node_ln(first_from, ff_h).prev = last_into;
        self.node_ln(last_from, lf_h).next = first_into;
        self.nodes[into as usize].num_arcs += self.nodes[from as usize].num_arcs;
        self.nodes[from as usize].num_arcs = 0;
        self.nodes[from as usize].first_arc = INV;
    }
    fn arc_flip_reversed(&mut self, arc: i32) {
        self.arcs[arc as usize].reversed = !self.arcs[arc as usize].reversed;
    }
    fn arc_set_reversed(&mut self, arc: i32, r: bool) {
        self.arcs[arc as usize].reversed = r;
    }
    fn arc_set_representative(&mut self, arc: i32, rep: i32) {
        self.arcs[arc as usize].representative = rep;
    }
    fn merge_nodes(&mut self, first: i32, second: i32) -> i32 {
        let (mut first, mut second) = (first, second);
        let first_rank = self.nodes[first as usize].rep;
        let second_rank = self.nodes[second as usize].rep;
        if first_rank > second_rank {
            std::mem::swap(&mut first, &mut second);
        }
        self.merge_node_arc_list(first, second);
        self.nodes[second as usize].rep = first;
        if first_rank == second_rank {
            self.nodes[first as usize].rep -= 1;
        }
        first
    }

    // ---- 部分 (member) の union-find ----
    fn member_is_representative(&self, m: i32) -> bool {
        !valid(self.members[m as usize].rep)
    }
    fn find_member(&mut self, member: i32) -> i32 {
        let mut cur = member;
        loop {
            let nx = self.members[cur as usize].rep;
            if !valid(nx) {
                break;
            }
            cur = nx;
        }
        let root = cur;
        cur = member;
        loop {
            let nx = self.members[cur as usize].rep;
            if !valid(nx) {
                break;
            }
            self.members[cur as usize].rep = root;
            cur = nx;
        }
        root
    }
    fn merge_members(&mut self, first: i32, second: i32) -> i32 {
        let (mut first, mut second) = (first, second);
        let first_rank = self.members[first as usize].rep;
        let second_rank = self.members[second as usize].rep;
        if first_rank > second_rank {
            std::mem::swap(&mut first, &mut second);
        }
        self.members[second as usize].rep = first;
        if first_rank == second_rank {
            self.members[first as usize].rep -= 1;
        }
        first
    }
    fn find_arc_member(&mut self, arc: i32) -> i32 {
        let r = self.find_member(self.arcs[arc as usize].member);
        self.arcs[arc as usize].member = r;
        r
    }
    fn find_member_parent(&mut self, member: i32) -> i32 {
        let p = self.members[member as usize].parent;
        if !valid(p) {
            return p;
        }
        let r = self.find_member(p);
        self.members[member as usize].parent = r;
        r
    }
    fn find_arc_child_member(&mut self, arc: i32) -> i32 {
        let r = self.find_member(self.arcs[arc as usize].child_member);
        self.arcs[arc as usize].child_member = r;
        r
    }
    /// 子への印の弧か (親への印は含まない。C の `arcIsMarker` と同じ)
    fn arc_is_marker(&self, arc: i32) -> bool {
        valid(self.arcs[arc as usize].child_member)
    }
    fn arc_is_tree(&self, arc: i32) -> bool {
        element_is_row(self.arcs[arc as usize].element)
    }

    // ---- 弧の向きの符号つき union-find ----
    fn find_arc_sign(&mut self, arc: i32) -> (i32, bool) {
        let mut cur = arc;
        let mut total = self.arcs[cur as usize].reversed;
        loop {
            let nx = self.arcs[cur as usize].representative;
            if !valid(nx) {
                break;
            }
            cur = nx;
            total ^= self.arcs[cur as usize].reversed;
        }
        let root = cur;
        cur = arc;
        let mut cur_rev = total != self.arcs[root as usize].reversed;
        loop {
            let nx = self.arcs[cur as usize].representative;
            if !valid(nx) {
                break;
            }
            let was = self.arcs[cur as usize].reversed;
            self.arcs[cur as usize].reversed = cur_rev;
            cur_rev = cur_rev != was;
            self.arcs[cur as usize].representative = root;
            cur = nx;
        }
        (root, total)
    }
    fn find_effective_arc_head(&mut self, arc: i32) -> i32 {
        if self.find_arc_sign(arc).1 {
            self.find_arc_tail(arc)
        } else {
            self.find_arc_head(arc)
        }
    }
    fn find_effective_arc_tail(&mut self, arc: i32) -> i32 {
        if self.find_arc_sign(arc).1 {
            self.find_arc_head(arc)
        } else {
            self.find_arc_tail(arc)
        }
    }
    fn merge_arc_signs(&mut self, first: i32, second: i32, reflect_relative: bool) -> i32 {
        let (mut first, mut second) = (first, second);
        let first_rank = self.arcs[first as usize].representative;
        let second_rank = self.arcs[second as usize].representative;
        if first_rank > second_rank {
            std::mem::swap(&mut first, &mut second);
        }
        self.arcs[second as usize].representative = first;
        if first_rank == second_rank {
            self.arcs[first as usize].representative -= 1;
        }
        let equal = self.arcs[first as usize].reversed == self.arcs[second as usize].reversed;
        self.arcs[second as usize].reversed = equal == reflect_relative;
        if first_rank > second_rank {
            self.arcs[first as usize].reversed = self.arcs[first as usize].reversed != reflect_relative;
        }
        first
    }
    fn arc_is_reversed_non_rigid(&self, arc: i32) -> bool {
        self.arcs[arc as usize].reversed
    }

    // ---- 部分の弧の並び ----
    fn get_first_member_arc(&self, m: i32) -> i32 {
        self.members[m as usize].first_arc
    }
    fn get_next_member_arc(&self, arc: i32) -> i32 {
        self.arcs[arc as usize].ln.next
    }
    fn get_previous_member_arc(&self, arc: i32) -> i32 {
        self.arcs[arc as usize].ln.prev
    }
    fn add_arc_to_member_arc_list(&mut self, arc: i32, member: i32) {
        let first = self.get_first_member_arc(member);
        if valid(first) {
            let last = self.get_previous_member_arc(first);
            self.arcs[arc as usize].ln.next = first;
            self.arcs[arc as usize].ln.prev = last;
            self.arcs[first as usize].ln.prev = arc;
            self.arcs[last as usize].ln.next = arc;
        } else {
            self.arcs[arc as usize].ln.next = arc;
            self.arcs[arc as usize].ln.prev = arc;
        }
        self.members[member as usize].first_arc = arc;
        self.members[member as usize].num_arcs += 1;
    }
    fn create_arc(&mut self, member: i32, reversed: bool) -> i32 {
        let idx = self.arcs.len() as i32;
        self.arcs.push(Arc {
            head: INV,
            tail: INV,
            member,
            child_member: INV,
            head_ln: Ln { prev: INV, next: INV },
            tail_ln: Ln { prev: INV, next: INV },
            ln: Ln::default(),
            element: 0,
            representative: INV,
            reversed,
        });
        idx
    }
    fn create_row_arc(&mut self, member: i32, row: i32, reversed: bool) -> i32 {
        let a = self.create_arc(member, reversed);
        self.row_arcs[row as usize] = a;
        self.add_arc_to_member_arc_list(a, member);
        self.arcs[a as usize].element = row_to_element(row);
        a
    }
    fn create_column_arc(&mut self, member: i32, col: i32, reversed: bool) -> i32 {
        let a = self.create_arc(member, reversed);
        self.col_arcs[col as usize] = a;
        self.add_arc_to_member_arc_list(a, member);
        self.arcs[a as usize].element = col;
        a
    }
    fn create_member(&mut self, ty: Mt) -> i32 {
        let idx = self.members.len() as i32;
        self.members.push(Member { rep: INV, ty, parent: INV, marker_to_parent: INV, marker_of_parent: INV, first_arc: INV, num_arcs: 0 });
        idx
    }
    fn create_node(&mut self) -> i32 {
        let idx = self.nodes.len() as i32;
        self.nodes.push(Node { rep: INV, first_arc: INV, num_arcs: 0 });
        idx
    }
    fn add_arc_to_node_arc_list(&mut self, arc: i32, node: i32, node_is_head: bool) {
        let first = self.get_first_node_arc(node);
        if valid(first) {
            let next_is_head = self.find_arc_head(first) == node;
            let last = self.node_ln(first, next_is_head).prev;
            {
                let l = self.node_ln(arc, node_is_head);
                l.next = first;
                l.prev = last;
            }
            let prev_is_head = self.find_arc_head(last) == node;
            self.node_ln(last, prev_is_head).next = arc;
            self.node_ln(first, next_is_head).prev = arc;
        } else {
            let l = self.node_ln(arc, node_is_head);
            l.next = arc;
            l.prev = arc;
        }
        self.nodes[node as usize].first_arc = arc;
        self.nodes[node as usize].num_arcs += 1;
        if node_is_head {
            self.arcs[arc as usize].head = node;
        } else {
            self.arcs[arc as usize].tail = node;
        }
    }
    fn set_arc_head_and_tail(&mut self, arc: i32, head: i32, tail: i32) {
        self.add_arc_to_node_arc_list(arc, head, true);
        self.add_arc_to_node_arc_list(arc, tail, false);
    }
    fn remove_arc_from_node_arc_list(&mut self, arc: i32, node: i32, node_is_head: bool) {
        if self.nodes[node as usize].num_arcs == 1 {
            self.nodes[node as usize].first_arc = INV;
        } else {
            let l = *self.node_ln(arc, node_is_head);
            let (next_arc, prev_arc) = (l.next, l.prev);
            let nh = self.find_arc_head(next_arc) == node;
            let ph = self.find_arc_head(prev_arc) == node;
            self.node_ln(next_arc, nh).prev = prev_arc;
            self.node_ln(prev_arc, ph).next = next_arc;
            if self.nodes[node as usize].first_arc == arc {
                self.nodes[node as usize].first_arc = next_arc;
            }
        }
        self.nodes[node as usize].num_arcs -= 1;
    }
    fn clear_arc_head_and_tail(&mut self, arc: i32) {
        let h = self.find_arc_head(arc);
        self.remove_arc_from_node_arc_list(arc, h, true);
        let t = self.find_arc_tail(arc);
        self.remove_arc_from_node_arc_list(arc, t, false);
        self.arcs[arc as usize].head = INV;
        self.arcs[arc as usize].tail = INV;
    }
    fn get_member_type(&self, m: i32) -> Mt {
        self.members[m as usize].ty
    }
    fn update_member_type(&mut self, m: i32, ty: Mt) {
        self.members[m as usize].ty = ty;
    }
    fn marker_to_parent(&self, m: i32) -> i32 {
        self.members[m as usize].marker_to_parent
    }
    fn marker_of_parent(&self, m: i32) -> i32 {
        self.members[m as usize].marker_of_parent
    }
    fn update_member_parent_information(&mut self, new_member: i32, to_remove: i32) {
        let t = self.members[to_remove as usize];
        self.members[new_member as usize].marker_of_parent = t.marker_of_parent;
        self.members[new_member as usize].marker_to_parent = t.marker_to_parent;
        self.members[new_member as usize].parent = t.parent;
        self.members[to_remove as usize].marker_of_parent = INV;
        self.members[to_remove as usize].marker_to_parent = INV;
        self.members[to_remove as usize].parent = INV;
    }
    fn get_num_member_arcs(&self, m: i32) -> i32 {
        self.members[m as usize].num_arcs
    }
    fn create_standalone_series(&mut self, rows: &[i32], reversed: &[bool], col: i32) -> i32 {
        let member = self.create_member(if rows.len() <= 1 { Mt::Loop } else { Mt::Series });
        self.create_column_arc(member, col, false);
        for (i, &r) in rows.iter().enumerate() {
            self.create_row_arc(member, r, !reversed[i]);
        }
        self.num_cc += 1;
        member
    }
    fn create_connected_series(&mut self, rows: &[i32], reversed: &[bool], col: i32) -> i32 {
        let member = self.create_member(Mt::Series);
        self.create_column_arc(member, col, false);
        for (i, &r) in rows.iter().enumerate() {
            self.create_row_arc(member, r, !reversed[i]);
        }
        member
    }
    fn remove_arc_from_member_arc_list(&mut self, arc: i32, member: i32) {
        if self.members[member as usize].num_arcs == 1 {
            self.members[member as usize].first_arc = INV;
        } else {
            let next = self.arcs[arc as usize].ln.next;
            let prev = self.arcs[arc as usize].ln.prev;
            self.arcs[next as usize].ln.prev = prev;
            self.arcs[prev as usize].ln.next = next;
            if self.members[member as usize].first_arc == arc {
                self.members[member as usize].first_arc = next;
            }
        }
        self.members[member as usize].num_arcs -= 1;
    }
    fn create_child_marker(&mut self, member: i32, child: i32, is_tree: bool, reversed: bool) -> i32 {
        let a = self.create_arc(member, reversed);
        self.arcs[a as usize].element = if is_tree { MARKER_ROW } else { MARKER_COL };
        self.arcs[a as usize].child_member = child;
        self.add_arc_to_member_arc_list(a, member);
        a
    }
    fn create_parent_marker(&mut self, member: i32, is_tree: bool, parent: i32, parent_marker: i32, reversed: bool) -> i32 {
        let a = self.create_arc(member, reversed);
        self.arcs[a as usize].element = if is_tree { MARKER_ROW } else { MARKER_COL };
        self.add_arc_to_member_arc_list(a, member);
        self.members[member as usize].parent = parent;
        self.members[member as usize].marker_of_parent = parent_marker;
        self.members[member as usize].marker_to_parent = a;
        a
    }
    /// (親から子への印, 子から親への印)
    fn create_marker_pair(&mut self, parent_member: i32, child_member: i32, parent_is_tree: bool, child_marker_reversed: bool, parent_marker_reversed: bool) -> (i32, i32) {
        let p2c = self.create_child_marker(parent_member, child_member, parent_is_tree, child_marker_reversed);
        let c2p = self.create_parent_marker(child_member, !parent_is_tree, parent_member, p2c, parent_marker_reversed);
        (p2c, c2p)
    }
    fn move_arc_to_new_member(&mut self, arc: i32, old_member: i32, new_member: i32) {
        self.remove_arc_from_member_arc_list(arc, old_member);
        self.add_arc_to_member_arc_list(arc, new_member);
        self.arcs[arc as usize].member = new_member;
        let child = self.arcs[arc as usize].child_member;
        if valid(child) {
            let cr = self.find_arc_child_member(arc);
            self.members[cr as usize].parent = new_member;
        }
        if self.members[old_member as usize].marker_to_parent == arc {
            let om = self.members[old_member as usize];
            self.members[new_member as usize].marker_to_parent = arc;
            self.members[new_member as usize].parent = om.parent;
            self.members[new_member as usize].marker_of_parent = om.marker_of_parent;
            self.arcs[om.marker_of_parent as usize].child_member = new_member;
        }
    }
    fn merge_member_arc_list(&mut self, into: i32, from: i32) {
        let first_into = self.get_first_member_arc(into);
        let first_from = self.get_first_member_arc(from);
        let last_into = self.get_previous_member_arc(first_into);
        let last_from = self.get_previous_member_arc(first_from);
        self.arcs[first_into as usize].ln.prev = last_from;
        self.arcs[last_into as usize].ln.next = first_from;
        self.arcs[first_from as usize].ln.prev = last_into;
        self.arcs[last_from as usize].ln.next = first_into;
        self.members[into as usize].num_arcs += self.members[from as usize].num_arcs;
        self.members[from as usize].num_arcs = 0;
        self.members[from as usize].first_arc = INV;
    }
    fn change_loop_to_parallel(&mut self, m: i32) {
        self.members[m as usize].ty = Mt::Parallel;
    }
    fn reorder_component(&mut self, new_root: i32) {
        if valid(self.members[new_root as usize].parent) {
            let mut member = self.find_member_parent(new_root);
            let mut new_parent = new_root;
            let mut new_marker_to_parent = self.members[new_root as usize].marker_of_parent;
            let mut marker_of_new_parent = self.members[new_root as usize].marker_to_parent;
            loop {
                let old_parent = self.find_member_parent(member);
                let old_marker_to_parent = self.members[member as usize].marker_to_parent;
                let old_marker_of_parent = self.members[member as usize].marker_of_parent;
                self.members[member as usize].marker_to_parent = new_marker_to_parent;
                self.members[member as usize].marker_of_parent = marker_of_new_parent;
                self.members[member as usize].parent = new_parent;
                self.arcs[marker_of_new_parent as usize].child_member = member;
                self.arcs[new_marker_to_parent as usize].child_member = INV;
                if valid(old_parent) {
                    new_parent = member;
                    member = old_parent;
                    new_marker_to_parent = old_marker_of_parent;
                    marker_of_new_parent = old_marker_to_parent;
                } else {
                    break;
                }
            }
            self.members[new_root as usize].parent = INV;
            self.members[new_root as usize].marker_to_parent = INV;
            self.members[new_root as usize].marker_of_parent = INV;
        }
    }
    fn merge_given_member_into_parent(&mut self, member: i32, parent: i32, parent_to_child: i32, child_to_parent: i32, head_to_head: bool) -> i32 {
        self.remove_arc_from_member_arc_list(parent_to_child, parent);
        self.remove_arc_from_member_arc_list(child_to_parent, member);
        let parent_nodes = [self.find_effective_arc_tail(parent_to_child), self.find_effective_arc_head(parent_to_child)];
        let child_nodes = [self.find_effective_arc_tail(child_to_parent), self.find_effective_arc_head(child_to_parent)];
        self.clear_arc_head_and_tail(parent_to_child);
        self.clear_arc_head_and_tail(child_to_parent);
        let first = child_nodes[if head_to_head { 0 } else { 1 }];
        let second = child_nodes[if head_to_head { 1 } else { 0 }];
        {
            let nn = self.merge_nodes(parent_nodes[0], first);
            let from = if nn == first { parent_nodes[0] } else { first };
            self.merge_node_arc_list(nn, from);
        }
        {
            let nn = self.merge_nodes(parent_nodes[1], second);
            let from = if nn == second { parent_nodes[1] } else { second };
            self.merge_node_arc_list(nn, from);
        }
        let new_member = self.merge_members(member, parent);
        let to_remove = if new_member == member { parent } else { member };
        self.merge_member_arc_list(new_member, to_remove);
        if to_remove == parent {
            self.update_member_parent_information(new_member, to_remove);
        }
        self.update_member_type(new_member, Mt::Rigid);
        new_member
    }
}

// ---- 列の追加の作業データ (MATREC の `MATRECNetworkColumnAddition`) ----

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Rt {
    Unassigned,
    Cycle,
    Merged,
    NotNetwork,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pt {
    IntoHead,
    IntoTail,
    OutHead,
    OutTail,
}
fn is_into(t: Pt) -> bool {
    matches!(t, Pt::IntoHead | Pt::IntoTail)
}
fn is_head(t: Pt) -> bool {
    matches!(t, Pt::IntoHead | Pt::OutHead)
}

#[derive(Clone, Copy)]
struct PathArc {
    arc: i32,
    arc_head: i32,
    arc_tail: i32,
    next_member: i32,
    next_overall: i32,
    reversed: bool,
}

#[derive(Clone, Copy)]
struct RedMem {
    member: i32,
    root_member: i32,
    depth: i32,
    ty: Rt,
    parent: i32,
    first_child: i32,
    num_children: i32,
    first_path_arc: i32,
    num_path_arcs: i32,
    reverse_arcs: bool,
    rigid_path_start: i32,
    rigid_path_end: i32,
    path_backwards: bool,
    num_propagated_children: i32,
    component_index: i32,
    path_type: Pt,
    next_path_member: i32,
    next_path_member_is_parent: bool,
    path_source_arc: i32,
    path_target_arc: i32,
}

impl Default for RedMem {
    fn default() -> Self {
        RedMem {
            member: INV,
            root_member: INV,
            depth: 0,
            ty: Rt::Unassigned,
            parent: INV,
            first_child: 0,
            num_children: 0,
            first_path_arc: INV,
            num_path_arcs: 0,
            reverse_arcs: false,
            rigid_path_start: INV,
            rigid_path_end: INV,
            path_backwards: false,
            num_propagated_children: 0,
            component_index: -1,
            path_type: Pt::IntoHead,
            next_path_member: INV,
            next_path_member_is_parent: false,
            path_source_arc: INV,
            path_target_arc: INV,
        }
    }
}

#[derive(Clone, Copy)]
struct RedComp {
    root_depth: i32,
    root: i32,
    path_end_members: [i32; 2],
    num_path_end_members: i32,
}

#[derive(Clone, Copy)]
struct MemberInfo {
    reduced_member: i32,
    root_depth_minimizer: i32,
}

#[derive(Default)]
struct ColAdd {
    remains_network: bool,
    reduced_members: Vec<RedMem>,
    reduced_components: Vec<RedComp>,
    member_information: Vec<MemberInfo>,
    children_storage: Vec<i32>,
    path_arcs: Vec<PathArc>,
    first_overall_path_arc: i32,
    node_in_path_degree: Vec<i32>,
    node_out_path_degree: Vec<i32>,
    arc_in_path: Vec<bool>,
    arc_in_path_reversed: Vec<bool>,
    new_col_index: i32,
    new_row_arcs: Vec<i32>,
    new_row_arc_reversed: Vec<bool>,
    decomposition_row_arcs: Vec<i32>,
    decomposition_arc_reversed: Vec<bool>,
    leaf_members: Vec<i32>,
}

/// 新しい列を置く場所 (C の `NewColInformation`)
#[derive(Clone, Copy)]
struct NewColInfo {
    member: i32,
    head: i32,
    tail: i32,
    representative: i32,
    reversed: bool,
}
fn empty_info() -> NewColInfo {
    NewColInfo { member: INV, head: INV, tail: INV, representative: INV, reversed: false }
}

/// 列を 1 本ずつ足してネットワーク行列かを確かめる (MATREC の列の追加)。
pub(crate) struct NetCols {
    dec: Dec,
    nc: ColAdd,
}

impl NetCols {
    pub(crate) fn new(nrows: usize, ncols: usize) -> NetCols {
        let mut nc = ColAdd::default();
        nc.first_overall_path_arc = INV;
        NetCols { dec: Dec::new(nrows, ncols), nc }
    }

    /// 列 `col` (行の番号と符号 (負なら真)) を足せるか確かめ、足せるなら足して真を返す。足せなければ偽 (分解は変えない)。
    pub(crate) fn add(&mut self, col: usize, rows: &[(usize, bool)]) -> bool {
        self.check(col as i32, rows);
        if !self.nc.remains_network {
            return false;
        }
        self.column_addition_add();
        true
    }

    // ---- 確かめ (MATRECNetworkColumnAdditionCheck) ----

    fn check(&mut self, column: i32, rows: &[(usize, bool)]) {
        self.nc.remains_network = true;
        self.cleanup_previous_iteration();
        self.update_col_information(column, rows);
        self.construct_reduced_decomposition();
        self.create_path_arcs();
        self.compute_leaf_members();
        self.propagate_cycles();
        if self.nc.remains_network {
            for i in 0..self.nc.reduced_components.len() {
                self.determine_component_types(i);
            }
        }
        self.clean_up_member_information();
    }

    fn cleanup_previous_iteration(&mut self) {
        let nc = &mut self.nc;
        let mut pa = nc.first_overall_path_arc;
        while valid(pa) {
            let p = nc.path_arcs[pa as usize];
            if valid(p.arc_head) {
                nc.node_in_path_degree[p.arc_head as usize] = 0;
            }
            if valid(p.arc_tail) {
                nc.node_out_path_degree[p.arc_tail as usize] = 0;
            }
            if (p.arc as usize) < nc.arc_in_path.len() {
                nc.arc_in_path[p.arc as usize] = false;
                nc.arc_in_path_reversed[p.arc as usize] = false;
            }
            pa = p.next_overall;
        }
        nc.first_overall_path_arc = INV;
        nc.path_arcs.clear();
    }

    fn update_col_information(&mut self, column: i32, rows: &[(usize, bool)]) {
        let nc = &mut self.nc;
        nc.new_col_index = column;
        nc.decomposition_row_arcs.clear();
        nc.decomposition_arc_reversed.clear();
        nc.new_row_arcs.clear();
        nc.new_row_arc_reversed.clear();
        for &(r, neg) in rows {
            let ra = self.dec.row_arcs[r];
            if valid(ra) {
                nc.decomposition_row_arcs.push(ra);
                nc.decomposition_arc_reversed.push(neg);
            } else {
                nc.new_row_arcs.push(r as i32);
                nc.new_row_arc_reversed.push(neg);
            }
        }
    }

    fn create_reduced_members_to_root(&mut self, first_member: i32) -> i32 {
        let mut callstack: Vec<i32> = vec![first_member];
        let mut call_depth: i32 = 0;
        while call_depth >= 0 {
            let member = callstack[call_depth as usize];
            let mut reduced_member = self.nc.member_information[member as usize].reduced_member;
            let reduced_valid = valid(reduced_member);
            if !reduced_valid {
                reduced_member = self.nc.reduced_members.len() as i32;
                self.nc.reduced_members.push(RedMem { member, ..Default::default() });
                self.nc.member_information[member as usize].reduced_member = reduced_member;
                let parent_member = self.dec.find_member_parent(member);
                if valid(parent_member) {
                    call_depth += 1;
                    if callstack.len() <= call_depth as usize {
                        callstack.push(parent_member);
                    } else {
                        callstack[call_depth as usize] = parent_member;
                    }
                    continue;
                } else {
                    let ci = self.nc.reduced_components.len() as i32;
                    let rm = &mut self.nc.reduced_members[reduced_member as usize];
                    rm.parent = INV;
                    rm.depth = 0;
                    rm.root_member = member;
                    rm.component_index = ci;
                    self.nc.reduced_components.push(RedComp { root_depth: 0, root: reduced_member, path_end_members: [INV, INV], num_path_end_members: 0 });
                }
            }
            if reduced_valid {
                let rmd = self.nc.reduced_members[reduced_member as usize];
                let dm = self.nc.member_information[rmd.root_member as usize].root_depth_minimizer;
                if !valid(dm) || rmd.depth < self.nc.reduced_members[dm as usize].depth {
                    self.nc.member_information[rmd.root_member as usize].root_depth_minimizer = reduced_member;
                }
            }
            loop {
                call_depth -= 1;
                if call_depth < 0 {
                    break;
                }
                let parent_member = callstack[(call_depth + 1) as usize];
                let parent_rm = self.nc.member_information[parent_member as usize].reduced_member;
                let current_member = callstack[call_depth as usize];
                let current_rm = self.nc.member_information[current_member as usize].reduced_member;
                let p = self.nc.reduced_members[parent_rm as usize];
                let r = &mut self.nc.reduced_members[current_rm as usize];
                r.parent = parent_rm;
                r.depth = p.depth + 1;
                r.root_member = p.root_member;
                r.component_index = p.component_index;
                self.nc.reduced_members[parent_rm as usize].num_children += 1;
            }
        }
        self.nc.member_information[callstack[0] as usize].reduced_member
    }

    fn construct_reduced_decomposition(&mut self) {
        self.nc.reduced_components.clear();
        self.nc.reduced_members.clear();
        if self.nc.decomposition_row_arcs.is_empty() {
            return;
        }
        let nm = self.dec.members.len();
        if self.nc.member_information.len() < nm {
            self.nc.member_information.resize(nm, MemberInfo { reduced_member: INV, root_depth_minimizer: INV });
        }
        for i in 0..self.nc.decomposition_row_arcs.len() {
            let arc = self.nc.decomposition_row_arcs[i];
            let arc_member = self.dec.find_arc_member(arc);
            let rm = self.create_reduced_members_to_root(arc_member);
            let root_member = self.nc.reduced_members[rm as usize].root_member;
            if !valid(self.nc.member_information[root_member as usize].root_depth_minimizer) {
                self.nc.member_information[root_member as usize].root_depth_minimizer = rm;
            }
        }
        for i in 0..self.nc.reduced_components.len() {
            let root = self.nc.reduced_components[i].root;
            let root_member = self.nc.reduced_members[root as usize].member;
            let minimizer = self.nc.member_information[root_member as usize].root_depth_minimizer;
            self.nc.reduced_components[i].root_depth = self.nc.reduced_members[minimizer as usize].depth;
            self.nc.reduced_components[i].root = minimizer;
            self.nc.reduced_members[minimizer as usize].parent = INV;
        }
        let mut num_total_children = 0;
        for i in 0..self.nc.reduced_members.len() {
            let rmd = self.nc.reduced_members[i];
            let minimizer = self.nc.member_information[rmd.root_member as usize].root_depth_minimizer;
            if rmd.depth >= self.nc.reduced_members[minimizer as usize].depth {
                self.nc.reduced_members[i].first_child = num_total_children;
                num_total_children += rmd.num_children;
                self.nc.reduced_members[i].num_children = 0;
            }
        }
        self.nc.children_storage.clear();
        self.nc.children_storage.resize(num_total_children as usize, INV);
        for rm in 0..self.nc.reduced_members.len() {
            let rmd = self.nc.reduced_members[rm];
            let minimizer = self.nc.member_information[rmd.root_member as usize].root_depth_minimizer;
            if rmd.depth <= self.nc.reduced_members[minimizer as usize].depth {
                continue;
            }
            let parent_member = self.dec.find_member_parent(rmd.member);
            let parent_rm = if valid(parent_member) { self.nc.member_information[parent_member as usize].reduced_member } else { INV };
            if valid(parent_rm) {
                let p = self.nc.reduced_members[parent_rm as usize];
                self.nc.children_storage[(p.first_child + p.num_children) as usize] = rm as i32;
                self.nc.reduced_members[parent_rm as usize].num_children += 1;
            }
        }
        for i in 0..self.nc.reduced_members.len() {
            let root_member = self.nc.reduced_members[i].root_member;
            self.nc.member_information[root_member as usize].root_depth_minimizer = INV;
        }
    }

    fn clean_up_member_information(&mut self) {
        for i in 0..self.nc.reduced_members.len() {
            let m = self.nc.reduced_members[i].member;
            self.nc.member_information[m as usize].reduced_member = INV;
        }
    }

    fn create_path_arc(&mut self, arc: i32, reduced_member: i32, reversed: bool) {
        let path_arc = self.nc.path_arcs.len() as i32;
        let next_member = self.nc.reduced_members[reduced_member as usize].first_path_arc;
        self.nc.reduced_members[reduced_member as usize].first_path_arc = path_arc;
        self.nc.reduced_members[reduced_member as usize].num_path_arcs += 1;
        let next_overall = self.nc.first_overall_path_arc;
        self.nc.first_overall_path_arc = path_arc;
        if self.nc.arc_in_path.len() <= arc as usize {
            let n = self.dec.arcs.len().max(arc as usize + 1);
            self.nc.arc_in_path.resize(n, false);
            self.nc.arc_in_path_reversed.resize(n, false);
        }
        self.nc.arc_in_path[arc as usize] = true;
        self.nc.arc_in_path_reversed[arc as usize] = reversed;
        let member = self.nc.reduced_members[reduced_member as usize].member;
        let (mut head, mut tail) = (INV, INV);
        if self.dec.get_member_type(member) == Mt::Rigid {
            head = self.dec.find_effective_arc_head(arc);
            tail = self.dec.find_effective_arc_tail(arc);
            if reversed {
                std::mem::swap(&mut head, &mut tail);
            }
            let nn = self.dec.nodes.len();
            if self.nc.node_in_path_degree.len() < nn {
                self.nc.node_in_path_degree.resize(nn, 0);
                self.nc.node_out_path_degree.resize(nn, 0);
            }
            self.nc.node_in_path_degree[head as usize] += 1;
            self.nc.node_out_path_degree[tail as usize] += 1;
        }
        self.nc.path_arcs.push(PathArc { arc, arc_head: head, arc_tail: tail, next_member, next_overall, reversed });
    }

    fn create_path_arcs(&mut self) {
        for i in 0..self.nc.decomposition_row_arcs.len() {
            let arc = self.nc.decomposition_row_arcs[i];
            let member = self.dec.find_arc_member(arc);
            let rm = self.nc.member_information[member as usize].reduced_member;
            let rev = self.nc.decomposition_arc_reversed[i];
            self.create_path_arc(arc, rm, rev);
        }
    }

    fn compute_leaf_members(&mut self) {
        self.nc.leaf_members.clear();
        for rm in 0..self.nc.reduced_members.len() {
            if self.nc.reduced_members[rm].num_children == 0 {
                self.nc.leaf_members.push(rm as i32);
            }
        }
    }

    fn determine_rigid_path(&mut self, rm: i32) {
        let mut is_valid_path = true;
        self.nc.reduced_members[rm as usize].rigid_path_start = INV;
        self.nc.reduced_members[rm as usize].rigid_path_end = INV;
        let mut pa = self.nc.reduced_members[rm as usize].first_path_arc;
        while valid(pa) {
            let p = self.nc.path_arcs[pa as usize];
            let (head, tail) = (p.arc_head, p.arc_tail);
            if self.nc.node_in_path_degree[head as usize] > 1 || self.nc.node_out_path_degree[tail as usize] > 1 {
                is_valid_path = false;
                break;
            }
            if self.nc.node_out_path_degree[head as usize] == 0 {
                if valid(self.nc.reduced_members[rm as usize].rigid_path_end) {
                    is_valid_path = false;
                    break;
                }
                self.nc.reduced_members[rm as usize].rigid_path_end = head;
            }
            if self.nc.node_in_path_degree[tail as usize] == 0 {
                if valid(self.nc.reduced_members[rm as usize].rigid_path_start) {
                    is_valid_path = false;
                    break;
                }
                self.nc.reduced_members[rm as usize].rigid_path_start = tail;
            }
            pa = p.next_member;
        }
        // 始点と終点が両方見つからない (閉じた有向閉路) なら道ではない (C では assert のみ)
        if is_valid_path && (!valid(self.nc.reduced_members[rm as usize].rigid_path_start) || !valid(self.nc.reduced_members[rm as usize].rigid_path_end)) {
            is_valid_path = false;
        }
        if !is_valid_path {
            let r = &mut self.nc.reduced_members[rm as usize];
            r.rigid_path_start = INV;
            r.rigid_path_end = INV;
            r.ty = Rt::NotNetwork;
            self.nc.remains_network = false;
        }
    }

    fn determine_single_rigid_type(&mut self, rm: i32) {
        self.determine_rigid_path(rm);
        if self.nc.reduced_members[rm as usize].ty != Rt::NotNetwork {
            self.nc.reduced_members[rm as usize].ty = Rt::Merged;
        }
    }

    /// 直列 (と輪) の部分の道の弧が同じ向きに並ぶか。(良いか, 前向きか, 数)
    fn series_path_direction(&self, rm: i32) -> (bool, bool, i32) {
        let mut counted = 0;
        let mut good = true;
        let mut passes_forwards = true;
        let mut pa = self.nc.reduced_members[rm as usize].first_path_arc;
        while valid(pa) {
            let p = self.nc.path_arcs[pa as usize];
            let f = p.reversed != self.dec.arc_is_reversed_non_rigid(p.arc);
            if counted == 0 {
                passes_forwards = f;
            } else if f != passes_forwards {
                good = false;
                break;
            }
            counted += 1;
            pa = p.next_member;
        }
        (good, passes_forwards, counted)
    }

    fn determine_single_component_type(&mut self, rm: i32) {
        let r = self.nc.reduced_members[rm as usize];
        let mut num_non_propagated_adjacent = r.num_children - r.num_propagated_children;
        if valid(r.parent) && self.nc.reduced_members[r.parent as usize].ty != Rt::Cycle {
            num_non_propagated_adjacent += 1;
        }
        if num_non_propagated_adjacent > 2 {
            self.nc.reduced_members[rm as usize].ty = Rt::NotNetwork;
            self.nc.remains_network = false;
            return;
        }
        let member = self.dec.find_member(r.member);
        match self.dec.get_member_type(member) {
            Mt::Rigid => self.determine_single_rigid_type(rm),
            Mt::Parallel => {
                let p = self.nc.path_arcs[r.first_path_arc as usize];
                let path_forwards = p.reversed == self.dec.arc_is_reversed_non_rigid(p.arc);
                let rr = &mut self.nc.reduced_members[rm as usize];
                rr.path_backwards = !path_forwards;
                rr.ty = Rt::Cycle;
            }
            Mt::Series | Mt::Loop => {
                let (good, passes_forwards, counted) = self.series_path_direction(rm);
                if !good {
                    self.nc.reduced_members[rm as usize].ty = Rt::NotNetwork;
                    self.nc.remains_network = false;
                    return;
                }
                let m = self.dec.find_member(r.member);
                let n = self.dec.get_num_member_arcs(m);
                let rr = &mut self.nc.reduced_members[rm as usize];
                rr.path_backwards = !passes_forwards;
                rr.ty = if counted == n - 1 { Rt::Cycle } else { Rt::Merged };
            }
            Mt::Unassigned => panic!("unassigned member"),
        }
    }

    fn determine_path_series_type(&mut self, rm: i32, previous_type: Pt, source: i32, target: i32) {
        let (good, passes_forwards, counted) = self.series_path_direction(rm);
        if !good {
            self.nc.reduced_members[rm as usize].ty = Rt::NotNetwork;
            self.nc.remains_network = false;
            return;
        }
        if !valid(source) {
            let r = self.nc.reduced_members[rm as usize];
            let fp = self.nc.path_arcs[r.first_path_arc as usize];
            let first_reversed = self.dec.arc_is_reversed_non_rigid(fp.arc);
            let target_reversed = self.dec.arc_is_reversed_non_rigid(target);
            let reverse_path = fp.reversed;
            let rr = &mut self.nc.reduced_members[rm as usize];
            rr.path_type = if (first_reversed == target_reversed) == reverse_path { Pt::IntoHead } else { Pt::OutHead };
            rr.path_backwards = !passes_forwards;
            return;
        }
        let source_reversed = self.dec.arc_is_reversed_non_rigid(source);
        if counted > 0 {
            let into_head_or_out_tail = is_into(previous_type) == is_head(previous_type);
            let is_good = into_head_or_out_tail == (source_reversed == passes_forwards);
            if !is_good {
                self.nc.reduced_members[rm as usize].ty = Rt::NotNetwork;
                self.nc.remains_network = false;
                return;
            }
        }
        self.nc.reduced_members[rm as usize].path_backwards = !passes_forwards;
        if valid(target) {
            let target_reversed = self.dec.arc_is_reversed_non_rigid(target);
            let same = source_reversed == target_reversed;
            let cur = match previous_type {
                Pt::IntoHead => {
                    if same {
                        Pt::IntoTail
                    } else {
                        Pt::IntoHead
                    }
                }
                Pt::IntoTail => {
                    if same {
                        Pt::IntoHead
                    } else {
                        Pt::IntoTail
                    }
                }
                Pt::OutHead => {
                    if same {
                        Pt::OutTail
                    } else {
                        Pt::OutHead
                    }
                }
                Pt::OutTail => {
                    if same {
                        Pt::OutHead
                    } else {
                        Pt::OutTail
                    }
                }
            };
            self.nc.reduced_members[rm as usize].path_type = cur;
            return;
        }
        self.nc.reduced_members[rm as usize].path_type = if is_into(previous_type) { Pt::IntoHead } else { Pt::OutHead };
    }

    fn determine_path_parallel_type(&mut self, rm: i32, previous_type: Pt, source: i32, target: i32) {
        let source_reversed = self.dec.arc_is_reversed_non_rigid(source);
        let target_reversed = self.dec.arc_is_reversed_non_rigid(target);
        let in_same_direction = source_reversed == target_reversed;
        let pa = self.nc.reduced_members[rm as usize].first_path_arc;
        let mut arc_present = false;
        if valid(pa) {
            let p = self.nc.path_arcs[pa as usize];
            let path_arc_reversed = p.reversed != self.dec.arc_is_reversed_non_rigid(p.arc);
            let into_head_or_out_tail = is_into(previous_type) == is_head(previous_type);
            let good = into_head_or_out_tail == (path_arc_reversed != source_reversed);
            if !good {
                self.nc.reduced_members[rm as usize].ty = Rt::NotNetwork;
                self.nc.remains_network = false;
                return;
            }
            arc_present = true;
        }
        let swap = arc_present != in_same_direction;
        let cur = match previous_type {
            Pt::IntoHead => {
                if swap {
                    Pt::IntoHead
                } else {
                    Pt::IntoTail
                }
            }
            Pt::IntoTail => {
                if swap {
                    Pt::IntoTail
                } else {
                    Pt::IntoHead
                }
            }
            Pt::OutHead => {
                if swap {
                    Pt::OutHead
                } else {
                    Pt::OutTail
                }
            }
            Pt::OutTail => {
                if swap {
                    Pt::OutTail
                } else {
                    Pt::OutHead
                }
            }
        };
        self.nc.reduced_members[rm as usize].path_type = cur;
    }

    fn not_network(&mut self, rm: i32) {
        self.nc.reduced_members[rm as usize].ty = Rt::NotNetwork;
        self.nc.remains_network = false;
    }

    fn determine_path_rigid_type(&mut self, rm: i32, previous_type: Pt, source: i32, target: i32) {
        if !valid(self.nc.reduced_members[rm as usize].first_path_arc) {
            let source_tail = self.dec.find_effective_arc_tail(source);
            let source_head = self.dec.find_effective_arc_head(source);
            let target_tail = self.dec.find_effective_arc_tail(target);
            let target_head = self.dec.find_effective_arc_head(target);
            let shth = source_head == target_head;
            let stth = source_tail == target_head;
            let shtt = source_head == target_tail;
            let sttt = source_tail == target_tail;
            if !(shth || stth || shtt || sttt) {
                self.not_network(rm);
                return;
            }
            let is_source_head = shth || shtt;
            let is_target_tail = shtt || sttt;
            let rr = &mut self.nc.reduced_members[rm as usize];
            if is_head(previous_type) == is_source_head {
                rr.reverse_arcs = false;
                rr.path_type = if is_into(previous_type) {
                    if is_target_tail {
                        Pt::IntoTail
                    } else {
                        Pt::IntoHead
                    }
                } else if is_target_tail {
                    Pt::OutTail
                } else {
                    Pt::OutHead
                };
            } else {
                rr.reverse_arcs = true;
                rr.path_type = if is_into(previous_type) {
                    if is_target_tail {
                        Pt::IntoHead
                    } else {
                        Pt::IntoTail
                    }
                } else if is_target_tail {
                    Pt::OutHead
                } else {
                    Pt::OutTail
                };
            }
            return;
        }
        self.determine_rigid_path(rm);
        if self.nc.reduced_members[rm as usize].ty == Rt::NotNetwork {
            return;
        }
        let (start, end) = (self.nc.reduced_members[rm as usize].rigid_path_start, self.nc.reduced_members[rm as usize].rigid_path_end);
        if !valid(source) {
            let target_tail = self.dec.find_effective_arc_tail(target);
            let target_head = self.dec.find_effective_arc_head(target);
            self.nc.reduced_members[rm as usize].reverse_arcs = false;
            let t = if end == target_head {
                Some(Pt::IntoHead)
            } else if end == target_tail {
                Some(Pt::IntoTail)
            } else if start == target_head {
                Some(Pt::OutHead)
            } else if start == target_tail {
                Some(Pt::OutTail)
            } else {
                None
            };
            match t {
                Some(t) => self.nc.reduced_members[rm as usize].path_type = t,
                None => self.not_network(rm),
            }
            return;
        }
        let source_tail = self.dec.find_effective_arc_tail(source);
        let source_head = self.dec.find_effective_arc_head(source);
        let starts_at_head = source_head == start;
        let ends_at_tail = source_tail == end;
        let starts_at_tail = source_tail == start;
        let ends_at_head = source_head == end;
        let pin = is_into(previous_type);
        let into_head_or_out_tail = pin == is_head(previous_type);
        if into_head_or_out_tail {
            if !starts_at_head && !ends_at_tail {
                self.not_network(rm);
                return;
            }
        } else if !starts_at_tail && !ends_at_head {
            self.not_network(rm);
            return;
        }
        if valid(target) {
            let target_tail = self.dec.find_effective_arc_tail(target);
            let target_head = self.dec.find_effective_arc_head(target);
            let s_th = start == target_head;
            let s_tt = start == target_tail;
            let e_th = end == target_head;
            let e_tt = end == target_tail;
            if !(s_th || s_tt || e_th || e_tt) {
                self.not_network(rm);
                return;
            }
            let out_reverse;
            let mut out_head = false;
            if into_head_or_out_tail {
                if starts_at_head && ends_at_tail {
                    out_reverse = (s_th || s_tt) == pin;
                } else if starts_at_head {
                    out_reverse = !pin;
                } else {
                    out_reverse = pin;
                }
            } else if starts_at_tail && ends_at_head {
                out_reverse = (s_th || s_tt) == pin;
            } else if starts_at_tail {
                out_reverse = !pin;
            } else {
                out_reverse = pin;
            }
            let mut is_bad = false;
            if into_head_or_out_tail {
                if starts_at_head && ends_at_tail {
                    out_head = (s_tt || e_th) == pin;
                } else if starts_at_head {
                    if e_th {
                        out_head = pin;
                    } else if e_tt {
                        out_head = !pin;
                    } else {
                        is_bad = true;
                    }
                } else if s_tt {
                    out_head = pin;
                } else if s_th {
                    out_head = !pin;
                } else {
                    is_bad = true;
                }
            } else if starts_at_tail && ends_at_head {
                out_head = (s_tt || e_th) == pin;
            } else if starts_at_tail {
                if e_th {
                    out_head = pin;
                } else if e_tt {
                    out_head = !pin;
                } else {
                    is_bad = true;
                }
            } else if s_tt {
                out_head = pin;
            } else if s_th {
                out_head = !pin;
            } else {
                is_bad = true;
            }
            if is_bad {
                self.not_network(rm);
                return;
            }
            let rr = &mut self.nc.reduced_members[rm as usize];
            rr.reverse_arcs = out_reverse;
            rr.path_type = if pin {
                if out_head {
                    Pt::IntoHead
                } else {
                    Pt::IntoTail
                }
            } else if out_head {
                Pt::OutHead
            } else {
                Pt::OutTail
            };
            return;
        }
        let rr = &mut self.nc.reduced_members[rm as usize];
        rr.reverse_arcs = if into_head_or_out_tail { starts_at_head != pin } else { starts_at_tail != pin };
        rr.path_type = if pin { Pt::IntoHead } else { Pt::OutHead };
    }

    fn determine_path_member_type(&mut self, rm: i32, member: i32, previous_type: Pt, source: i32, target: i32) {
        self.nc.reduced_members[rm as usize].path_source_arc = source;
        self.nc.reduced_members[rm as usize].path_target_arc = target;
        match self.dec.get_member_type(member) {
            Mt::Rigid => self.determine_path_rigid_type(rm, previous_type, source, target),
            Mt::Parallel => self.determine_path_parallel_type(rm, previous_type, source, target),
            Mt::Series => self.determine_path_series_type(rm, previous_type, source, target),
            Mt::Loop | Mt::Unassigned => {
                self.nc.remains_network = false;
            }
        }
    }

    fn determine_path_types(&mut self, ci: usize) {
        let comp = self.nc.reduced_components[ci];
        let reduced_start = comp.path_end_members[0];
        let mut to_previous = INV;
        let mut previous_type = Pt::IntoHead;
        let mut member = self.nc.reduced_members[reduced_start as usize].member;
        let mut reduced_member = reduced_start;
        let mut previous_reduced_member = reduced_start;
        while reduced_member != comp.root {
            let to_next = self.dec.marker_to_parent(member);
            self.determine_path_member_type(reduced_member, member, previous_type, to_previous, to_next);
            if !self.nc.remains_network {
                return;
            }
            previous_type = self.nc.reduced_members[reduced_member as usize].path_type;
            to_previous = self.dec.marker_of_parent(member);
            let m = self.nc.reduced_members[reduced_member as usize].member;
            member = self.dec.find_member_parent(m);
            previous_reduced_member = reduced_member;
            reduced_member = self.nc.member_information[member as usize].reduced_member;
            self.nc.reduced_members[previous_reduced_member as usize].next_path_member = reduced_member;
            self.nc.reduced_members[previous_reduced_member as usize].next_path_member_is_parent = true;
        }
        while reduced_member != comp.path_end_members[1] {
            let r = self.nc.reduced_members[reduced_member as usize];
            let mut child = INV;
            for i in r.first_child..r.first_child + r.num_children {
                let c = self.nc.children_storage[i as usize];
                if self.nc.reduced_members[c as usize].ty != Rt::Cycle && c != previous_reduced_member {
                    child = c;
                    break;
                }
            }
            let child_member = self.nc.reduced_members[child as usize].member;
            let to_next = self.dec.marker_of_parent(child_member);
            self.determine_path_member_type(reduced_member, member, previous_type, to_previous, to_next);
            if !self.nc.remains_network {
                return;
            }
            previous_type = self.nc.reduced_members[reduced_member as usize].path_type;
            to_previous = self.dec.marker_to_parent(child_member);
            member = child_member;
            previous_reduced_member = reduced_member;
            reduced_member = child;
            self.nc.reduced_members[previous_reduced_member as usize].next_path_member = reduced_member;
            self.nc.reduced_members[previous_reduced_member as usize].next_path_member_is_parent = false;
        }
        self.determine_path_member_type(reduced_member, member, previous_type, to_previous, INV);
        self.nc.reduced_members[reduced_member as usize].next_path_member = INV;
    }

    fn check_rigid_leaf(&mut self, leaf: i32, to_parent: i32, parent: i32, to_child: i32) {
        self.determine_rigid_path(leaf);
        if self.nc.reduced_members[leaf as usize].ty == Rt::NotNetwork {
            return;
        }
        let target_head = self.dec.find_effective_arc_head(to_parent);
        let target_tail = self.dec.find_effective_arc_tail(to_parent);
        let l = self.nc.reduced_members[leaf as usize];
        let matches = l.rigid_path_start == target_tail && l.rigid_path_end == target_head;
        let opposite = l.rigid_path_start == target_head && l.rigid_path_end == target_tail;
        if matches || opposite {
            self.nc.reduced_members[leaf as usize].ty = Rt::Cycle;
            self.create_path_arc(to_child, parent, opposite);
            return;
        }
        self.nc.reduced_members[leaf as usize].ty = Rt::Merged;
    }

    fn check_leaf(&mut self, leaf: i32, to_parent: i32, parent: i32, to_child: i32) -> Rt {
        let member = self.nc.reduced_members[leaf as usize].member;
        match self.dec.get_member_type(member) {
            Mt::Rigid => self.check_rigid_leaf(leaf, to_parent, parent, to_child),
            Mt::Parallel => {
                self.nc.reduced_members[leaf as usize].ty = Rt::Cycle;
                let p = self.nc.path_arcs[self.nc.reduced_members[leaf as usize].first_path_arc as usize];
                let path_arc_reversed = p.reversed;
                let arc_rev = self.dec.arc_is_reversed_non_rigid(p.arc);
                let parent_reversed = self.dec.arc_is_reversed_non_rigid(to_parent);
                self.create_path_arc(to_child, parent, (arc_rev == parent_reversed) == path_arc_reversed);
            }
            Mt::Series | Mt::Loop => {
                let (good, passes_forwards, counted) = self.series_path_direction(leaf);
                if !good {
                    self.not_network(leaf);
                    return Rt::NotNetwork;
                }
                self.nc.reduced_members[leaf as usize].path_backwards = !passes_forwards;
                let m = self.dec.find_member(member);
                if counted == self.dec.get_num_member_arcs(m) - 1 {
                    self.nc.reduced_members[leaf as usize].ty = Rt::Cycle;
                    let p = self.nc.path_arcs[self.nc.reduced_members[leaf as usize].first_path_arc as usize];
                    let first_arc_reversed = self.dec.arc_is_reversed_non_rigid(p.arc);
                    let first_in_path_reverse = p.reversed;
                    let parent_reversed = self.dec.arc_is_reversed_non_rigid(to_parent);
                    self.create_path_arc(to_child, parent, (parent_reversed == first_arc_reversed) != first_in_path_reverse);
                } else {
                    self.nc.reduced_members[leaf as usize].ty = Rt::Merged;
                }
            }
            Mt::Unassigned => panic!("unassigned member"),
        }
        self.nc.reduced_members[leaf as usize].ty
    }

    fn add_path_end(&mut self, component: usize, rm: i32) -> bool {
        let c = &mut self.nc.reduced_components[component];
        if c.num_path_end_members >= 2 {
            self.nc.remains_network = false;
            return false;
        }
        c.path_end_members[c.num_path_end_members as usize] = rm;
        c.num_path_end_members += 1;
        true
    }

    fn propagate_cycles(&mut self) {
        let mut leaf_idx = 0;
        while leaf_idx != self.nc.leaf_members.len() {
            let leaf = self.nc.leaf_members[leaf_idx];
            let next = self.nc.reduced_members[leaf as usize].parent;
            let lm = self.nc.reduced_members[leaf as usize].member;
            let parent_marker = self.dec.marker_to_parent(lm);
            if valid(next) && !self.dec.arc_is_tree(parent_marker) {
                let mop = self.dec.marker_of_parent(lm);
                let ty = self.check_leaf(leaf, parent_marker, next, mop);
                if ty == Rt::Cycle {
                    self.nc.reduced_members[next as usize].num_propagated_children += 1;
                    let n = self.nc.reduced_members[next as usize];
                    if n.num_propagated_children == n.num_children {
                        self.nc.leaf_members[leaf_idx] = next;
                    } else {
                        leaf_idx += 1;
                    }
                } else if ty == Rt::NotNetwork {
                    return;
                } else {
                    leaf_idx += 1;
                    let comp = self.nc.reduced_members[leaf as usize].component_index as usize;
                    if !self.add_path_end(comp, leaf) {
                        return;
                    }
                }
            } else {
                leaf_idx += 1;
                let comp = self.nc.reduced_members[leaf as usize].component_index as usize;
                if !self.add_path_end(comp, leaf) {
                    return;
                }
            }
        }
        for j in 0..self.nc.reduced_components.len() {
            let mut root = self.nc.reduced_components[j].root;
            loop {
                let r = self.nc.reduced_members[root as usize];
                if r.num_propagated_children != r.num_children - 1 {
                    break;
                }
                let mut child = INV;
                let mut marker_to_child = INV;
                for i in r.first_child..r.first_child + r.num_children {
                    let c = self.nc.children_storage[i as usize];
                    if self.nc.reduced_members[c as usize].ty != Rt::Cycle {
                        child = c;
                        marker_to_child = self.dec.marker_of_parent(self.nc.reduced_members[c as usize].member);
                        break;
                    }
                }
                if !self.dec.arc_is_tree(marker_to_child) {
                    let cm = self.nc.reduced_members[child as usize].member;
                    let mtp = self.dec.marker_to_parent(cm);
                    let ty = self.check_leaf(root, marker_to_child, child, mtp);
                    if ty == Rt::Cycle {
                        root = child;
                        continue;
                    } else if ty == Rt::NotNetwork {
                        return;
                    }
                }
                let comp = self.nc.reduced_members[root as usize].component_index as usize;
                let c = self.nc.reduced_components[comp];
                let root_present = (0..c.num_path_end_members as usize).any(|i| c.path_end_members[i] == root);
                if !root_present && !self.add_path_end(comp, root) {
                    return;
                }
                break;
            }
            self.nc.reduced_components[j].root = root;
            self.nc.reduced_members[root as usize].parent = INV;
        }
    }

    fn determine_component_types(&mut self, ci: usize) {
        let c = self.nc.reduced_components[ci];
        if c.num_path_end_members == 1 {
            self.determine_single_component_type(c.root);
        } else {
            self.determine_path_types(ci);
        }
    }

    // ---- 足す (MATRECNetworkColumnAdditionAdd) ----

    fn split_parallel(&mut self, parallel: i32, arc1: i32, arc2: i32) -> i32 {
        let d = &mut self.dec;
        let child_contains_tree = d.arc_is_tree(arc1) || d.arc_is_tree(arc2);
        let to_parent = d.marker_to_parent(parallel);
        let parent_moved = to_parent == arc1 || to_parent == arc2;
        let child = d.create_member(Mt::Parallel);
        d.move_arc_to_new_member(arc1, parallel, child);
        d.move_arc_to_new_member(arc2, parallel, child);
        if parent_moved {
            d.create_marker_pair(child, parallel, !child_contains_tree, false, false);
        } else {
            d.create_marker_pair(parallel, child, child_contains_tree, false, false);
        }
        child
    }

    fn split_series(&mut self, rm: i32, member: i32, info: &mut NewColInfo) {
        let r = self.nc.reduced_members[rm as usize];
        let create_path_series = r.num_path_arcs > 1;
        let convert_original = r.num_path_arcs == self.dec.get_num_member_arcs(member) - 1;
        if !create_path_series && !convert_original {
            let parallel = self.dec.create_member(Mt::Parallel);
            let marked = self.nc.path_arcs[r.first_path_arc as usize].arc;
            self.dec.move_arc_to_new_member(marked, member, parallel);
            let reversed = self.dec.arc_is_reversed_non_rigid(marked);
            self.dec.create_marker_pair(member, parallel, true, reversed, reversed);
            info.reversed = if reversed == r.path_backwards { !reversed } else { reversed };
            info.member = parallel;
            return;
        }
        if !create_path_series && convert_original {
            self.dec.change_loop_to_parallel(member);
            let marked = self.nc.path_arcs[r.first_path_arc as usize].arc;
            self.dec.arc_flip_reversed(marked);
            info.reversed = r.path_backwards;
            info.member = member;
            return;
        }
        if create_path_series && !convert_original {
            let path_member = self.dec.create_member(Mt::Series);
            let mut pa = r.first_path_arc;
            let mut parent_moved = false;
            while valid(pa) {
                let arc = self.nc.path_arcs[pa as usize].arc;
                pa = self.nc.path_arcs[pa as usize].next_member;
                if arc == self.dec.marker_to_parent(member) {
                    parent_moved = true;
                }
                self.dec.move_arc_to_new_member(arc, member, path_member);
            }
            let loop_member = self.dec.create_member(Mt::Parallel);
            if !parent_moved {
                self.dec.create_marker_pair(member, loop_member, true, false, false);
                self.dec.create_marker_pair(loop_member, path_member, true, false, true);
            } else {
                self.dec.create_marker_pair(path_member, loop_member, false, false, true);
                self.dec.create_marker_pair(loop_member, member, false, false, false);
            }
            info.reversed = !r.path_backwards;
            info.member = loop_member;
            return;
        }
        // createPathSeries && convertOriginal
        {
            let mut adjacent_member = INV;
            let mut adjacent_marker = INV;
            let mut member_marker = INV;
            let first_arc = self.dec.get_first_member_arc(r.member);
            let mut arc = first_arc;
            loop {
                let in_path = self.nc.arc_in_path.get(arc as usize).copied().unwrap_or(false);
                if !in_path {
                    if arc == self.dec.marker_to_parent(r.member) {
                        adjacent_member = self.dec.find_member_parent(r.member);
                        adjacent_marker = self.dec.marker_of_parent(r.member);
                        member_marker = arc;
                    } else if self.dec.arc_is_marker(arc) {
                        adjacent_member = self.dec.find_arc_child_member(arc);
                        adjacent_marker = self.dec.marker_to_parent(adjacent_member);
                        member_marker = arc;
                    }
                    break;
                }
                arc = self.dec.get_next_member_arc(arc);
                if arc == first_arc {
                    break;
                }
            }
            if valid(adjacent_member) && self.dec.get_member_type(adjacent_member) == Mt::Parallel {
                let same = self.dec.arc_is_reversed_non_rigid(adjacent_marker) == self.dec.arc_is_reversed_non_rigid(member_marker);
                info.reversed = r.path_backwards == same;
                info.member = adjacent_member;
                return;
            }
        }
        let path_member = self.dec.create_member(Mt::Series);
        let mut pa = r.first_path_arc;
        let mut parent_moved = false;
        while valid(pa) {
            let arc = self.nc.path_arcs[pa as usize].arc;
            pa = self.nc.path_arcs[pa as usize].next_member;
            if arc == self.dec.marker_to_parent(member) {
                parent_moved = true;
            }
            self.dec.move_arc_to_new_member(arc, member, path_member);
        }
        if parent_moved {
            self.dec.create_marker_pair(path_member, member, false, false, false);
        } else {
            self.dec.create_marker_pair(member, path_member, true, false, false);
        }
        self.dec.change_loop_to_parallel(member);
        info.reversed = r.path_backwards;
        info.member = member;
    }

    /// (道の代表, 道でない代表)
    fn split_series_merging(&mut self, rm: i32, member: i32, exc1: i32, exc2: i32) -> (i32, i32) {
        let r = self.nc.reduced_members[rm as usize];
        let num_exc = (if valid(exc1) { 1 } else { 0 }) + (if valid(exc2) { 1 } else { 0 });
        let num_non_path = self.dec.get_num_member_arcs(member) - r.num_path_arcs - num_exc;
        let create_path_series = r.num_path_arcs > 1;
        let create_non_path_series = num_non_path > 1;
        let path_rep;
        if create_path_series {
            let path_member = self.dec.create_member(Mt::Series);
            let mut pa = r.first_path_arc;
            let mut parent_moved = false;
            while valid(pa) {
                let arc = self.nc.path_arcs[pa as usize].arc;
                pa = self.nc.path_arcs[pa as usize].next_member;
                parent_moved = parent_moved || self.dec.marker_to_parent(member) == arc;
                self.dec.move_arc_to_new_member(arc, member, path_member);
            }
            let (in_old_rev, in_new_rev) = (true, false);
            if parent_moved {
                let (_ignored, c2p) = self.dec.create_marker_pair(path_member, member, false, in_new_rev, in_old_rev);
                path_rep = c2p;
            } else {
                let (p2c, _ignored) = self.dec.create_marker_pair(member, path_member, true, in_old_rev, in_new_rev);
                path_rep = p2c;
            }
        } else if valid(r.first_path_arc) {
            path_rep = self.nc.path_arcs[r.first_path_arc as usize].arc;
        } else {
            path_rep = INV;
        }
        let mut non_path_rep = INV;
        if create_non_path_series {
            let non_path_member = self.dec.create_member(Mt::Series);
            let mut arc = self.dec.get_first_member_arc(member);
            let mut parent_moved = false;
            let mut can_stop = false;
            loop {
                let next_arc = self.dec.get_next_member_arc(arc);
                if arc != path_rep && arc != exc1 && arc != exc2 {
                    parent_moved = parent_moved || self.dec.marker_to_parent(member) == arc;
                    self.dec.move_arc_to_new_member(arc, member, non_path_member);
                } else {
                    can_stop = true;
                }
                arc = next_arc;
                if can_stop && arc == self.dec.get_first_member_arc(member) {
                    break;
                }
            }
            let mut representative_is_tree = !self.dec.arc_is_tree(exc1);
            if valid(exc2) {
                representative_is_tree = representative_is_tree || !self.dec.arc_is_tree(exc2);
            }
            let (in_old_rev, in_new_rev) = (true, false);
            if parent_moved {
                let (_i, c2p) = self.dec.create_marker_pair(non_path_member, member, !representative_is_tree, in_new_rev, in_old_rev);
                non_path_rep = c2p;
            } else {
                let (p2c, _i) = self.dec.create_marker_pair(member, non_path_member, representative_is_tree, in_old_rev, in_new_rev);
                non_path_rep = p2c;
            }
        } else if num_non_path != 0 {
            let first_arc = self.dec.get_first_member_arc(member);
            let mut arc = first_arc;
            loop {
                if arc != path_rep && arc != exc1 && arc != exc2 {
                    non_path_rep = arc;
                    break;
                }
                arc = self.dec.get_next_member_arc(arc);
                if arc == first_arc {
                    break;
                }
            }
        }
        (path_rep, non_path_rep)
    }

    /// (代表の弧, まとめた部分)
    fn transform_first_path_member(&mut self, rm: i32, info: &mut NewColInfo) -> (i32, i32) {
        let r = self.nc.reduced_members[rm as usize];
        let member = r.member;
        if self.dec.get_member_type(member) == Mt::Rigid {
            match r.path_type {
                Pt::IntoHead | Pt::IntoTail => info.tail = r.rigid_path_start,
                Pt::OutHead | Pt::OutTail => info.head = r.rigid_path_end,
            }
            let rep = self.dec.find_arc_sign(r.path_target_arc).0;
            return (rep, member);
        }
        let target = r.path_target_arc;
        let (path_rep, non_path_rep) = self.split_series_merging(rm, member, target, INV);
        let a = self.dec.create_node();
        let b = self.dec.create_node();
        let c = self.dec.create_node();
        let target_reversed = self.dec.arc_is_reversed_non_rigid(target);
        self.dec.set_arc_head_and_tail(target, c, b);
        let path_type = r.path_type;
        if self.dec.arc_is_reversed_non_rigid(path_rep) == target_reversed {
            self.dec.set_arc_head_and_tail(path_rep, a, c);
        } else {
            self.dec.set_arc_head_and_tail(path_rep, c, a);
        }
        if self.dec.arc_is_reversed_non_rigid(non_path_rep) == target_reversed {
            self.dec.set_arc_head_and_tail(non_path_rep, b, a);
        } else {
            self.dec.set_arc_head_and_tail(non_path_rep, a, b);
        }
        self.dec.arc_set_reversed(target, false);
        self.dec.arc_set_reversed(path_rep, false);
        self.dec.arc_set_reversed(non_path_rep, false);
        self.dec.arc_set_representative(target, INV);
        self.dec.arc_set_representative(path_rep, target);
        self.dec.arc_set_representative(non_path_rep, target);
        if path_type == Pt::IntoHead {
            info.tail = a;
        } else {
            info.head = a;
        }
        (target, member)
    }

    fn transform_and_merge_parallel(&mut self, current: i32, next: i32, next_member: i32, next_is_parent: bool, rep_arc: &mut i32, merged: &mut i32) {
        let mut next_member = next_member;
        let source = self.nc.reduced_members[next as usize].path_source_arc;
        let target = self.nc.reduced_members[next as usize].path_target_arc;
        if self.dec.get_num_member_arcs(next_member) > 3 {
            let child = self.split_parallel(next_member, source, target);
            next_member = child;
            self.nc.reduced_members[next as usize].member = child;
        }
        let source_head = self.dec.create_node();
        let source_tail = self.dec.create_node();
        {
            let first_arc = self.dec.get_first_member_arc(next_member);
            let mut arc = first_arc;
            let source_reversed = self.dec.arc_is_reversed_non_rigid(source);
            loop {
                if self.dec.arc_is_reversed_non_rigid(arc) == source_reversed {
                    self.dec.set_arc_head_and_tail(arc, source_head, source_tail);
                } else {
                    self.dec.set_arc_head_and_tail(arc, source_tail, source_head);
                }
                self.dec.arc_set_representative(arc, source);
                self.dec.arc_set_reversed(arc, false);
                arc = self.dec.get_next_member_arc(arc);
                if arc == first_arc {
                    break;
                }
            }
            self.dec.arc_set_representative(source, INV);
        }
        *rep_arc = self.dec.merge_arc_signs(*rep_arc, source, false);
        let cur_target = self.nc.reduced_members[current as usize].path_target_arc;
        let nm = if next_is_parent {
            self.dec.merge_given_member_into_parent(*merged, next_member, source, cur_target, true)
        } else {
            self.dec.merge_given_member_into_parent(next_member, *merged, cur_target, source, true)
        };
        *merged = nm;
    }

    fn transform_and_merge_series(&mut self, current: i32, next: i32, next_member: i32, next_is_parent: bool, rep_arc: &mut i32, merged: &mut i32, info: &mut NewColInfo) {
        let source = self.nc.reduced_members[next as usize].path_source_arc;
        let target = self.nc.reduced_members[next as usize].path_target_arc;
        let (path_rep, non_path_rep) = self.split_series_merging(next, next_member, source, target);
        let a = self.dec.create_node();
        let b = self.dec.create_node();
        let c = if valid(path_rep) { self.dec.create_node() } else { b };
        let has_non_path = valid(non_path_rep);
        let has_target = valid(target);
        let d = if has_non_path && has_target {
            self.dec.create_node()
        } else if has_non_path {
            c
        } else {
            a
        };
        let source_reversed = self.dec.arc_is_reversed_non_rigid(source);
        let cur_path_type = self.nc.reduced_members[current as usize].path_type;
        let path_start_in_head = is_head(cur_path_type);
        if path_start_in_head {
            self.dec.set_arc_head_and_tail(source, b, a);
        } else {
            self.dec.set_arc_head_and_tail(source, a, b);
        }
        if valid(path_rep) {
            if (self.dec.arc_is_reversed_non_rigid(path_rep) == source_reversed) == path_start_in_head {
                self.dec.set_arc_head_and_tail(path_rep, c, b);
            } else {
                self.dec.set_arc_head_and_tail(path_rep, b, c);
            }
            self.dec.arc_set_reversed(path_rep, false);
            self.dec.arc_set_representative(path_rep, source);
        }
        if has_target {
            if (self.dec.arc_is_reversed_non_rigid(target) == source_reversed) == path_start_in_head {
                self.dec.set_arc_head_and_tail(target, d, c);
            } else {
                self.dec.set_arc_head_and_tail(target, c, d);
            }
            self.dec.arc_set_reversed(target, false);
            self.dec.arc_set_representative(target, source);
        }
        if has_non_path {
            if (self.dec.arc_is_reversed_non_rigid(non_path_rep) == source_reversed) == path_start_in_head {
                self.dec.set_arc_head_and_tail(non_path_rep, a, d);
            } else {
                self.dec.set_arc_head_and_tail(non_path_rep, d, a);
            }
            self.dec.arc_set_reversed(non_path_rep, false);
            self.dec.arc_set_representative(non_path_rep, source);
        }
        self.dec.arc_set_reversed(source, false);
        self.dec.arc_set_representative(source, INV);
        let cur_target = self.nc.reduced_members[current as usize].path_target_arc;
        let nm = if next_is_parent {
            self.dec.merge_given_member_into_parent(*merged, next_member, source, cur_target, true)
        } else {
            self.dec.merge_given_member_into_parent(next_member, *merged, cur_target, source, true)
        };
        *merged = nm;
        *rep_arc = self.dec.merge_arc_signs(*rep_arc, source, false);
        if !has_target {
            info.reversed = false;
            if is_into(cur_path_type) {
                info.head = c;
            } else {
                info.tail = c;
            }
            info.member = *merged;
            info.representative = *rep_arc;
        }
    }

    fn transform_and_merge_rigid(&mut self, current: i32, next: i32, next_member: i32, next_is_parent: bool, rep_arc: &mut i32, merged: &mut i32, info: &mut NewColInfo) {
        let r = self.nc.reduced_members[next as usize];
        let source = r.path_source_arc;
        let source_rep = self.dec.find_arc_sign(source).0;
        let cur_target = self.nc.reduced_members[current as usize].path_target_arc;
        let nm = if next_is_parent {
            self.dec.merge_given_member_into_parent(*merged, next_member, source, cur_target, !r.reverse_arcs)
        } else {
            self.dec.merge_given_member_into_parent(next_member, *merged, cur_target, source, !r.reverse_arcs)
        };
        *merged = nm;
        *rep_arc = self.dec.merge_arc_signs(*rep_arc, source_rep, r.reverse_arcs);
        if !valid(r.path_target_arc) {
            info.reversed = false;
            let cur_path_type = self.nc.reduced_members[current as usize].path_type;
            if is_into(cur_path_type) {
                info.head = if r.reverse_arcs { r.rigid_path_start } else { r.rigid_path_end };
            } else {
                info.tail = if r.reverse_arcs { r.rigid_path_end } else { r.rigid_path_start };
            }
            info.member = *merged;
            info.representative = *rep_arc;
        }
    }

    fn transform_path(&mut self, ci: usize, info: &mut NewColInfo) {
        let first = self.nc.reduced_components[ci].path_end_members[0];
        let (mut rep_arc, mut merged) = self.transform_first_path_member(first, info);
        let mut current = first;
        let mut next = self.nc.reduced_members[current as usize].next_path_member;
        let mut next_is_parent = self.nc.reduced_members[current as usize].next_path_member_is_parent;
        while valid(next) {
            let next_member = self.nc.reduced_members[next as usize].member;
            match self.dec.get_member_type(next_member) {
                Mt::Rigid => self.transform_and_merge_rigid(current, next, next_member, next_is_parent, &mut rep_arc, &mut merged, info),
                Mt::Parallel => self.transform_and_merge_parallel(current, next, next_member, next_is_parent, &mut rep_arc, &mut merged),
                Mt::Series => self.transform_and_merge_series(current, next, next_member, next_is_parent, &mut rep_arc, &mut merged, info),
                Mt::Loop | Mt::Unassigned => panic!("loop member on the path"),
            }
            current = next;
            next = self.nc.reduced_members[current as usize].next_path_member;
            next_is_parent = self.nc.reduced_members[current as usize].next_path_member_is_parent;
        }
    }

    fn column_transform_single_rigid(&mut self, rm: i32, member: i32, info: &mut NewColInfo) {
        let r = self.nc.reduced_members[rm as usize];
        let start = r.rigid_path_start;
        let end = r.rigid_path_end;
        let mut existing = INV;
        let mut path_in_same_direction = false;
        {
            let first_arc = self.dec.get_first_node_arc(start);
            let mut arc = first_arc;
            loop {
                let head = self.dec.find_arc_head(arc);
                let tail = self.dec.find_arc_tail(arc);
                let other = if head == start { tail } else { head };
                if other == end {
                    existing = arc;
                    path_in_same_direction = (head == other) != self.dec.find_arc_sign(existing).1;
                    break;
                }
                arc = self.dec.get_next_node_arc(arc, start);
                if arc == first_arc {
                    break;
                }
            }
        }
        if valid(existing) {
            let mut is_parent = false;
            let mut adjacent_member = if self.dec.arc_is_marker(existing) { self.dec.find_arc_child_member(existing) } else { INV };
            if existing == self.dec.marker_to_parent(member) {
                adjacent_member = self.dec.find_member_parent(member);
                is_parent = true;
            }
            if valid(adjacent_member) && self.dec.get_member_type(adjacent_member) == Mt::Parallel {
                let parallel_marker = if is_parent { self.dec.marker_of_parent(member) } else { self.dec.marker_to_parent(adjacent_member) };
                let marker_reversed = self.dec.arc_is_reversed_non_rigid(parallel_marker);
                info.member = adjacent_member;
                info.reversed = marker_reversed == path_in_same_direction;
            } else {
                let adjacent_parallel = self.dec.create_member(Mt::Parallel);
                let element = self.dec.arcs[existing as usize].element;
                let existing_is_tree = self.dec.arc_is_tree(existing);
                if element != MARKER_COL && element != MARKER_ROW {
                    if !element_is_row(element) {
                        self.dec.create_column_arc(adjacent_parallel, element, false);
                    } else {
                        self.dec.create_row_arc(adjacent_parallel, element_to_row(element), false);
                    }
                } else if is_parent {
                    let mop = self.dec.marker_of_parent(member);
                    self.dec.create_parent_marker(adjacent_parallel, existing_is_tree, adjacent_member, mop, false);
                } else {
                    let dup = self.dec.create_child_marker(adjacent_parallel, adjacent_member, existing_is_tree, false);
                    self.dec.members[adjacent_member as usize].parent = adjacent_parallel;
                    self.dec.members[adjacent_member as usize].marker_of_parent = dup;
                }
                let parallel_marker = if is_parent {
                    self.dec.create_child_marker(adjacent_parallel, member, !existing_is_tree, false)
                } else {
                    self.dec.create_parent_marker(adjacent_parallel, !existing_is_tree, member, existing, false)
                };
                if is_parent {
                    let mop = self.dec.marker_of_parent(member);
                    self.dec.arcs[mop as usize].child_member = adjacent_parallel;
                    self.dec.members[member as usize].parent = adjacent_parallel;
                    self.dec.members[member as usize].marker_to_parent = existing;
                    self.dec.members[member as usize].marker_of_parent = parallel_marker;
                    self.dec.arcs[existing as usize].element = if existing_is_tree { MARKER_ROW } else { MARKER_COL };
                    self.dec.arcs[existing as usize].child_member = adjacent_parallel;
                } else {
                    self.dec.arcs[existing as usize].element = if existing_is_tree { MARKER_ROW } else { MARKER_COL };
                    self.dec.arcs[existing as usize].child_member = adjacent_parallel;
                }
                info.member = adjacent_parallel;
                info.reversed = !path_in_same_direction;
            }
            return;
        }
        info.member = member;
        info.reversed = false;
        info.tail = start;
        info.head = end;
        let fa = self.nc.path_arcs[r.first_path_arc as usize].arc;
        info.representative = self.dec.find_arc_sign(fa).0;
    }

    fn transform_component(&mut self, ci: usize, info: &mut NewColInfo) {
        let root = self.nc.reduced_components[ci].root;
        let r = self.nc.reduced_members[root as usize];
        if r.num_children == r.num_propagated_children {
            let member = r.member;
            match self.dec.get_member_type(member) {
                Mt::Rigid => self.column_transform_single_rigid(root, member, info),
                Mt::Parallel => {
                    info.reversed = r.path_backwards;
                    info.member = member;
                }
                Mt::Loop | Mt::Series => {
                    if self.dec.get_num_member_arcs(member) == 1 {
                        info.member = member;
                        let fa = self.dec.get_first_member_arc(member);
                        info.reversed = self.nc.arc_in_path_reversed.get(fa as usize).copied().unwrap_or(false);
                    } else {
                        self.split_series(root, member, info);
                    }
                }
                Mt::Unassigned => panic!("unassigned member"),
            }
            return;
        }
        self.transform_path(ci, info);
    }

    fn column_addition_add(&mut self) {
        let ncomp = self.nc.reduced_components.len();
        let col = self.nc.new_col_index;
        let new_rows = self.nc.new_row_arcs.clone();
        let new_rev = self.nc.new_row_arc_reversed.clone();
        if ncomp == 0 {
            self.dec.create_standalone_series(&new_rows, &new_rev, col);
        } else if ncomp == 1 {
            let mut info = empty_info();
            self.transform_component(0, &mut info);
            if new_rows.is_empty() {
                let col_arc = self.dec.create_column_arc(info.member, col, info.reversed);
                if valid(info.head) {
                    let h = self.dec.find_node(info.head);
                    let t = self.dec.find_node(info.tail);
                    self.dec.set_arc_head_and_tail(col_arc, h, t);
                    self.dec.arc_set_representative(col_arc, info.representative);
                    let rr = self.dec.arc_is_reversed_non_rigid(info.representative);
                    self.dec.arc_set_reversed(col_arc, info.reversed != rr);
                }
            } else {
                let new_series = self.dec.create_connected_series(&new_rows, &new_rev, col);
                let (marker_arc, _ignore) = self.dec.create_marker_pair(info.member, new_series, false, info.reversed, true);
                if valid(info.head) {
                    let h = self.dec.find_node(info.head);
                    let t = self.dec.find_node(info.tail);
                    self.dec.set_arc_head_and_tail(marker_arc, h, t);
                    self.dec.arc_set_representative(marker_arc, info.representative);
                    let rr = self.dec.arc_is_reversed_non_rigid(info.representative);
                    self.dec.arc_set_reversed(marker_arc, info.reversed != rr);
                }
            }
            if self.dec.get_member_type(info.member) == Mt::Loop && self.dec.get_num_member_arcs(info.member) == 3 {
                self.dec.change_loop_to_parallel(info.member);
            }
        } else {
            let new_series = self.dec.create_connected_series(&new_rows, &new_rev, col);
            for i in 0..ncomp {
                let mut info = empty_info();
                self.transform_component(i, &mut info);
                if self.dec.get_member_type(info.member) == Mt::Loop {
                    let arc = self.dec.get_first_member_arc(info.member);
                    self.dec.move_arc_to_new_member(arc, info.member, new_series);
                    let rev = self.nc.arc_in_path_reversed.get(arc as usize).copied().unwrap_or(false);
                    self.dec.arc_set_reversed(arc, !rev);
                    self.dec.members[info.member as usize].ty = Mt::Unassigned;
                } else {
                    self.dec.reorder_component(info.member);
                    let (_ignore, marker_arc) = self.dec.create_marker_pair(new_series, info.member, true, info.reversed, true);
                    if valid(info.head) {
                        let h = self.dec.find_node(info.head);
                        let t = self.dec.find_node(info.tail);
                        self.dec.set_arc_head_and_tail(marker_arc, h, t);
                        self.dec.arc_set_representative(marker_arc, info.representative);
                        let rr = self.dec.arc_is_reversed_non_rigid(info.representative);
                        self.dec.arc_set_reversed(marker_arc, info.reversed == rr);
                    }
                }
            }
            self.dec.num_cc -= ncomp as i32 - 1;
        }
    }

    /// 分解から無向のグラフを組み立てる: 行 r の弧の両端 (row_ends[r])、列 c の弧の両端 (col_ends[c])。節点の番号は詰めない。
    /// 直列は弧の並びの順に輪にし、並列は 2 節点の間に並べ、剛は分解の節点を使う。印の対は 2-和として両端を同一視する
    /// (どちらの向きに同一視しても閉路マトロイドは同じ)。
    fn realization(&mut self) -> Option<(Vec<(usize, usize)>, Vec<(usize, usize)>)> {
        let na = self.dec.arcs.len();
        let mut ends: Vec<(usize, usize)> = vec![(usize::MAX, usize::MAX); na];
        let mut nn = self.dec.nodes.len();
        for m in 0..self.dec.members.len() as i32 {
            if !self.dec.member_is_representative(m) {
                continue;
            }
            let ty = self.dec.get_member_type(m);
            let first = self.dec.get_first_member_arc(m);
            if ty == Mt::Unassigned || !valid(first) {
                continue;
            }
            let k = self.dec.get_num_member_arcs(m) as usize;
            let mut arcs = Vec::with_capacity(k);
            let mut a = first;
            loop {
                arcs.push(a);
                a = self.dec.get_next_member_arc(a);
                if a == first || arcs.len() > k {
                    break;
                }
            }
            if arcs.len() != k {
                return None;
            }
            match ty {
                Mt::Rigid => {
                    for &a in &arcs {
                        let h = self.dec.find_arc_head(a);
                        let t = self.dec.find_arc_tail(a);
                        if !valid(h) || !valid(t) {
                            return None;
                        }
                        ends[a as usize] = (h as usize, t as usize);
                    }
                }
                Mt::Series => {
                    let base = nn;
                    nn += k;
                    for (i, &a) in arcs.iter().enumerate() {
                        ends[a as usize] = (base + i, base + (i + 1) % k);
                    }
                }
                Mt::Parallel | Mt::Loop => {
                    let base = nn;
                    nn += 2;
                    for &a in &arcs {
                        ends[a as usize] = if k == 1 { (base, base) } else { (base, base + 1) };
                    }
                }
                Mt::Unassigned => {}
            }
        }
        // 2-和: 子の親への印と親の子への印の両端を同一視
        let mut uf: Vec<usize> = (0..nn).collect();
        fn f(uf: &mut [usize], mut x: usize) -> usize {
            while uf[x] != x {
                uf[x] = uf[uf[x]];
                x = uf[x];
            }
            x
        }
        let mut is_marker = vec![false; na];
        for m in 0..self.dec.members.len() as i32 {
            if !self.dec.member_is_representative(m) || self.dec.get_member_type(m) == Mt::Unassigned || !valid(self.dec.get_first_member_arc(m)) {
                continue;
            }
            let p = self.dec.find_member_parent(m);
            if !valid(p) {
                continue;
            }
            let (ac, ap) = (self.dec.marker_to_parent(m), self.dec.marker_of_parent(m));
            if !valid(ac) || !valid(ap) {
                return None;
            }
            let (ec, ep) = (ends[ac as usize], ends[ap as usize]);
            if ec.0 == usize::MAX || ep.0 == usize::MAX {
                return None;
            }
            is_marker[ac as usize] = true;
            is_marker[ap as usize] = true;
            let (x, y) = (f(&mut uf, ec.0), f(&mut uf, ep.0));
            uf[x] = y;
            let (x, y) = (f(&mut uf, ec.1), f(&mut uf, ep.1));
            uf[x] = y;
        }
        let mut row_ends = vec![(usize::MAX, usize::MAX); self.dec.row_arcs.len()];
        let mut col_ends = vec![(usize::MAX, usize::MAX); self.dec.col_arcs.len()];
        for a in 0..na {
            let e = self.dec.arcs[a].element;
            if is_marker[a] || e == MARKER_ROW || e == MARKER_COL || ends[a].0 == usize::MAX {
                continue;
            }
            let (u, v) = (f(&mut uf, ends[a].0), f(&mut uf, ends[a].1));
            if element_is_row(e) {
                let r = element_to_row(e) as usize;
                if self.dec.row_arcs[r] == a as i32 {
                    row_ends[r] = (u, v);
                }
            } else if self.dec.col_arcs[e as usize] == a as i32 {
                col_ends[e as usize] = (u, v);
            }
        }
        Some((row_ends, col_ends))
    }
}

/// 証明書の検査: `row_ends`・`col_ends` (無向のグラフ) で行の弧が森になり、どの列 (非零が 2 以上) も両端を結ぶ木の道の弧がちょうど
/// 列の非零の行で、符号が行・列の ±1 倍を除いて道の向き (任意に向けた木の弧に対する) と一致するか。
fn verify(nrows: usize, cols: &[Vec<(usize, bool)>], row_ends: &[(usize, usize)], col_ends: &[(usize, usize)]) -> bool {
    // 使う行
    let mut used = vec![false; nrows];
    for c in cols {
        if c.len() >= 2 {
            for &(r, _) in c {
                used[r] = true;
            }
        }
    }
    // 森: 節点の番号を詰めて隣接を作る
    let mut id: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut adj: Vec<Vec<(usize, usize)>> = Vec::new(); // (隣, 行)
    let mut nid = |x: usize, adj: &mut Vec<Vec<(usize, usize)>>| -> usize {
        let k = id.len();
        let v = *id.entry(x).or_insert(k);
        if v == adj.len() {
            adj.push(Vec::new());
        }
        v
    };
    let mut uf: Vec<usize> = Vec::new();
    fn f(uf: &mut [usize], mut x: usize) -> usize {
        while uf[x] != x {
            uf[x] = uf[uf[x]];
            x = uf[x];
        }
        x
    }
    for r in 0..nrows {
        if !used[r] {
            continue;
        }
        let (a, b) = row_ends[r];
        if a == usize::MAX || a == b {
            return false;
        }
        let (u, v) = (nid(a, &mut adj), nid(b, &mut adj));
        while uf.len() < adj.len() {
            uf.push(uf.len());
        }
        let (x, y) = (f(&mut uf, u), f(&mut uf, v));
        if x == y {
            return false; // 行の弧が閉路を作る
        }
        uf[x] = y;
        adj[u].push((v, r));
        adj[v].push((u, r));
    }
    let n = adj.len();
    // 根つきの森: 親・親への行・深さ。行の弧の向きは「子 → 親」とする
    let mut parent = vec![usize::MAX; n];
    let mut prow = vec![usize::MAX; n];
    let mut depth = vec![0usize; n];
    let mut seen = vec![false; n];
    for s in 0..n {
        if seen[s] {
            continue;
        }
        seen[s] = true;
        let mut st = vec![s];
        while let Some(u) = st.pop() {
            for &(v, r) in &adj[u] {
                if !seen[v] {
                    seen[v] = true;
                    parent[v] = u;
                    prow[v] = r;
                    depth[v] = depth[u] + 1;
                    st.push(v);
                }
            }
        }
    }
    // 符号の 2 色塗り: 行と列に ±1 を付けて A_rc = s_r t_c N_rc。節点 = 行 (0..nrows) と列 (nrows..)
    let ncols = cols.len();
    let mut sadj: Vec<Vec<(usize, bool)>> = vec![Vec::new(); nrows + ncols];
    let mut mark = vec![usize::MAX; nrows];
    for (j, c) in cols.iter().enumerate() {
        if c.len() < 2 {
            continue;
        }
        let (a, b) = col_ends[j];
        if a == usize::MAX {
            return false;
        }
        let (Some(&u0), Some(&v0)) = (id.get(&a), id.get(&b)) else { return false };
        if f(&mut uf, u0) != f(&mut uf, v0) {
            return false;
        }
        for &(r, _) in c {
            mark[r] = j;
        }
        // u0 から v0 への道: u0 側は上り (子 → 親 = 弧の向き、+1)、v0 側は下り (-1)
        let (mut u, mut v) = (u0, v0);
        let mut path: Vec<(usize, bool)> = Vec::new(); // (行, 道の向きが弧と逆か)
        while u != v {
            if path.len() > c.len() {
                return false;
            }
            if depth[u] >= depth[v] {
                path.push((prow[u], false));
                u = parent[u];
            } else {
                path.push((prow[v], true));
                v = parent[v];
            }
        }
        if path.len() != c.len() {
            return false;
        }
        let neg: std::collections::HashMap<usize, bool> = c.iter().copied().collect();
        for &(r, back) in &path {
            if mark[r] != j {
                return false;
            }
            let a_neg = neg[&r];
            // A_rj と N_rj の符号の違い: s_r と t_j の積が (a_neg != back)
            let differ = a_neg != back;
            sadj[r].push((nrows + j, differ));
            sadj[nrows + j].push((r, differ));
        }
    }
    let mut color: Vec<i8> = vec![-1; nrows + ncols];
    for s in 0..nrows + ncols {
        if color[s] >= 0 || sadj[s].is_empty() {
            continue;
        }
        color[s] = 0;
        let mut st = vec![s];
        while let Some(u) = st.pop() {
            for &(v, d) in &sadj[u] {
                let want = color[u] ^ (d as i8);
                if color[v] < 0 {
                    color[v] = want;
                    st.push(v);
                } else if color[v] != want {
                    return false;
                }
            }
        }
    }
    true
}

/// 列の形の 0/±1 の行列 (列ごとに (行, 負か)) がネットワーク行列か (MATREC の列の追加で判定し、証明書を確かめる)。
/// 非零が 1 つ以下の列は飛ばす (道 1 本か空の列はどの実現にも足せ、TU も保つ)。偽は「ネットワーク行列でない」か「判定の途中で
/// 失敗した」(どちらも安全側)。
pub(crate) fn is_network_cols(nrows: usize, cols: &[Vec<(usize, bool)>]) -> bool {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut net = NetCols::new(nrows, cols.len());
        for (j, c) in cols.iter().enumerate() {
            if c.len() < 2 {
                continue;
            }
            if !net.add(j, c) {
                return Some(false);
            }
        }
        let (row_ends, col_ends) = net.realization()?;
        Some(verify(nrows, cols, &row_ends, &col_ends))
    }));
    match r {
        Ok(Some(true)) => true,
        Ok(Some(false)) => false,
        Ok(None) => {
            NET_VERIFY_FAIL.with(|c| c.set(c.get() + 1));
            false
        }
        Err(_) => {
            NET_PANICS.with(|c| c.set(c.get() + 1));
            false
        }
    }
}

/// [`is_network_cols`] の本体だけ (証明書の検査と catch_unwind の前の、MATREC の答え)。単体テストで証明書の検査と食い違わないかを見る用。
#[cfg(test)]
fn matrec_says_network(nrows: usize, cols: &[Vec<(usize, bool)>]) -> bool {
    let mut net = NetCols::new(nrows, cols.len());
    cols.iter().enumerate().all(|(j, c)| c.len() < 2 || net.add(j, c))
}

/// 0/±1 の行列 (列 → (行, 負か)) がネットワーク行列かその転置か (列の数・行の数を入れ替えて同じ判定)。
pub(crate) fn is_network_or_transpose(nrows: usize, cols: &[Vec<(usize, bool)>]) -> bool {
    if is_network_cols(nrows, cols) {
        return true;
    }
    let mut t: Vec<Vec<(usize, bool)>> = vec![Vec::new(); nrows];
    for (j, c) in cols.iter().enumerate() {
        for &(r, s) in c {
            t[r].push((j, s));
        }
    }
    is_network_cols(cols.len(), &t)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    /// 乱数の木 (節点 nrows+1、弧 = 行) と、乱数の節点の対の列でネットワーク行列を作る。行・列の順は乱数で入れ替える。
    fn random_network(rng: &mut Rng, nrows: usize, ncols: usize) -> Vec<Vec<(usize, bool)>> {
        let n = nrows + 1;
        // 木: 節点 v (1..n) の親 p(v) < v。弧 v は向き (v → p) か (p → v)
        let mut par = vec![0usize; n];
        let mut up = vec![true; n];
        for v in 1..n {
            // 道に近い木や星形も混ぜる
            par[v] = match rng.below(3) {
                0 => v - 1,
                1 => rng.below(v),
                _ => rng.below(v.min(3)),
            };
            up[v] = rng.below(2) == 0;
        }
        let mut perm: Vec<usize> = (0..nrows).collect();
        for i in (1..nrows).rev() {
            perm.swap(i, rng.below(i + 1));
        }
        let mut depth = vec![0usize; n];
        for v in 1..n {
            depth[v] = depth[par[v]] + 1;
        }
        let mut cols = Vec::new();
        for _ in 0..ncols {
            let (mut a, mut b) = (rng.below(n), rng.below(n));
            if a == b {
                continue;
            }
            // a から b への道。弧 v (v と par[v] の間) を a 側では上りに、b 側では下りに通る
            let mut col = Vec::new();
            while a != b {
                if depth[a] >= depth[b] {
                    // 上り: v → par[v]。弧の向きが up なら +1
                    col.push((perm[a - 1], !up[a]));
                    a = par[a];
                } else {
                    col.push((perm[b - 1], up[b]));
                    b = par[b];
                }
            }
            cols.push(col);
        }
        cols
    }

    /// 行列式の絶対値 (小さい行列用、分数なしのガウスの消去を f64 で)
    fn det(m: &[Vec<f64>]) -> f64 {
        let n = m.len();
        let mut a: Vec<Vec<f64>> = m.to_vec();
        let mut d = 1.0;
        for i in 0..n {
            let p = (i..n).max_by(|&x, &y| a[x][i].abs().partial_cmp(&a[y][i].abs()).unwrap()).unwrap();
            if a[p][i].abs() < 1e-9 {
                return 0.0;
            }
            if p != i {
                a.swap(p, i);
                d = -d;
            }
            d *= a[i][i];
            for r in i + 1..n {
                let f = a[r][i] / a[i][i];
                for c in i..n {
                    a[r][c] -= f * a[i][c];
                }
            }
        }
        d
    }

    /// 全部の正方の部分行列の行列式で TU を確かめる (行・列とも 7 以下用)
    fn brute_tu(nrows: usize, cols: &[Vec<(usize, bool)>]) -> bool {
        let ncols = cols.len();
        let mut a = vec![vec![0.0; ncols]; nrows];
        for (j, c) in cols.iter().enumerate() {
            for &(r, s) in c {
                a[r][j] = if s { -1.0 } else { 1.0 };
            }
        }
        for rs in 1u32..(1 << nrows) {
            let ri: Vec<usize> = (0..nrows).filter(|&i| rs >> i & 1 == 1).collect();
            for cs in 1u32..(1 << ncols) {
                if cs.count_ones() != rs.count_ones() {
                    continue;
                }
                let ci: Vec<usize> = (0..ncols).filter(|&j| cs >> j & 1 == 1).collect();
                let m: Vec<Vec<f64>> = ri.iter().map(|&i| ci.iter().map(|&j| a[i][j]).collect()).collect();
                if det(&m).abs() > 1.5 {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn random_network_matrices_are_accepted() {
        let mut rng = Rng(0x1234_5678_9abc_def1);
        for t in 0..3000 {
            let nrows = 1 + rng.below(if t < 2000 { 8 } else { 60 });
            let ncols = rng.below(if t < 2000 { 10 } else { 120 });
            let cols = random_network(&mut rng, nrows, ncols);
            assert!(matrec_says_network(nrows, &cols), "MATREC rejected a network matrix (case {t}): {cols:?}");
            assert!(is_network_cols(nrows, &cols), "certificate failed on a network matrix (case {t}): {cols:?}");
            // 転置もネットワーク行列かその転置
            assert!(is_network_or_transpose(nrows, &cols));
        }
        assert_eq!(NET_PANICS.with(|c| c.get()), 0);
        assert_eq!(NET_VERIFY_FAIL.with(|c| c.get()), 0);
    }

    #[test]
    fn perturbed_matrices_agree_with_brute_force() {
        // ネットワーク行列の符号を 1 つ変える・非零を 1 つ足す/消す。判定が真なら総当たりの TU も真 (健全性)。
        // ネットワーク行列でない TU 行列はありうるので、真偽が食い違うのは「判定が偽で総当たりが真」の向きだけ許す。
        let mut rng = Rng(0x0bad_cafe_dead_beef);
        let (mut n_true, mut n_tu) = (0, 0);
        for _ in 0..4000 {
            let nrows = 2 + rng.below(5);
            let ncols = 2 + rng.below(5);
            let mut cols = random_network(&mut rng, nrows, ncols + 2);
            cols.truncate(ncols.min(cols.len()));
            if cols.is_empty() {
                continue;
            }
            let j = rng.below(cols.len());
            match rng.below(3) {
                0 if !cols[j].is_empty() => {
                    let k = rng.below(cols[j].len());
                    cols[j][k].1 = !cols[j][k].1;
                }
                1 => {
                    let r = rng.below(nrows);
                    if !cols[j].iter().any(|e| e.0 == r) {
                        cols[j].push((r, rng.below(2) == 0));
                    }
                }
                _ if !cols[j].is_empty() => {
                    let k = rng.below(cols[j].len());
                    cols[j].remove(k);
                }
                _ => {}
            }
            let net = is_network_cols(nrows, &cols);
            let tu = brute_tu(nrows, &cols);
            if net {
                n_true += 1;
                assert!(tu, "network test said true on a non-TU matrix: {cols:?}");
            }
            if tu {
                n_tu += 1;
            }
            // MATREC の答えと証明書の検査は食い違わない (MATREC が真なのに証明書が偽なら移植の誤り)
            assert_eq!(matrec_says_network(nrows, &cols), net, "MATREC and the certificate disagree: {cols:?}");
        }
        assert!(n_true > 500 && n_tu >= n_true, "n_true {n_true} n_tu {n_tu}");
    }

    #[test]
    fn known_matrices() {
        // K_{3,3} 的な 0/1 の奇数の穴 (3x3 の巡回、行列式 2): TU でない
        let odd: Vec<Vec<(usize, bool)>> = vec![vec![(0, false), (1, false)], vec![(1, false), (2, false)], vec![(0, false), (2, false)]];
        assert!(!is_network_or_transpose(3, &odd));
        // 区間行列 (連続する 1、ネットワーク行列)
        let iv: Vec<Vec<(usize, bool)>> = vec![vec![(0, false), (1, false), (2, false)], vec![(1, false), (2, false)], vec![(2, false), (3, false)], vec![(0, false), (1, false)]];
        assert!(is_network_cols(4, &iv));
        // 有向グラフの接続行列の転置 (列 = 弧、+1 と -1): 転置がネットワーク行列
        let inc: Vec<Vec<(usize, bool)>> = vec![vec![(0, false), (1, true)], vec![(1, false), (2, true)], vec![(0, false), (2, true)], vec![(2, false), (3, true)]];
        assert!(is_network_or_transpose(4, &inc));
        // R10 (TU だがネットワーク行列でも転置でもない) の 0/±1 の形
        let r10 = [[1, -1, 0, 0, -1], [-1, 1, -1, 0, 0], [0, -1, 1, -1, 0], [0, 0, -1, 1, -1], [-1, 0, 0, -1, 1]];
        let cols: Vec<Vec<(usize, bool)>> = (0..5).map(|j| (0..5).filter(|&i| r10[i][j] != 0).map(|i| (i, r10[i][j] < 0)).collect()).collect();
        assert!(brute_tu(5, &cols));
        assert!(!is_network_or_transpose(5, &cols));
    }
}
