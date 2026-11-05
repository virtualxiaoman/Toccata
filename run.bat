@echo off
setlocal
chcp 936 >nul
cd /d "%~dp0"

echo.
echo   Toccata for Windows
echo   环境与数据都在本项目内；首次运行会准备工具链并构建，耗时较长。
echo.

"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\windows\run.ps1"
if errorlevel 1 (
    echo.
    echo 启动失败，请查看上方日志。
    pause
)

endlocal
