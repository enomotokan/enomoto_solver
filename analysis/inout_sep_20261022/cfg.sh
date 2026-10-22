# 設定の名前 → 環境変数
E2="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1"
cfg() {
  case $1 in
    def) echo "";;
    io) echo "ENOMOTO_MIP_INOUT_SEP=1";;
    ioac) echo "ENOMOTO_MIP_INOUT_SEP=ac";;
    ioinc) echo "ENOMOTO_MIP_INOUT_SEP=inc";;
    e2) echo "$E2";;
    e2io) echo "$E2 ENOMOTO_MIP_INOUT_SEP=1";;
    *) echo "${!1}";;
  esac
}
face="ENOMOTO_MIP_INOUT_SEP=face"
e2face="$E2 ENOMOTO_MIP_INOUT_SEP=face"
facenb="ENOMOTO_MIP_INOUT_SEP=face ENOMOTO_MIP_INOUT_NO_BG=1"
ionb="ENOMOTO_MIP_INOUT_SEP=1 ENOMOTO_MIP_INOUT_NO_BG=1"
