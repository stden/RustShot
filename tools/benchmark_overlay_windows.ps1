#requires -Version 7.0
<#
.SYNOPSIS
Measures overlay CPU time under rapid cursor movement and checks all screen edges.
.DESCRIPTION
Opens a temporary selection overlay and moves the cursor on every monitor.
Restores the cursor and closes only the benchmark's child process afterward.
Run on an interactive Windows desktop without moving the mouse during sampling.
.EXAMPLE
rtk proxy pwsh -NoProfile -File tools/benchmark_overlay_windows.ps1 -Iterations 3
.EXAMPLE
rtk proxy pwsh -NoProfile -File tools/benchmark_overlay_windows.ps1 -ExecutablePath C:/Windows/RustShot.exe -CsvPath target/overlay-baseline.csv
#>
[CmdletBinding()]
param(
    [string]$ExecutablePath = (Join-Path $PSScriptRoot '../target/release/RustShot.exe'),
    [ValidateRange(1, 100)][int]$Iterations = 3,
    [ValidateRange(1, 100)][int]$InputIntervalMs = 2,
    [string]$CsvPath
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'This benchmark requires an interactive Windows desktop.' }
$ExecutablePath = (Resolve-Path -LiteralPath $ExecutablePath).Path

if (-not ('RustShotOverlayBenchmark' -as [type])) {
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;

public static class RustShotOverlayBenchmark {
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct MonitorInfo { public int Size; public Rect Monitor, Work; public uint Flags; }
    public sealed class Sample {
        public int Width, Height, VisibleSamples, TotalSamples, InputEvents;
        public double CpuMilliseconds, ElapsedMilliseconds;
    }
    public delegate bool EnumProc(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback, IntPtr parameter);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll")] static extern bool GetMonitorInfoW(IntPtr monitor, ref MonitorInfo info);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] static extern IntPtr GetDC(IntPtr window);
    [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr window, IntPtr dc);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("gdi32.dll")] static extern uint GetPixel(IntPtr dc, int x, int y);
    [DllImport("gdi32.dll")] static extern IntPtr CreateCompatibleDC(IntPtr dc);
    [DllImport("gdi32.dll")] static extern IntPtr CreateDIBSection(IntPtr dc, int[] info, uint usage, out IntPtr bits, IntPtr section, uint offset);
    [DllImport("gdi32.dll")] static extern IntPtr SelectObject(IntPtr dc, IntPtr item);
    [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr item);
    [DllImport("gdi32.dll")] static extern bool DeleteDC(IntPtr dc);
    [DllImport("gdi32.dll")] static extern bool GdiFlush();
    [DllImport("gdi32.dll")] static extern bool BitBlt(IntPtr destination, int x, int y, int width, int height, IntPtr source, int sx, int sy, uint rop);

    public static IntPtr[] Windows(uint processId) {
        var found = new List<IntPtr>();
        EnumWindows((window, parameter) => {
            uint process; Rect rect;
            GetWindowThreadProcessId(window, out process);
            if (process == processId && GetClientRect(window, out rect) && rect.Right > 100 && rect.Bottom > 100)
                found.Add(window);
            return true;
        }, IntPtr.Zero);
        return found.ToArray();
    }

    static bool EdgesAreRed(IntPtr dc, int width, int height) {
        int x = width - 2, y = height - 2;
        return GetPixel(dc, 0, y) == 255 && GetPixel(dc, width - 1, y) == 255 &&
            GetPixel(dc, x, 0) == 255 && GetPixel(dc, x, height - 1) == 255;
    }

    public static Sample Measure(IntPtr window, int inputIntervalMs) {
        Rect rect, bounds;
        if (!GetClientRect(window, out rect) || !GetWindowRect(window, out bounds)) throw new Exception("Window geometry unavailable");
        var info = new MonitorInfo { Size = Marshal.SizeOf<MonitorInfo>() };
        if (!GetMonitorInfoW(MonitorFromWindow(window, 2), ref info)) throw new Exception("Monitor geometry unavailable");
        if (bounds.Left != info.Monitor.Left || bounds.Top != info.Monitor.Top || bounds.Right != info.Monitor.Right || bounds.Bottom != info.Monitor.Bottom)
            throw new Exception("Overlay does not cover its physical monitor");

        IntPtr dc = GetDC(window), memory = IntPtr.Zero, bitmap = IntPtr.Zero, previous = IntPtr.Zero;
        using (var cancellation = new CancellationTokenSource()) {
            Task sender = null;
            try {
                if (dc == IntPtr.Zero) throw new Exception("Window DC unavailable");
                SetCursorPos(bounds.Right - 2, bounds.Bottom - 2);
                var settling = Stopwatch.StartNew();
                while (!EdgesAreRed(dc, rect.Right, rect.Bottom) && settling.ElapsedMilliseconds < 1500) Thread.Sleep(10);
                if (!EdgesAreRed(dc, rect.Right, rect.Bottom)) throw new Exception("Crosshair does not reach every screen edge");

                memory = CreateCompatibleDC(dc);
                IntPtr bits;
                bitmap = CreateDIBSection(memory, new int[] {40, rect.Right, -1, 1 | (32 << 16), 0, 0, 0, 0, 0, 0, 0}, 0, out bits, IntPtr.Zero, 0);
                if (memory == IntPtr.Zero || bitmap == IntPtr.Zero || bits == IntPtr.Zero) throw new Exception("Snapshot allocation failed");
                previous = SelectObject(memory, bitmap);
                if (previous == IntPtr.Zero || previous == new IntPtr(-1)) throw new Exception("Snapshot bitmap selection failed");
                int[] row = new int[rect.Right];
                uint processId; GetWindowThreadProcessId(window, out processId);
                using (var process = Process.GetProcessById((int)processId)) {
                    double cpuBefore = process.TotalProcessorTime.TotalMilliseconds;
                    var elapsed = Stopwatch.StartNew();
                    int inputEvents = 0;
                    sender = Task.Run(() => {
                        IntPtr previousContext = SetThreadDpiAwarenessContext(new IntPtr(-4));
                        try {
                        var burst = Stopwatch.StartNew();
                        int step = 0;
                        long nextInputAt = 0;
                        long inputTicks = Math.Max(1, Stopwatch.Frequency * inputIntervalMs / 1000);
                        while (!cancellation.IsCancellationRequested && burst.ElapsedMilliseconds < 1000) {
                            // Sleep(2) can become a full Windows timer tick. Pace
                            // this separate driver process with the monotonic clock.
                            if (burst.ElapsedTicks < nextInputAt) { Thread.SpinWait(32); continue; }
                            int x = (++step % 2 == 0) ? rect.Right / 3 : 2 * rect.Right / 3;
                            if (!SetCursorPos(bounds.Left + x, bounds.Top + rect.Bottom / 2)) throw new Exception("Cursor movement failed");
                            inputEvents++;
                            nextInputAt = burst.ElapsedTicks + inputTicks;
                        }
                        } finally { SetThreadDpiAwarenessContext(previousContext); }
                    });
                    Thread.Sleep(40);
                    const int samples = 64;
                    int visible = 0;
                    for (int sample = 0; sample < samples; sample++) {
                        if (!BitBlt(memory, 0, 0, rect.Right, 1, dc, 0, 10, 0x00CC0020)) throw new Exception("Snapshot failed");
                        GdiFlush();
                        Marshal.Copy(bits, row, 0, row.Length);
                        if (Array.Exists(row, pixel => (pixel & 0x00ffffff) == 0x00ff0000)) visible++;
                        Thread.Sleep(5);
                    }
                    sender.Wait();
                    Thread.Sleep(100); // Allow the final pending display frame to arrive.
                    Point cursor; GetCursorPos(out cursor);
                    int finalX = cursor.X - bounds.Left;
                    if (GetPixel(dc, finalX, 0) != 255 || GetPixel(dc, finalX, rect.Bottom - 1) != 255)
                        throw new Exception("The final cursor position was not presented after input stopped");
                    process.Refresh();
                    return new Sample { Width = rect.Right, Height = rect.Bottom,
                        CpuMilliseconds = process.TotalProcessorTime.TotalMilliseconds - cpuBefore,
                        ElapsedMilliseconds = elapsed.Elapsed.TotalMilliseconds,
                        VisibleSamples = visible, TotalSamples = samples, InputEvents = inputEvents };
                }
            } finally {
                cancellation.Cancel();
                try { if (sender != null) sender.GetAwaiter().GetResult(); }
                finally {
                    GdiFlush();
                    if (previous != IntPtr.Zero && previous != new IntPtr(-1)) SelectObject(memory, previous);
                    if (bitmap != IntPtr.Zero) DeleteObject(bitmap);
                    if (memory != IntPtr.Zero) DeleteDC(memory);
                    if (dc != IntPtr.Zero) ReleaseDC(window, dc);
                }
            }
        }
    }
}
'@
}

