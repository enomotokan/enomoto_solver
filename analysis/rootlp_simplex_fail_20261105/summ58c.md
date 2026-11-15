== loss21
  base            14.26s (20/21)
  combB           15.30s (19/21)
== miplib40
  base            18.59s (27/40)
  combB           18.86s (26/40)

問題ごと (各回の時間、base と 20% 以上かつ 2 秒以上違うもの):
  mas76                  rep0 base=9.7  combB=12.1
  mc11                   rep0 base=25.3  combB=32.3
  misc07                 rep0 base=16.2  combB=12.5
  noswot                 rep0 base=25.2  combB=19.1
  qiu                    rep0 base=25.9  combB=32.7
  roll3000               rep0 base=38.8  combB=51.8

食い違い (どれかが optimal で目的値が違う) と異常終了:
