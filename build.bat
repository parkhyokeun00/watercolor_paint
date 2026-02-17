@echo off
chcp 65001 >nul
echo === 수채화 WASM 엔진 빌드 시작 ===
echo.

echo [1/3] Rust 확인 중...
rustc --version
if errorlevel 1 (
    echo.
    echo [오류] Rust가 설치되어 있지 않습니다!
    echo 아래 명령으로 설치해 주세요:
    echo   winget install Rustlang.Rustup
    echo 또는 https://rustup.rs 에서 다운로드
    echo.
    pause
    exit /b 1
)

echo.
echo [1.5/3] wasm-pack 확인 중...
wasm-pack --version
if errorlevel 1 (
    echo wasm-pack이 없습니다. 설치합니다...
    cargo install wasm-pack
)

echo.
echo [1.7/3] wasm32 타겟 추가 중...
rustup target add wasm32-unknown-unknown

echo.
echo [2/3] WASM 빌드 중...
cd watercolor-engine
wasm-pack build --target web --out-dir ../wasm-pkg
if errorlevel 1 (
    echo.
    echo [오류] WASM 빌드에 실패했습니다!
    pause
    exit /b 1
)
cd ..

echo.
echo [3/3] npm 패키지 설치 중...
npm install
if errorlevel 1 (
    echo.
    echo [오류] npm install에 실패했습니다!
    pause
    exit /b 1
)

echo.
echo === 빌드 완료! ===
echo npm run dev 로 개발 서버를 시작하세요.
echo.
pause
