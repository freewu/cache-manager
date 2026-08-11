# Cache Manager release script (invoked by `just release`)
# 1) copy exe -> release/CacheManager-<version>.exe
# 2) pack source -> release/source-<version>.tar.gz (exclude node_modules/target/.git/release/dist/gen)
# 3) generate .md5 / .sha1 hash files for both artifacts
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# read version from Cargo.toml
$version = ((Select-String -Path "src-tauri/Cargo.toml" '^version' | Select-Object -First 1).Line -replace '.*"([^"]*)".*', '$1').Trim()
if (-not $version) { throw "cannot parse version from src-tauri/Cargo.toml" }
Write-Host "[release] version: $version"

# 1) exe
$exeSrc = "src-tauri/target/release/cache-manager.exe"
if (-not (Test-Path $exeSrc)) { throw "not found $exeSrc, please run `just build` first" }
$exeDest = "release/CacheManager-$version.exe"
if (-not (Test-Path "release")) { New-Item -ItemType Directory -Path "release" | Out-Null }
if (Test-Path $exeDest) { Remove-Item $exeDest -Force }
Copy-Item $exeSrc $exeDest -Force
Write-Host "[release] generated $exeDest"

# 2) source tarball
$tgz = "release/source-$version.tar.gz"
if (Test-Path $tgz) { Remove-Item $tgz -Force }
tar -czf $tgz --exclude=node_modules --exclude=src-tauri/target --exclude=src-tauri/gen --exclude=.git --exclude=.zed --exclude=release --exclude=dist .
if ($LASTEXITCODE -ne 0) { throw "tar source packaging failed" }
Write-Host "[release] generated $tgz"

# 3) hash files (format: <hash>  <filename>, matching GNU coreutils)
foreach ($file in @($exeDest, $tgz)) {
    $name = Split-Path $file -Leaf
    $md5 = (Get-FileHash $file -Algorithm MD5).Hash.ToLower()
    $sha1 = (Get-FileHash $file -Algorithm SHA1).Hash.ToLower()
    Set-Content -Path "$file.md5" -Value "$md5  $name" -Encoding ASCII
    Set-Content -Path "$file.sha1" -Value "$sha1  $name" -Encoding ASCII
    Write-Host "[release] generated $file.md5 / $file.sha1"
}

# 4) git tag (v<version>) + push to remote
$tag = "v$version"
$tagExists = git tag --list $tag
if ($tagExists) {
    Write-Host "[release] git tag $tag already exists, skipped"
} else {
    git tag $tag
    if ($LASTEXITCODE -ne 0) { throw "git tag $tag creation failed" }
    git push origin $tag
    if ($LASTEXITCODE -ne 0) { throw "git push tag $tag failed" }
    Write-Host "[release] created and pushed git tag $tag"
}
Write-Host "[release] done"
