$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$iconSource = 'C:\Users\richs\.codex\generated_images\01a08d89-4ae2-7d01-9ed8-4e3deeb2f2c3\exec-83282fa6-3e89-44dc-a48e-3389a03675ec.png'
$emptyStateSource = 'C:\Users\richs\.codex\generated_images\01a08d89-4ae2-7d01-9ed8-4e3deeb2f2c3\exec-5260251e-bb53-4432-a913-95dd1565c74e.png'
$desktopAssets = 'D:\WorkSpace\AgentMeter\desktop-p0\ui\assets'
$tabletAssets = 'D:\WorkSpace\AgentMeter\tablet-ui\assets'
$nativeIcons = 'D:\WorkSpace\AgentMeter\desktop-p0\icons'

New-Item -ItemType Directory -Force -Path $desktopAssets, $tabletAssets, $nativeIcons | Out-Null
function Export-SquarePng([string]$Source, [string]$Destination, [int]$Size) {
    $sourceImage = [System.Drawing.Image]::FromFile($Source)
    try {
        $bitmap = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        try {
            $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
            try {
                $graphics.Clear([System.Drawing.Color]::Transparent)
                $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
                $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
                $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
                $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
                $graphics.DrawImage($sourceImage, 0, 0, $Size, $Size)
            } finally {
                $graphics.Dispose()
            }
            $bitmap.Save($Destination, [System.Drawing.Imaging.ImageFormat]::Png)
        } finally {
            $bitmap.Dispose()
        }
    } finally {
        $sourceImage.Dispose()
    }
}

Export-SquarePng $iconSource (Join-Path $desktopAssets 'agentmeter-icon.png') 256
Export-SquarePng $iconSource (Join-Path $tabletAssets 'agentmeter-icon.png') 256
Export-SquarePng $iconSource (Join-Path $nativeIcons 'icon.png') 256
Export-SquarePng $emptyStateSource (Join-Path $desktopAssets 'empty-cloud.png') 256
Export-SquarePng $emptyStateSource (Join-Path $tabletAssets 'empty-cloud.png') 256

$providerSources = @{
    codex = 'C:\Users\richs\.codex\generated_images\01a08d89-4ae2-7d01-9ed8-4e3deeb2f2c3\exec-7da2cf87-bf76-4397-90d4-af9dbd83af8f.png'
    claude = 'C:\Users\richs\.codex\generated_images\01a08d89-4ae2-7d01-9ed8-4e3deeb2f2c3\exec-2fe35408-ae51-4d43-a09d-eb6a0915895c.png'
    copilot = 'C:\Users\richs\.codex\generated_images\01a08d89-4ae2-7d01-9ed8-4e3deeb2f2c3\exec-b48b78b6-cc8f-46e7-b869-1b0ef1af57ff.png'
    antigravity = 'C:\Users\richs\.codex\generated_images\01a08d89-4ae2-7d01-9ed8-4e3deeb2f2c3\exec-0c9f572c-f758-4727-b61f-4f53677f6665.png'
}
foreach ($provider in $providerSources.Keys) {
    Export-SquarePng $providerSources[$provider] (Join-Path $desktopAssets "$provider-icon.png") 192
    Export-SquarePng $providerSources[$provider] (Join-Path $tabletAssets "$provider-icon.png") 192
}

$iconPng = [System.IO.File]::ReadAllBytes((Join-Path $nativeIcons 'icon.png'))
$iconHeader = [byte[]](0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 32, 0)
$iconSize = [BitConverter]::GetBytes([int]$iconPng.Length)
$iconOffset = [BitConverter]::GetBytes([int]22)
$iconBytes = New-Object byte[] (22 + $iconPng.Length)
[Array]::Copy($iconHeader, 0, $iconBytes, 0, $iconHeader.Length)
[Array]::Copy($iconSize, 0, $iconBytes, 14, 4)
[Array]::Copy($iconOffset, 0, $iconBytes, 18, 4)
[Array]::Copy($iconPng, 0, $iconBytes, 22, $iconPng.Length)
[System.IO.File]::WriteAllBytes((Join-Path $nativeIcons 'icon.ico'), $iconBytes)

$trayPng = Join-Path $nativeIcons 'tray-icon.png'
Export-SquarePng $iconSource $trayPng 32
$trayBitmap = [System.Drawing.Bitmap]::FromFile($trayPng)
try {
    $rgba = New-Object byte[] (32 * 32 * 4)
    for ($y = 0; $y -lt 32; $y++) {
        for ($x = 0; $x -lt 32; $x++) {
            $pixel = $trayBitmap.GetPixel($x, $y)
            $offset = (($y * 32) + $x) * 4
            $rgba[$offset] = $pixel.R
            $rgba[$offset + 1] = $pixel.G
            $rgba[$offset + 2] = $pixel.B
            $rgba[$offset + 3] = $pixel.A
        }
    }
    [System.IO.File]::WriteAllBytes((Join-Path $nativeIcons 'tray-icon.rgba'), $rgba)
} finally {
    $trayBitmap.Dispose()
}

$alphaCheck = [System.Drawing.Bitmap]::FromFile((Join-Path $nativeIcons 'icon.png'))
try {
    Write-Output "icon=$($alphaCheck.Width)x$($alphaCheck.Height) cornerAlpha=$($alphaCheck.GetPixel(0,0).A)"
} finally {
    $alphaCheck.Dispose()
}
