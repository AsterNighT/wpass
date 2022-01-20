@echo off
cd "%~dp0\.."
rm release.zip
rm -rf "release"
cargo build --release
mkdir release
cp "target\x86_64-pc-windows-msvc\release\wpass.exe" "release\wpass.exe"
cp "bin\7za.exe" "release\7za.exe"
cp "config\config.toml.example" "release\config.toml"
cp "hack\install.bat" "release\install.bat"
cp "readme.md" "release\readme.md"
touch "release\dict.txt"
"release\7za.exe" a release.zip release
rm -rf "release"