# 마스터 수채화 스튜디오 — WASM Engine

Rust/WASM 기반 실시간 수채화 시뮬레이션

## 기술 스택

| 레이어 | 기술 | 역할 |
|---|---|---|
| 물리 엔진 | Rust → WASM | 유체역학, 안료 침전, KM 렌더링 |
| 바인딩 | wasm-bindgen | JS ↔ Rust 타입 변환 |
| 프론트엔드 | Vite + React | UI, Canvas 렌더링 |

## 필수 요구사항

- **Rust** (rustup): https://rustup.rs
- **wasm-pack**: `cargo install wasm-pack`
- **Node.js** 18+: https://nodejs.org
- **wasm32 타겟**: `rustup target add wasm32-unknown-unknown`

## 빠른 시작

### 1. WASM 엔진 빌드
```bash
cd watercolor-engine
wasm-pack build --target web --out-dir ../wasm-pkg
cd ..
```

### 2. npm 패키지 설치
```bash
npm install
```

### 3. 개발 서버 실행
```bash
npm run dev
```

또는 `build.bat`을 더블클릭하면 1~2단계를 자동으로 진행합니다.

## 프로젝트 구조

```
├── watercolor-engine/     Rust WASM 크레이트
│   ├── Cargo.toml
│   └── src/lib.rs         물리 엔진 (유체+안료+KM)
├── src/                   React 프론트엔드
│   ├── App.jsx            메인 UI
│   ├── main.jsx           엔트리
│   └── index.css          디자인 시스템
├── index.html             Vite 엔트리
├── vite.config.js         Vite + WASM 플러그인
├── package.json
└── build.bat              원클릭 빌드
```
