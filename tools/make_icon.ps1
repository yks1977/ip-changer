# IP Changer 아이콘 생성 스크립트
#
# assets\ip-changer.ico        : exe에 심는 멀티 사이즈 아이콘 (16~256)
# assets\ip-changer-64.rgba    : egui 창 아이콘용 64x64 RGBA 원본
# assets\ip-changer-preview.png: 확인용 256x256 미리보기
#
# 사용법: powershell -ExecutionPolicy Bypass -File tools\make_icon.ps1

Add-Type -AssemblyName System.Drawing

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$assets = Join-Path $root 'assets'
New-Item -ItemType Directory -Force -Path $assets | Out-Null

# 앱 테마에 맞춘 색
$bgTop = [System.Drawing.Color]::FromArgb(255, 58, 64, 72)      # 3a4048
$bgBottom = [System.Drawing.Color]::FromArgb(255, 23, 25, 29)   # 17191d
$gold = [System.Drawing.Color]::FromArgb(255, 224, 183, 110)    # e0b76e
$goldDeep = [System.Drawing.Color]::FromArgb(255, 196, 152, 82) # c49852
$white = [System.Drawing.Color]::FromArgb(255, 247, 248, 250)   # f7f8fa

function New-RoundedPath {
    param([single]$x, [single]$y, [single]$w, [single]$h, [single]$r)
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $d = $r * 2
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc(($x + $w - $d), $y, $d, $d, 270, 90)
    $path.AddArc(($x + $w - $d), ($y + $h - $d), $d, $d, 0, 90)
    $path.AddArc($x, ($y + $h - $d), $d, $d, 90, 90)
    $path.CloseFigure()
    return $path
}

function New-IconBitmap {
    $size = 256

    $bmp = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear([System.Drawing.Color]::Transparent)

    # 배경: 둥근 사각형 + 세로 그라데이션
    $bg = New-RoundedPath 6 6 244 244 56
    $top = [System.Drawing.PointF]::new(0, 0)
    $bottom = [System.Drawing.PointF]::new(0, 256)
    $brush = [System.Drawing.Drawing2D.LinearGradientBrush]::new($top, $bottom, $bgTop, $bgBottom)
    $g.FillPath($brush, $bg)
    $brush.Dispose()

    # 위쪽 은은한 광택
    $gloss = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        [System.Drawing.PointF]::new(0, 0),
        [System.Drawing.PointF]::new(0, 150),
        [System.Drawing.Color]::FromArgb(26, 255, 255, 255),
        [System.Drawing.Color]::FromArgb(0, 255, 255, 255)
    )
    $g.FillPath($gloss, $bg)
    $gloss.Dispose()

    # 테두리 (살짝 밝게)
    $edge = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(34, 255, 255, 255), 2)
    $g.DrawPath($edge, $bg)
    $edge.Dispose()
    $bg.Dispose()

    # "IP" 워드마크
    $font = [System.Drawing.Font]::new('Segoe UI', 104, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $fmt = [System.Drawing.StringFormat]::new()
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center
    $fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
    $textRect = [System.Drawing.RectangleF]::new(0, 24, 256, 132)
    $textBrush = [System.Drawing.SolidBrush]::new($white)
    $g.DrawString('IP', $font, $textBrush, $textRect, $fmt)
    $textBrush.Dispose()
    $fmt.Dispose()
    $font.Dispose()

    # 교체 화살표 두 개 (위: 오른쪽, 아래: 왼쪽)
    $shaftBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        [System.Drawing.PointF]::new(50, 166),
        [System.Drawing.PointF]::new(210, 166),
        $gold,
        $goldDeep
    )

    $shaft = New-RoundedPath 50 170 136 14 7
    $g.FillPath($shaftBrush, $shaft)
    $shaft.Dispose()

    $head = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $head.AddPolygon([System.Drawing.PointF[]]@(
        [System.Drawing.PointF]::new(184, 162),
        [System.Drawing.PointF]::new(212, 177),
        [System.Drawing.PointF]::new(184, 192)
    ))
    $g.FillPath($shaftBrush, $head)
    $head.Dispose()

    $shaft2 = New-RoundedPath 70 194 136 14 7
    $g.FillPath($shaftBrush, $shaft2)
    $shaft2.Dispose()

    $head2 = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $head2.AddPolygon([System.Drawing.PointF[]]@(
        [System.Drawing.PointF]::new(72, 186),
        [System.Drawing.PointF]::new(44, 201),
        [System.Drawing.PointF]::new(72, 216)
    ))
    $g.FillPath($shaftBrush, $head2)
    $head2.Dispose()
    $shaftBrush.Dispose()

    $g.Dispose()
    return $bmp
}

function Save-Png {
    param([System.Drawing.Bitmap]$Bitmap, [string]$Path)
    $Bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
}

function ConvertTo-IconPng {
    param([System.Drawing.Bitmap]$Bitmap)
    $ms = [System.IO.MemoryStream]::new()
    $Bitmap.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    return $ms.ToArray()
}

