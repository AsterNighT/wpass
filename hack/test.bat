cd "%~dp0\.."
cargo build
rm -rf test
mkdir test
cd test
echo test > text.txt
..\bin\7za.exe a -tzip test.zip text.txt -p123456
rm test.txt
echo 123456 > dict.txt
..\target\x86_64-pc-windows-msvc\debug\wpass.exe -p dict.txt -d test.zip
if errorlevel 1 (
    echo "error"
)
cd ..
rm -rf test