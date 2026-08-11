#!/usr/bin/env sh
# Cache Manager release script (invoked by `just release`)
# POSIX sh compatible: macOS / Linux / Windows (Git Bash)
# 1) copy exe -> release/CacheManager-<version>.exe
# 2) pack source -> release/source-<version>.tar.gz (exclude node_modules/target/.git/release/dist/gen/zed)
# 3) generate .md5 / .sha1 hash files for both artifacts
# 4) create and push git tag v<version> (skipped if already exists)
set -eu
cd "$(dirname "$0")/.."

# read version from Cargo.toml
version=$(grep '^version' src-tauri/Cargo.toml | head -1 | sed 's/.*"\([^"]*\)".*/\1/')
if [ -z "$version" ]; then
  echo "[release] cannot parse version from src-tauri/Cargo.toml" >&2
  exit 1
fi
echo "[release] version: $version"

# 1) exe
exe_src="src-tauri/target/release/cache-manager.exe"
if [ ! -f "$exe_src" ]; then
  echo "[release] not found $exe_src, please run 'just build' first" >&2
  exit 1
fi
mkdir -p release
exe_dest="release/CacheManager-$version.exe"
cp -f "$exe_src" "$exe_dest"
echo "[release] generated $exe_dest"

# 2) source tarball
tgz="release/source-$version.tar.gz"
tar -czf "$tgz" \
  --exclude=node_modules \
  --exclude=src-tauri/target \
  --exclude=src-tauri/gen \
  --exclude=.git \
  --exclude=.zed \
  --exclude=release \
  --exclude=dist \
  .
echo "[release] generated $tgz"

# 3) hash files (format: <hash>  <filename>, matching GNU coreutils)
for f in "$exe_dest" "$tgz"; do
  name=$(basename "$f")
  if command -v md5sum >/dev/null 2>&1; then
    md5=$(md5sum "$f" | awk '{print $1}')
  else
    md5=$(md5 -q "$f")   # macOS
  fi
  if command -v sha1sum >/dev/null 2>&1; then
    sha1=$(sha1sum "$f" | awk '{print $1}')
  else
    sha1=$(shasum -a 1 "$f" | awk '{print $1}')   # macOS
  fi
  printf '%s  %s\n' "$md5" "$name" > "$f.md5"
  printf '%s  %s\n' "$sha1" "$name" > "$f.sha1"
  echo "[release] generated $f.md5 / $f.sha1"
done

# 4) git tag (v<version>) + push to remote
tag="v$version"
if git tag --list "$tag" | grep -q .; then
  echo "[release] git tag $tag already exists, skipped"
else
  git tag "$tag"
  git push origin "$tag"
  echo "[release] created and pushed git tag $tag"
fi
echo "[release] done"
