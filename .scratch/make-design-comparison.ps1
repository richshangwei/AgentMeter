$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

function Save-SideBySide([string]$SourcePath, [string]$ImplementationPath, [string]$DestinationPath, [System.Drawing.Rectangle]$Crop) {
    $source = [System.Drawing.Image]::FromFile($SourcePath)
    $implementation = [System.Drawing.Image]::FromFile($ImplementationPath)
    try {
        $width = $Crop.Width
        $height = $Crop.Height
        $canvas = [System.Drawing.Bitmap]::new(($width * 2), $height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
        try {
            $graphics = [System.Drawing.Graphics]::FromImage($canvas)
            try {
                $graphics.Clear([System.Drawing.Color]::FromArgb(2, 13, 25))
                $graphics.DrawImage($source, [System.Drawing.Rectangle]::new(0, 0, $width, $height), $Crop, [System.Drawing.GraphicsUnit]::Pixel)
                $graphics.DrawImage($implementation, [System.Drawing.Rectangle]::new($width, 0, $width, $height), $Crop, [System.Drawing.GraphicsUnit]::Pixel)
            } finally {
                $graphics.Dispose()
            }
            $canvas.Save($DestinationPath, [System.Drawing.Imaging.ImageFormat]::Png)
        } finally {
            $canvas.Dispose()
        }
    } finally {
        $source.Dispose()
        $implementation.Dispose()
    }
}

$sourcePath = 'C:\Users\richs\AppData\Local\Temp\codex-clipboard-f275fc64-32de-4b85-8dee-590c7ef96a62.png'
$implementationPath = 'D:\WorkSpace\AgentMeter\.scratch\desktop-reference-size.png'
Save-SideBySide $sourcePath $implementationPath 'D:\WorkSpace\AgentMeter\.scratch\design-comparison-desktop.png' ([System.Drawing.Rectangle]::new(0, 0, 1452, 1086))
Save-SideBySide $sourcePath $implementationPath 'D:\WorkSpace\AgentMeter\.scratch\design-comparison-card.png' ([System.Drawing.Rectangle]::new(24, 112, 690, 440))

$ultrawideSourcePath = 'C:\Users\richs\AppData\Local\Temp\codex-clipboard-c38525b2-97dc-4229-ac49-d6061f923462.png'
$ultrawideImplementationPath = 'D:\WorkSpace\AgentMeter\.scratch\desktop-ultrawide.png'
Save-SideBySide $ultrawideSourcePath $ultrawideImplementationPath 'D:\WorkSpace\AgentMeter\.scratch\design-comparison-ultrawide.png' ([System.Drawing.Rectangle]::new(0, 0, 2491, 1312))

$percentageSourcePath = 'C:\Users\richs\AppData\Local\Temp\codex-clipboard-4814636e-b166-429a-a44b-f2cc76718a09.png'
$percentageImplementationPath = 'D:\WorkSpace\AgentMeter\.scratch\desktop-percentage-fixed.png'
Save-SideBySide $percentageSourcePath $percentageImplementationPath 'D:\WorkSpace\AgentMeter\.scratch\design-comparison-percentage.png' ([System.Drawing.Rectangle]::new(0, 0, 2440, 1288))
