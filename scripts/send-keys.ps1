# Presses keys through SendInput, for checking the launcher by hand on Windows.
# The keys go to whatever window is in front, as a real press would.
#
#   send-keys.ps1 alt+space 'text:fire' wait:300 ctrl+k escape alt+tab
#
# Steps run in order: a chord (modifiers alt, ctrl, shift, win joined with +),
# `text:<string>` to type characters, `wait:<ms>`, or `down:<key>` and
# `up:<key>` to hold a key across steps or calls. 50 ms pass between steps.
param([Parameter(Mandatory, ValueFromRemainingArguments)][string[]]$Steps)

$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class DangoKeys {
    [StructLayout(LayoutKind.Sequential)]
    struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)]
    struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)]
    struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)]
    struct INPUT { public uint type; public InputUnion u; }

    [DllImport("user32.dll", SetLastError = true)]
    static extern uint SendInput(uint count, INPUT[] inputs, int size);

    const uint KEYUP = 0x2, UNICODE = 0x4;

    static INPUT Key(ushort vk, ushort scan, uint flags) {
        var input = new INPUT { type = 1 };
        input.u.ki = new KEYBDINPUT { wVk = vk, wScan = scan, dwFlags = flags };
        return input;
    }

    static void Send(INPUT[] inputs) {
        if (SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(INPUT))) != inputs.Length)
            throw new InvalidOperationException("SendInput was refused, error " + Marshal.GetLastWin32Error());
    }

    public static void Chord(ushort[] modifiers, ushort key) {
        var inputs = new System.Collections.Generic.List<INPUT>();
        foreach (var m in modifiers) inputs.Add(Key(m, 0, 0));
        inputs.Add(Key(key, 0, 0));
        inputs.Add(Key(key, 0, KEYUP));
        for (int i = modifiers.Length - 1; i >= 0; i--) inputs.Add(Key(modifiers[i], 0, KEYUP));
        Send(inputs.ToArray());
    }

    public static void Press(ushort key, bool up) {
        Send(new[] { Key(key, 0, up ? KEYUP : 0) });
    }

    public static void Text(string text) {
        foreach (char c in text) Send(new[] { Key(0, c, UNICODE), Key(0, c, UNICODE | KEYUP) });
    }
}
'@

$modifiers = @{ alt = 0x12; ctrl = 0x11; shift = 0x10; win = 0x5B }
$named = @{
    space = 0x20; enter = 0x0D; escape = 0x1B; esc = 0x1B; tab = 0x09; backspace = 0x08
    delete = 0x2E; up = 0x26; down = 0x28; left = 0x25; right = 0x27; home = 0x24; end = 0x23
    pageup = 0x21; pagedown = 0x22
}

function Resolve-Key([string]$name) {
    if ($named.ContainsKey($name)) { return $named[$name] }
    if ($name -match '^[a-z0-9]$') { return [int][char]$name.ToUpper() }
    if ($name -match '^f([1-9]|1[0-2])$') { return 0x6F + [int]$Matches[1] }
    throw "Unknown key '$name'"
}

foreach ($step in $Steps) {
    if ($step -like 'text:*') {
        [DangoKeys]::Text($step.Substring(5))
    } elseif ($step -match '^(down|up):(.+)$') {
        $name = $Matches[2].ToLower()
        $key = if ($modifiers.ContainsKey($name)) { $modifiers[$name] } else { Resolve-Key $name }
        [DangoKeys]::Press([uint16]$key, $Matches[1] -eq 'up')
    } elseif ($step -like 'wait:*') {
        Start-Sleep -Milliseconds ([int]$step.Substring(5))
    } else {
        $parts = $step.ToLower() -split '\+'
        # 0..-1 counts down in PowerShell, so a lone key needs no range at all.
        $names = if ($parts.Length -gt 1) { $parts[0..($parts.Length - 2)] } else { @() }
        $held = [uint16[]]@($names | ForEach-Object {
            if (-not $modifiers.ContainsKey($_)) { throw "Unknown modifier '$_'" }
            $modifiers[$_]
        })
        [DangoKeys]::Chord($held, [uint16](Resolve-Key $parts[-1]))
    }
    Start-Sleep -Milliseconds 50
}