$master = New-IconBitmap
$sizes = @(16, 24, 32, 48, 64, 128, 256)
$frames = @{}

foreach ($s in $sizes) {
    if ($s -eq 256) {
        $frames[$s] = ConvertTo-IconPng $master
        continue
    }
    $small = [System.Drawing.Bitmap]::new($s, $s, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $sg = [System.Drawing.Graphics]::FromImage($small)
    $sg.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $sg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $sg.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $sg.Clear([System.Drawing.Color]::Transparent)
    $sg.DrawImage($master, [System.Drawing.Rectangle]::new(0, 0, $s, $s))
    $sg.Dispose()
    $frames[$s] = ConvertTo-IconPng $small
    $small.Dispose()
}

# ICO 컨테이너 작성 (PNG 압축 프레임)
$icoPath = Join-Path $assets 'ip-changer.ico'
$ico = [System.IO.MemoryStream]::new()
$writer = [System.IO.BinaryWriter]::new($ico)
$writer.Write([uint16]0)                 # reserved
$writer.Write([uint16]1)                 # type: icon
$writer.Write([uint16]$sizes.Count)

$offset = 6 + (16 * $sizes.Count)
foreach ($s in $sizes) {
    $data = $frames[$s]
    $dim = if ($s -ge 256) { 0 } else { $s }
    $writer.Write([byte]$dim)            # width
    $writer.Write([byte]$dim)            # height
    $writer.Write([byte]0)               # palette
    $writer.Write([byte]0)               # reserved
    $writer.Write([uint16]1)             # planes
    $writer.Write([uint16]32)            # bpp
    $writer.Write([uint32]$data.Length)
    $writer.Write([uint32]$offset)
    $offset += $data.Length
}
foreach ($s in $sizes) {
    $writer.Write([byte[]]$frames[$s])
}
$writer.Flush()
[System.IO.File]::WriteAllBytes($icoPath, $ico.ToArray())
$ico.Dispose()

# egui 창 아이콘용 64x64 RGBA (GDI+는 BGRA 순서라 R/B를 바꾼다)
$icon64 = [System.Drawing.Bitmap]::new(64, 64, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g64 = [System.Drawing.Graphics]::FromImage($icon64)
$g64.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$g64.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g64.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$g64.Clear([System.Drawing.Color]::Transparent)
$g64.DrawImage($master, [System.Drawing.Rectangle]::new(0, 0, 64, 64))
$g64.Dispose()

$rect64 = [System.Drawing.Rectangle]::new(0, 0, 64, 64)
$locked = $icon64.LockBits($rect64, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$raw = [byte[]]::new($locked.Stride * 64)
[System.Runtime.InteropServices.Marshal]::Copy($locked.Scan0, $raw, 0, $raw.Length)
$icon64.UnlockBits($locked)
for ($i = 0; $i -lt $raw.Length; $i += 4) {
    $b = $raw[$i]
    $raw[$i] = $raw[$i + 2]
    $raw[$i + 2] = $b
}
[System.IO.File]::WriteAllBytes((Join-Path $assets 'ip-changer-64.rgba'), $raw)
$icon64.Dispose()

# 미리보기
Save-Png $master (Join-Path $assets 'ip-changer-preview.png')

# 크기별 미리보기 시트 (실제 크기로 그려서 작은 아이콘의 가독성을 확인)
$sheet = [System.Drawing.Bitmap]::new(720, 320, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$sg = [System.Drawing.Graphics]::FromImage($sheet)
$sg.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$sg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$sg.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$sg.Clear([System.Drawing.Color]::White)
$sg.DrawImage($master, [System.Drawing.Rectangle]::new(24, 32, 256, 256))

$labelFont = [System.Drawing.Font]::new('Segoe UI', 11, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$labelBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 107, 114, 128))
$sg.DrawString('256 px', $labelFont, $labelBrush, 24, 292)

$sheetSizes = @(16, 24, 32, 48, 64)
$x = 330
$y = 150
foreach ($s in $sheetSizes) {
    $sg.DrawImage($master, [System.Drawing.Rectangle]::new($x, $y, $s, $s))
    $sg.DrawString("$s", $labelFont, $labelBrush, ($x + ($s / 2) - 6), 180)
    $x += $s + 26
}
$sg.DrawString('toolbar / list sizes', $labelFont, $labelBrush, 330, 210)

# 어두운 배경에서도 확인
$darkRect = [System.Drawing.Rectangle]::new(330, 40, 360, 90)
$darkBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 32, 34, 38))
$sg.FillRectangle($darkBrush, $darkRect)
$darkBrush.Dispose()
$x = 344
$y = 68
foreach ($s in $sheetSizes) {
    $sg.DrawImage($master, [System.Drawing.Rectangle]::new($x, $y, $s, $s))
    $x += $s + 18
}
$sg.Dispose()
$sheet.Save((Join-Path $assets 'ip-changer-sizes.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$sheet.Dispose()
$labelFont.Dispose()
$labelBrush.Dispose()

$master.Dispose()

Write-Output "icon written: $icoPath"
Write-Output ("sizes: " + ($sizes -join ', '))
