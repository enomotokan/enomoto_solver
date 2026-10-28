# 計測の待ち行列 (2026-10-28)。Claude のセッションと切り離してタスク スケジューラから動かす (ネットが切れても止まらないように)。
# 1 つずつ順に回す (同時に計測しない)。どの段も run.py --skip-done なので、止まってもう一度起動すれば続きから回る。
# 進み具合は queue.log、全部終われば queue.done。
$ErrorActionPreference = "Continue"
# このスクリプトが動いている間はスリープさせない (ES_CONTINUOUS | ES_SYSTEM_REQUIRED。画面は消えてよい)
Add-Type -Name Power -Namespace Win32 -MemberDefinition '[DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint esFlags);'
[void][Win32.Power]::SetThreadExecutionState([uint32]"0x80000001")
$wt = "C:\Users\enomo\wt_nbhdfj"
Set-Location $wt
$here = "$wt\analysis\heurfix_20261028"
$log = "$here\queue.log"
function Log($s) { "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') $s" | Out-File -Append -Encoding utf8 $log }
$env:PYTHONUTF8 = "1"
$env:VIRTUAL_ENV = "$wt\.venv"
$env:PATH = "$wt\.venv\Scripts;C:\Users\enomo\.cargo\bin;$env:PATH"
$py = "$wt\.venv\Scripts\python.exe"
$run = "$wt\analysis\restart_cuts_fix_20261028\run.py"
$L = @("sp150x300d","exp-1-500-5-5","neos-911970","p200x1188c","neos17","nu25-pr12","neos-3083819-nubu","roll3000","h80x6320d","swath1","drayage-25-23","drayage-100-23","neos-827175","mzzv42z","neos-960392","neos8","neos-860300","ns1116954","hypothyroid-k1","physiciansched6-2","mc11")
$M = (& $py -c "import sys;sys.path.insert(0,'scripts');import run_miplib_benchmark as r;print(' '.join(r.PROBLEMS))").Split(' ')
function R($out, $cfg, $k, $probs, $envs, $extra = @()) {
    $a = @($run, $out, $cfg, "1") + $probs + @("--threads", "16", "--rep-start", "$k", "--skip-done") + $extra
    if ($envs.Count -gt 0) { $a += @("--env") + $envs }
    & $py @a 2>&1 | Out-File -Append -Encoding utf8 "$here\queue_runs.log"
}
Log "start"

# 0. ほかの計測 (Claude のセッションから起動したものなど) が動いていれば終わるのを待つ
while (Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*restart_cuts_fix_20261028*run.py*eprostop_*' -or $_.CommandLine -like '*run_miplib_benchmark.py --worker-ours*' }) {
    Start-Sleep -Seconds 30
}
Start-Sleep -Seconds 20
# ループの次の問題が始まる隙間で抜けた場合のため、もう一度確かめる
while (Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*restart_cuts_fix_20261028*run.py*eprostop_*' -or $_.CommandLine -like '*run_miplib_benchmark.py --worker-ours*' }) {
    Start-Sleep -Seconds 30
}
Log "no other measurement running"

# 2. 修正 (PAR_HEUR_FOUND・RESTART_LIGHT_HEURS) を入れたビルドと cargo test (既定は eprostop を既定にした後のもの)
& cargo test --release 2>&1 | Select-String "test result|FAILED|panicked|^error" | Out-File -Append -Encoding utf8 $log
& maturin develop --release 2>&1 | Select-String "^error|Installed" | Out-File -Append -Encoding utf8 $log
Log "build done"

$C = [ordered]@{
    base     = @()
    parfound = @("ENOMOTO_MIP_PAR_HEUR_FOUND=1")
    light    = @("ENOMOTO_MIP_RESTART_LIGHT_HEURS=1")
    both     = @("ENOMOTO_MIP_PAR_HEUR_FOUND=1", "ENOMOTO_MIP_RESTART_LIGHT_HEURS=1")
}

# 3. 診断ログつきの確かめ (再スタートする 6 問 x 4 設定、各 1 回)
$D = @("p200x1188c","exp-1-500-5-5","h80x6320d","neos-860300","swath1","neos17")
foreach ($c in $C.Keys) {
    R "$here\diag.jsonl" $c 0 $D ($C[$c] + @("ENOMOTO_MIP_LOG=1")) @("--log-dir", "$wt\runs\heurfix_diag")
}
Log "diag done"

# 4. 負けている 21 問、4 設定を交互に各 2 回
foreach ($k in 0, 1) {
    foreach ($c in $C.Keys) { R "benchmarks\heurfix_loss21_16t_20261028.jsonl" $c $k $L $C[$c] }
}
Log "loss21 done"

# 5. MIPLIB 40 問、既定と both を交互に各 2 回
foreach ($k in 0, 1) {
    R "benchmarks\heurfix_miplib40_16t_20261028.jsonl" base $k $M @()
    R "benchmarks\heurfix_miplib40_16t_20261028.jsonl" both $k $M $C["both"]
}
Log "miplib40 done"
"done" | Out-File -Encoding utf8 "$here\queue.done"
