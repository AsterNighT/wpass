@echo off
.\wpass.exe -g .
REGEDIT.EXE  /S  "%~dp0\wpass.reg"
rm "%~dp0\wpass.reg"
pause