$originalContext = [RustShotOverlayBenchmark]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
$originalCursor = [RustShotOverlayBenchmark+Point]::new()
if (-not [RustShotOverlayBenchmark]::GetCursorPos([ref]$originalCursor)) { throw 'Cursor position unavailable.' }
$results = [System.Collections.Generic.List[object]]::new()
try {
    for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
        $selection = Start-Process -FilePath $ExecutablePath -ArgumentList '--select' -WorkingDirectory (Split-Path -Parent $ExecutablePath) -WindowStyle Hidden -PassThru
        try {
            $windows = @()
            for ($attempt = 0; $attempt -lt 50; $attempt++) {
                Start-Sleep -Milliseconds 100
                $windows = [RustShotOverlayBenchmark]::Windows([uint32]$selection.Id)
                if ($windows.Count -gt 0) { break }
                if ($selection.HasExited) { throw 'Selection exited before an overlay appeared.' }
            }
            if ($windows.Count -eq 0) { throw 'No selection overlay found.' }
            Start-Sleep -Milliseconds 500
            $windows = [RustShotOverlayBenchmark]::Windows([uint32]$selection.Id)
            foreach ($window in $windows) { [void][RustShotOverlayBenchmark]::ShowWindow($window, 4) }
            Start-Sleep -Milliseconds 100
            foreach ($window in $windows) {
                $sample = [RustShotOverlayBenchmark]::Measure($window, $InputIntervalMs)
                if ($sample.VisibleSamples -ne $sample.TotalSamples) { throw "Crosshair visible in only $($sample.VisibleSamples)/$($sample.TotalSamples) samples." }
                $result = [pscustomobject]@{
                    Iteration = $iteration; Resolution = "$($sample.Width)x$($sample.Height)"
                    InputIntervalMs = $InputIntervalMs
                    InputEvents = $sample.InputEvents
                    CpuMilliseconds = $sample.CpuMilliseconds
                    ElapsedMilliseconds = $sample.ElapsedMilliseconds
                    VisibleSamples = $sample.VisibleSamples; TotalSamples = $sample.TotalSamples
                }
                $results.Add($result)
                $result
            }
        } finally {
            if (-not $selection.HasExited) {
                foreach ($window in [RustShotOverlayBenchmark]::Windows([uint32]$selection.Id)) {
                    [void][RustShotOverlayBenchmark]::PostMessageW($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
                }
                if (-not $selection.WaitForExit(5000)) { Stop-Process -Id $selection.Id }
            }
            $selection.Dispose()
        }
    }
} finally {
    [void][RustShotOverlayBenchmark]::SetCursorPos($originalCursor.X, $originalCursor.Y)
    [void][RustShotOverlayBenchmark]::SetThreadDpiAwarenessContext($originalContext)
    # Preserve completed measurements even when a later correctness check fails.
    if ($CsvPath -and $results.Count -gt 0) { $results | Export-Csv -LiteralPath $CsvPath -NoTypeInformation }
}
