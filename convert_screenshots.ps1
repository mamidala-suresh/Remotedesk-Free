Add-Type -AssemblyName System.Drawing
$dir = "C:\Users\ETP\.gemini\antigravity\brain\f112c352-7c64-4556-8203-e71800f9324f\.user_uploaded"
$outDir = "C:\Users\ETP\Downloads\Free Remote\docs_images"
if (!(Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
Get-ChildItem -Path $dir -Filter "*.png" | ForEach-Object {
    $bmp = New-Object System.Drawing.Bitmap($_.FullName)
    $outPath = Join-Path $outDir ($_.BaseName + ".jpg")
    $encoder = [System.Drawing.Imaging.ImageFormat]::Jpeg
    $bmp.Save($outPath, $encoder)
    Write-Host "$($_.Name) -> $($bmp.Width)x$($bmp.Height) -> $outPath"
    $bmp.Dispose()
}
