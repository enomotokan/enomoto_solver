# メモリの試作 mem3・前処理の強制等式 forceq・解発見スレッドの止め方 fstop の既定化 (2026-11-04)

既定にしたもの (それぞれ `=0` で使わない。まとめて戻すのは run.py の `preall4`):

| 環境変数 | 既定 | 中身 | 記録 |
|---|---|---|---|
| `ENOMOTO_MIP_MEMB` | `rcq` | 待ち行列のノードの基底を 2 ビットに詰める (`c`)、ノードの DSE 重みを 8 ビットの対数にする (`q`)、行の変更の記録の削除を番号で持つ (`r`) | `memcap_b_20261104.md` |
| `ENOMOTO_MIP_MEMC` | 有効 | 共有の待ち行列に出すノードの境界の変更の並びと基底を詰める (取り出すときに戻す、探索は変えない) | `memcap_c_20261104.md` |
| `ENOMOTO_MIP_MEMCAP` | 有効 | 0.5 秒ごとに RSS を読み、使えるメモリの 0.7 倍を超えたら開いたノードの DSE 重み・基底を捨てる、0.8 倍で深さ優先に寄せる、0.9 倍で止める (安全弁) | `memcap_a_20261104.md` |
| `ENOMOTO_MIP_TREE2_FORCEQ` | 有効 | 0-1 列 1 本の強制等式 `Σ x = K y` から `x_j = y` の 2 項等式を足す (前処理) | `tree_close2_20261104.md` |
| `ENOMOTO_MIP_QIU_FIND_STOP_GAP` | 有効 | FJ 以外の暫定解が入っても、根の LP との相対ギャップが 0.5 を越える間は根の解発見スレッドを止めない | `qiu_margin_20261104.md` |

## 根拠

- bench25 (`halve_submip_20261030.md`、60 秒): 58 問 2 回で mem3 −1.0% (84 → 86)、forceq −5.3% (87)、fstop −5.3% (86)、all4 −2.5% (86)。
  Mittelmann 186 問で all4 −0.4%・49 → 50。crash・食い違い 0。
- mem3 の 1200 秒 (`mitt600_20261103.md`): 既定 600 秒でメモリ不足で落ちた 6 問・上限の手前の 3 問の 9 問とも、1200 秒を落ちずに最大 RSS 1.3〜5.0 GB
  (既定 600 秒は 12.2〜13.6 GB)。
- forceq: neos-1445765 を 600 秒で時間切れ → 185 秒で最適 (tree_close2)。fstop: qiu 4/6 → 6/6、uccase12 も正しい (qiu_margin)。

## 確かめ (`mem3_default_20261104/check.py` → `check.out`)

- `cargo test --release --lib`: 339 通過・0 失敗。
- 既定 (def) と `preall4` で 3 問を短く解き、試作が働いた印のログの行を数えた: 既定では `memc (all threads)`・`MEMCAP` の行・`FORCEQ` の行が出て、
  `preall4` では出ない。neos-1445765 (40 秒) は既定で暫定解 −17593 (最適 −17783)、`preall4` で −6920。qiu (30 秒) は既定で解発見スレッドを止めない
  (`find thread asked to stop` が 0)、`preall4` で止める (1)。
