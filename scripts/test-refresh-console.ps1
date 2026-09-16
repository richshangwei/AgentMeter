param([int]$RepeatCount = 3, [switch]$InjectVisibleGrandchild, [switch]$AntigravityFixture,
    [ValidateSet('codex','claude','copilot','antigravity')][string]$LiveProvider,
    [string]$LiveWorkdir, [string]$InstalledExecutable, [int]$ObserveSeconds = 0,
    [int]$ExpectedGuiProcessId = 0)
$ErrorActionPreference = 'Stop'
# Run on the interactive desktop. The default mode never accesses a Provider
# account. InjectVisibleGrandchild is an intentional RED control; live modes are
# explicit opt-ins. Unrelated windows opened during the probe also fail it, with
# class/PID provenance for triage; window titles and Provider output are omitted.
if ($RepeatCount -lt 1 -or $RepeatCount -gt 20) { throw 'RepeatCount must be 1..20.' }
if ($ObserveSeconds -lt 0 -or $ObserveSeconds -gt 180) { throw 'ObserveSeconds must be 0..180.' }
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$runtime = Join-Path $repo 'desktop-p0/resources/quota-helper'
$activeRuntime = $runtime
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('agentmeter-console-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path (Join-Path $fixture '.local/bin') -Force | Out-Null
try {
    # This fake official CLI exercises the real discovery, auth, ConPTY and cleanup
    # paths without opening an account, network connection, or model turn.
    Add-Type -TypeDefinition @'
using System;
public class FakeClaude {
    public static void Main(string[] args) {
        if (Array.IndexOf(args, "--version") >= 0 && Environment.GetEnvironmentVariable("AGENTMETER_TEST_VISIBLE_CHILD") == "1") {
            var start = new System.Diagnostics.ProcessStartInfo("cmd.exe", "/d /c start \"AgentMeter test console\" /wait cmd /d /c ping -n 2 127.0.0.1");
            start.UseShellExecute = false;
            using (var child = System.Diagnostics.Process.Start(start)) child.WaitForExit();
        }
        if (Array.IndexOf(args, "--version") >= 0) { Console.WriteLine("2.0.0"); return; }
        if (Array.IndexOf(args, "auth") >= 0) { Console.WriteLine("{\"loggedIn\":true}"); return; }
        Console.WriteLine("Claude Code\r\nSafe mode:");
        while (true) {
            var line = Console.ReadLine();
            if (line == null) return;
            if (line.Contains("/usage")) {
                Console.WriteLine("Current session\r\n 23% used\r\n Resets 3pm\r\nCurrent week (all models)\r\n 45% used\r\n Resets Monday");
            }
        }
    }
}
'@ -OutputAssembly (Join-Path $fixture '.local/bin/claude.exe') -OutputType ConsoleApplication
    if ($AntigravityFixture) {
        $activeRuntime = Join-Path $fixture 'quota-helper'
        New-Item -ItemType Directory -Path $activeRuntime -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $runtime 'node.exe') -Destination $activeRuntime
        Copy-Item -LiteralPath (Join-Path $runtime 'quota-desktop.mjs') -Destination $activeRuntime
        Copy-Item -LiteralPath (Join-Path $runtime 'quota-smoke.mjs') -Destination $activeRuntime
        Add-Type -TypeDefinition @'
using System;
public class FakeAgy {
    public static void Main() {
        if (Environment.GetEnvironmentVariable("AGY_CLI_DISABLE_AUTO_UPDATE") != "true") {
            var start = new System.Diagnostics.ProcessStartInfo("cmd.exe", "/d /c start \"AgentMeter updater test\" /wait cmd /d /c ping -n 2 127.0.0.1");
            start.UseShellExecute = false;
            using (var child = System.Diagnostics.Process.Start(start)) child.WaitForExit();
        }
        Console.WriteLine("Gemini Models\tWeekly Limit Remaining\t70%\t2026-09-18T00:00:00Z");
        Console.WriteLine("Claude and GPT models\tFive Hour Limit Remaining\t80%\t2026-09-17T12:00:00Z");
    }
}
'@ -OutputAssembly (Join-Path $activeRuntime 'agy.exe') -OutputType ConsoleApplication
    }
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
public static class ConsoleWatch {
    delegate bool EnumCallback(IntPtr hwnd, IntPtr lparam);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, StringBuilder name, int size);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [StructLayout(LayoutKind.Sequential)] struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] static extern int GetSystemMetrics(int index);
    [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint access, bool inherit, uint id);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    [DllImport("ntdll.dll")] static extern int NtQueryInformationProcess(IntPtr handle, int kind, out ProcessBasicInformation info, int size, out int returned);
    [StructLayout(LayoutKind.Sequential)] struct ProcessBasicInformation {
        public IntPtr Reserved, Peb, Reserved2, Reserved3, Id, ParentId;
    }
    static string Ancestry(uint id) {
        var text = new StringBuilder();
        for (int depth = 0; id != 0 && depth < 5; depth++) {
            string name = "exited";
            try { using (var p = System.Diagnostics.Process.GetProcessById((int)id)) name = p.ProcessName; } catch {}
            text.Append(name + "(" + id + ") ");
            var handle = OpenProcess(0x1000, false, id);
            if (handle == IntPtr.Zero) break;
            try {
                ProcessBasicInformation info; int returned;
                if (NtQueryInformationProcess(handle, 0, out info, Marshal.SizeOf(typeof(ProcessBasicInformation)), out returned) != 0) break;
                id = (uint)info.ParentId.ToInt64();
            } finally { CloseHandle(handle); }
        }
        return text.ToString();
    }
    static HashSet<IntPtr> initial = new HashSet<IntPtr>();
    static HashSet<IntPtr> shown = new HashSet<IntPtr>();
    static List<string> details = new List<string>();
    static volatile bool running;
    static Thread worker;
    static HashSet<int> collectors = new HashSet<int>();
    static long nextProcessScan;
    static void CheckCollectors() {
        var now = System.Diagnostics.Stopwatch.GetTimestamp();
        if (now < nextProcessScan) return;
        nextProcessScan = now + System.Diagnostics.Stopwatch.Frequency / 10;
        foreach (var process in System.Diagnostics.Process.GetProcessesByName("node")) {
            using (process) {
                try {
                    if (process.MainModule.FileName.EndsWith("\\quota-helper\\node.exe", StringComparison.OrdinalIgnoreCase)) collectors.Add(process.Id);
                } catch { }
            }
        }
    }
    static IntPtr foreground;
    static void CheckForeground() {
        var current = GetForegroundWindow();
        if (current != foreground) {
            foreground = current;
            var name = new StringBuilder(256);
            GetClassName(current, name, name.Capacity);
            if ((name.ToString() == "ConsoleWindowClass" || name.ToString().StartsWith("CASCADIA" , StringComparison.OrdinalIgnoreCase)) && shown.Add(current)) {
                uint id; GetWindowThreadProcessId(current, out id);
                details.Add("console foreground class=" + name + " ancestry=" + Ancestry(id));
            }
        }
    }
    static void Scan(HashSet<IntPtr> into) {
        EnumWindows((hwnd, unused) => {
            var name = new StringBuilder(256);
            GetClassName(hwnd, name, name.Capacity);
            Rect rect;
            bool painted = GetWindowRect(hwnd, out rect) && rect.Right > rect.Left && rect.Bottom > rect.Top &&
                rect.Right > GetSystemMetrics(76) && rect.Bottom > GetSystemMetrics(77) &&
                rect.Left < GetSystemMetrics(76) + GetSystemMetrics(78) && rect.Top < GetSystemMetrics(77) + GetSystemMetrics(79);
            if (IsWindowVisible(hwnd) && painted && !initial.Contains(hwnd) && into.Add(hwnd) && running) {
                uint id; GetWindowThreadProcessId(hwnd, out id);
                string processName = "exited";
                try { processName = System.Diagnostics.Process.GetProcessById((int)id).ProcessName; } catch {}
                details.Add("class=" + name + " rect=" + rect.Left + "," + rect.Top + "," + rect.Right + "," + rect.Bottom + " pid=" + id + " process=" + processName + " ancestry=" + Ancestry(id));
            }
            return true;
        }, IntPtr.Zero);
    }
    public static void Start() {
        Scan(initial); foreground = GetForegroundWindow(); running = true;
        worker = new Thread(() => { while (running) { Scan(shown); CheckForeground(); CheckCollectors(); Thread.Sleep(1); } });
        worker.Start();
    }
    public static int Stop() { running = false; worker.Join(); return shown.Count; }
    public static string[] Details() { return details.ToArray(); }
    public static int BaselineCount() { return initial.Count; }
    public static int CollectorCount() { return collectors.Count; }
    public static bool HasVisibleProcess(int expected) {
        foreach (var hwnd in initial) { uint id; GetWindowThreadProcessId(hwnd, out id); if (id == expected) return true; }
        return false;
    }
}
'@
    [ConsoleWatch]::Start()
    Write-Host ("Visible baseline windows: " + [ConsoleWatch]::BaselineCount())
    $operationError = $null
    try {
        if ($ExpectedGuiProcessId -gt 0 -and -not [ConsoleWatch]::HasVisibleProcess($ExpectedGuiProcessId)) {
            throw 'The expected GUI is not visible on this desktop; observation cannot verify it.'
        }
        if ($ObserveSeconds -gt 0) {
            if ($ObserveSeconds -gt 180) { throw 'ObserveSeconds must not exceed 180.' }
            Start-Sleep -Seconds $ObserveSeconds
        } else {
        for ($attempt = 0; $attempt -lt $RepeatCount; $attempt++) {
            $start = New-Object Diagnostics.ProcessStartInfo
            $start.FileName = Join-Path $activeRuntime 'node.exe'
            $start.Arguments = '"' + (Join-Path $activeRuntime 'quota-desktop.mjs') + '" ' + $(if ($AntigravityFixture) { 'antigravity' } else { 'claude' }) + ' "' + $fixture + '"'
            if ($LiveProvider) {
                if (-not $LiveWorkdir -or -not [IO.Path]::IsPathRooted($LiveWorkdir)) { throw 'LiveWorkdir must be explicit and absolute.' }
                $start.Arguments = '"' + (Join-Path $runtime 'quota-desktop.mjs') + '" ' + $LiveProvider + ' "' + $LiveWorkdir + '"'
            }
            if ($InstalledExecutable) {
                $start.FileName = $InstalledExecutable
                $start.Arguments = '--quota-collect'
            }
            $start.WorkingDirectory = $activeRuntime
            $start.UseShellExecute = $false
            $start.CreateNoWindow = $true
            $start.RedirectStandardOutput = $true
            $start.RedirectStandardError = $true
            if (-not $LiveProvider -and -not $InstalledExecutable) { $start.EnvironmentVariables['USERPROFILE'] = $fixture }
            $start.EnvironmentVariables['AGENTMETER_TEST_VISIBLE_CHILD'] = $(if ($InjectVisibleGrandchild) { '1' } else { '0' })
            $process = [Diagnostics.Process]::Start($start)
            $output = $process.StandardOutput.ReadToEndAsync()
            $errors = $process.StandardError.ReadToEndAsync()
            if (-not $process.WaitForExit(40000)) { $process.Kill(); throw 'Fixture refresh timed out.' }
            $report = $output.Result | ConvertFrom-Json
            if ($process.ExitCode -ne 0 -or -not $report.results -or @($report.results | Where-Object { $_.status -ne 'PASS' }).Count) {
                throw 'Refresh did not complete every requested quota path.'
            }
            if (-not $LiveProvider -and -not $InstalledExecutable -and -not $AntigravityFixture -and
                ($report.results[0].quota.Count -ne 2 -or $report.results[0].quota[0].remaining_percent -ne 77)) {
                throw 'Credential-free fixture was not the selected provider.'
            }
            $process.Dispose()
        }
        }
    } catch {
        $operationError = $_
    } finally { $visible = [ConsoleWatch]::Stop() }
    [ConsoleWatch]::Details() | ForEach-Object { Write-Host $_ }
    Write-Host ("Bundled collector process IDs observed: " + [ConsoleWatch]::CollectorCount())
    if ($visible -ne 0) { throw "Observed $visible new/focused window(s); inspect class/PID provenance above." }
    if ($operationError) { throw $operationError }
    if ($ObserveSeconds -gt 0) {
        if ($ExpectedGuiProcessId -gt 0 -and [ConsoleWatch]::CollectorCount() -eq 0) {
            throw 'No bundled collector ran during the observation; refresh silence is unverified.'
        }
        Write-Host "PASS: $ObserveSeconds seconds of passive GUI observation; no new windows or console focus changes (1 ms sampling)."
    } else {
        Write-Host "PASS: $RepeatCount refreshes; no new windows or console focus changes (1 ms sampling)."
    }
} finally {
    $resolvedFixture = [IO.Path]::GetFullPath($fixture)
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolvedFixture.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase) -and
        [IO.Path]::GetFileName($resolvedFixture).StartsWith('agentmeter-console-')) {
        Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
    }
}
