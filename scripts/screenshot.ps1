# Captures every display into one PNG and prints its path. For checking the
# launcher by hand on Windows: placement, focus, and what it draws.
param(
    [string]$Path = (Join-Path $env:TEMP "dango-shots\shot-$(Get-Date -Format 'yyyyMMdd-HHmmss-fff').png")
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -Namespace Dango -Name Dpi -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
'@
# Without this a scaled display is captured at its logical size.
[void][Dango.Dpi]::SetProcessDPIAware()

$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    New-Item -ItemType Directory -Force -Path (Split-Path $Path) | Out-Null
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $graphics.Dispose()
    $bitmap.Dispose()
}
Write-Output $Path
