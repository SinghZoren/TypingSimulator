# Regenerate the shipping PNG and Windows ICO from the original artwork.
Add-Type -AssemblyName System.Drawing
$sourcePath = Join-Path $PSScriptRoot '..\assets\icon-source.png'
$pngPath = Join-Path $PSScriptRoot '..\assets\icon.png'
$icoPath = Join-Path $PSScriptRoot '..\assets\icon.ico'
$source = [System.Drawing.Image]::FromFile($sourcePath)

function Get-ResizedPng([int]$size) {
    $bitmap = [System.Drawing.Bitmap]::new($size, $size)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.DrawImage($source, 0, 0, $size, $size)
    $graphics.Dispose()
    $memory = [System.IO.MemoryStream]::new()
    $bitmap.Save($memory, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $bytes = $memory.ToArray()
    $memory.Dispose()
    return ,$bytes
}

try {
    [System.IO.File]::WriteAllBytes($pngPath, (Get-ResizedPng 1024))
    $sizes = @(16, 32, 48, 256)
    $images = @($sizes | ForEach-Object { Get-ResizedPng $_ })
    $stream = [System.IO.File]::Create($icoPath)
    $writer = [System.IO.BinaryWriter]::new($stream)
    try {
        $writer.Write([uint16]0)
        $writer.Write([uint16]1)
        $writer.Write([uint16]$sizes.Count)
        $offset = 6 + 16 * $sizes.Count
        for ($i = 0; $i -lt $sizes.Count; $i++) {
            $writer.Write([byte]($sizes[$i] % 256))
            $writer.Write([byte]($sizes[$i] % 256))
            $writer.Write([byte]0)
            $writer.Write([byte]0)
            $writer.Write([uint16]1)
            $writer.Write([uint16]32)
            $writer.Write([uint32]$images[$i].Length)
            $writer.Write([uint32]$offset)
            $offset += $images[$i].Length
        }
        foreach ($bytes in $images) { $writer.Write([byte[]]$bytes) }
    } finally {
        $writer.Dispose()
    }
} finally {
    $source.Dispose()
}
