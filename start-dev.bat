@echo off
echo Starting RustNewsLatest Development Environment...
echo.

echo Starting backend server...
cd server
start "Backend Server" cmd /k "cargo run"
timeout /t 3 /nobreak >nul

echo.
echo Starting frontend development server...
cd ..
start "Frontend Dev Server" cmd /k "npm run dev"

echo.
echo Development environment started!
echo Backend: http://localhost:8080
echo Frontend: http://localhost:3000
echo.
pause
