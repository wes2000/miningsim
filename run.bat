@echo off
REM MiningSim — quick smoke-test launcher.
REM Runs the game in dev profile; close the window to exit.
cd /d "%~dp0"
cargo run
