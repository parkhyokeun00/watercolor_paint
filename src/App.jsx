import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Palette, Droplets, Waves, Beaker, RefreshCcw, Paintbrush, Image } from 'lucide-react';

const SCALE = 3;

// 프리셋 색상 팔레트
const COLOR_PRESETS = [
    { name: '울트라마린', color: '#1e3a8a' },
    { name: '세룰리안', color: '#0284c7' },
    { name: '비리디안', color: '#047857' },
    { name: '알리자린', color: '#be123c' },
    { name: '버밀리온', color: '#dc2626' },
    { name: '감보지', color: '#f59e0b' },
    { name: '번트시에나', color: '#92400e' },
    { name: '페인즈 그레이', color: '#374151' },
    { name: '인디고', color: '#312e81' },
    { name: '옐로 오커', color: '#a16207' },
    { name: '사프 그린', color: '#65a30d' },
    { name: '로즈 마더', color: '#e11d48' },
];

// hex → {r, g, b} (0~1)
function hexToRgb(hex) {
    const r = parseInt(hex.slice(1, 3), 16) / 255;
    const g = parseInt(hex.slice(3, 5), 16) / 255;
    const b = parseInt(hex.slice(5, 7), 16) / 255;
    return { r, g, b };
}

// --- 슬라이더 ---
function ControlSlider({ label, value, min, max, step, onChange }) {
    const display = typeof value === 'number' && !Number.isInteger(value)
        ? value.toFixed(3) : value;
    return (
        <div className="slider-row">
            <div className="slider-label-row">
                <span className="slider-label">{label}</span>
                <span className="slider-value">{display}</span>
            </div>
            <input type="range" min={min} max={max} step={step} value={value}
                onChange={(e) => onChange(parseFloat(e.target.value))} />
        </div>
    );
}

export default function App() {
    const canvasRef = useRef(null);
    const engineRef = useRef(null);
    const wasmModuleRef = useRef(null);

    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [activeColor, setActiveColor] = useState('#1e3a8a');
    const [brush, setBrush] = useState({ size: 6, water: 2.5, pigment: 0.6 });
    const [physics, setPhysics] = useState({
        dt: 0.15, evaporation: 0.002, viscosity: 0.05,
        pressure: 5.0, iterations: 10,
    });
    const [pigmentProps, setPigmentProps] = useState({ adhesion: 0.05, granularity: 0.8 });
    const [isSimulating, setIsSimulating] = useState(true);
    const [showTexture, setShowTexture] = useState(true);
    const [gridSize, setGridSize] = useState(300);
    const [paperTextureUrl, setPaperTextureUrl] = useState('');

    // WASM 초기화
    useEffect(() => {
        let cancelled = false;
        async function initWasm() {
            try {
                const wasm = await import('../wasm-pkg/watercolor_engine.js');
                await wasm.default();
                if (cancelled) return;
                wasmModuleRef.current = wasm;
                const engine = new wasm.WatercolorEngine();
                engineRef.current = engine;
                setGridSize(engine.grid_size());
                setLoading(false);
            } catch (err) {
                if (!cancelled) {
                    console.error('WASM 초기화 실패:', err);
                    setError(err.message || String(err));
                }
            }
        }
        initWasm();
        return () => { cancelled = true; };
    }, []);

    // 종이 텍스처 로드
    const loadPaperTexture = useCallback((url) => {
        const engine = engineRef.current;
        if (!engine) return;
        const img = new window.Image();
        img.crossOrigin = 'anonymous';
        img.onload = () => {
            const tempCanvas = document.createElement('canvas');
            tempCanvas.width = img.width;
            tempCanvas.height = img.height;
            const ctx = tempCanvas.getContext('2d');
            ctx.drawImage(img, 0, 0);
            const imageData = ctx.getImageData(0, 0, img.width, img.height);
            engine.load_paper_texture(imageData.data, img.width, img.height);
        };
        img.src = url;
    }, []);

    const handleTextureUpload = useCallback((e) => {
        const file = e.target.files[0];
        if (!file) return;
        const url = URL.createObjectURL(file);
        setPaperTextureUrl(url);
        loadPaperTexture(url);
    }, [loadPaperTexture]);

    // 파라미터 동기화
    useEffect(() => {
        const e = engineRef.current;
        if (!e) return;
        e.set_physics(physics.dt, physics.evaporation, physics.viscosity, physics.pressure, physics.iterations);
    }, [physics]);

    useEffect(() => {
        const e = engineRef.current;
        if (!e) return;
        e.set_pigment_props(pigmentProps.adhesion, pigmentProps.granularity);
    }, [pigmentProps]);

    useEffect(() => {
        const e = engineRef.current;
        if (!e) return;
        e.set_show_texture(showTexture);
    }, [showTexture]);

    // 렌더링
    const renderFrame = useCallback(() => {
        const engine = engineRef.current;
        const canvas = canvasRef.current;
        if (!engine || !canvas) return;
        const ctx = canvas.getContext('2d');
        const gs = gridSize;
        const pixelArray = engine.render();
        const pixelData = new Uint8ClampedArray(pixelArray.buffer || pixelArray);
        const imageData = new ImageData(pixelData, gs, gs);
        const tmp = document.createElement('canvas');
        tmp.width = gs; tmp.height = gs;
        tmp.getContext('2d').putImageData(imageData, 0, 0);
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.imageSmoothingEnabled = true;
        ctx.drawImage(tmp, 0, 0, canvas.width, canvas.height);
    }, [gridSize]);

    // 시뮬레이션 루프
    useEffect(() => {
        if (!isSimulating || loading) return;
        const engine = engineRef.current;
        if (!engine) return;
        let frameId;
        const loop = () => {
            engine.step();
            renderFrame();
            frameId = requestAnimationFrame(loop);
        };
        loop();
        return () => cancelAnimationFrame(frameId);
    }, [isSimulating, loading, renderFrame]);

    // 브러시 상호작용
    const lastPosRef = useRef(null);

    const handleInteraction = useCallback((e, isFirst) => {
        const engine = engineRef.current;
        const canvas = canvasRef.current;
        if (!engine || !canvas) return;
        const rect = canvas.getBoundingClientRect();
        const x = Math.floor((e.clientX - rect.left) / SCALE);
        const y = Math.floor((e.clientY - rect.top) / SCALE);
        const { r, g, b } = hexToRgb(activeColor);

        if (isFirst || !lastPosRef.current) {
            engine.apply_brush(x, y, brush.size, brush.water, brush.pigment, r, g, b);
        } else {
            engine.apply_brush_stroke(
                lastPosRef.current.x, lastPosRef.current.y,
                x, y, brush.size, brush.water, brush.pigment, r, g, b
            );
        }
        lastPosRef.current = { x, y };
    }, [brush, activeColor]);

    const handleMouseDown = useCallback((e) => {
        handleInteraction(e, true);
        const draw = (me) => handleInteraction(me, false);
        const stop = () => {
            lastPosRef.current = null;
            window.removeEventListener('mousemove', draw);
            window.removeEventListener('mouseup', stop);
        };
        window.addEventListener('mousemove', draw);
        window.addEventListener('mouseup', stop);
    }, [handleInteraction]);

    const handleReset = useCallback(() => {
        const engine = engineRef.current;
        if (!engine) return;
        engine.reset();
        renderFrame();
    }, [renderFrame]);

    // --- 에러/로딩 ---
    if (error) {
        return (
            <div className="error-overlay">
                <div className="error-title">WASM 엔진 로드 실패</div>
                <div className="error-message">
                    Rust WASM 모듈을 불러올 수 없습니다.<br />
                    1. <code>cd watercolor-engine && wasm-pack build --target web --out-dir ../wasm-pkg</code><br />
                    2. <code>npm install && npm run dev</code><br /><br />
                    에러: {error}
                </div>
            </div>
        );
    }

    if (loading) {
        return (
            <div className="loading-overlay">
                <div className="loading-spinner" />
                <div className="loading-text">WASM 엔진 로딩 중...</div>
                <div className="loading-sub">Rust 물리 엔진을 초기화하고 있습니다</div>
            </div>
        );
    }

    return (
        <div className="app-container">
            <header className="app-header">
                <div className="header-left">
                    <Palette />
                    <h1 className="header-title">
                        마스터 수채화 스튜디오
                        <span className="header-badge">WASM</span>
                    </h1>
                </div>
                <button className="btn-reset" onClick={handleReset}>
                    <RefreshCcw /> 캔버스 초기화
                </button>
            </header>

            <main className="main-area">
                {/* 사이드바 */}
                <aside className="sidebar">
                    {/* 색상 선택 */}
                    <section>
                        <div className="section-header">
                            <Palette />
                            <span className="section-title">색상 선택</span>
                        </div>

                        {/* 컬러 피커 */}
                        <div className="color-picker-area">
                            <input
                                type="color"
                                value={activeColor}
                                onChange={(e) => setActiveColor(e.target.value)}
                                className="color-picker-input"
                            />
                            <span className="color-hex">{activeColor.toUpperCase()}</span>
                        </div>

                        {/* 프리셋 팔레트 */}
                        <div className="preset-palette">
                            {COLOR_PRESETS.map((preset) => (
                                <button
                                    key={preset.color}
                                    className={`preset-swatch ${activeColor === preset.color ? 'active' : ''}`}
                                    style={{ backgroundColor: preset.color }}
                                    title={preset.name}
                                    onClick={() => setActiveColor(preset.color)}
                                />
                            ))}
                        </div>
                    </section>

                    {/* 브러시 설정 */}
                    <section className="card-section">
                        <div className="section-header">
                            <Paintbrush />
                            <span className="section-title">브러시 설정</span>
                        </div>
                        <div className="slider-group">
                            <ControlSlider label="붓 크기" value={brush.size} min={1} max={20} step={1}
                                onChange={(v) => setBrush({ ...brush, size: v })} />
                            <ControlSlider label="수분량" value={brush.water} min={0.1} max={5.0} step={0.1}
                                onChange={(v) => setBrush({ ...brush, water: v })} />
                            <ControlSlider label="안료 농도" value={brush.pigment} min={0.05} max={2.0} step={0.05}
                                onChange={(v) => setBrush({ ...brush, pigment: v })} />
                        </div>
                    </section>

                    {/* 물리 엔진 */}
                    <section className="card-section">
                        <div className="section-header">
                            <Waves />
                            <span className="section-title">물리 엔진</span>
                        </div>
                        <div className="slider-group">
                            <ControlSlider label="점성" value={physics.viscosity} min={0} max={0.5} step={0.01}
                                onChange={(v) => setPhysics({ ...physics, viscosity: v })} />
                            <ControlSlider label="수압" value={physics.pressure} min={0.5} max={15.0} step={0.5}
                                onChange={(v) => setPhysics({ ...physics, pressure: v })} />
                            <ControlSlider label="증발" value={physics.evaporation} min={0.0001} max={0.01} step={0.0001}
                                onChange={(v) => setPhysics({ ...physics, evaporation: v })} />
                        </div>
                    </section>

                    {/* 안료 거동 */}
                    <section className="card-section">
                        <div className="section-header">
                            <Beaker />
                            <span className="section-title">안료 거동</span>
                        </div>
                        <div className="slider-group">
                            <ControlSlider label="흡착력" value={pigmentProps.adhesion} min={0.001} max={0.3} step={0.001}
                                onChange={(v) => setPigmentProps({ ...pigmentProps, adhesion: v })} />
                            <ControlSlider label="과립화" value={pigmentProps.granularity} min={0} max={2.0} step={0.1}
                                onChange={(v) => setPigmentProps({ ...pigmentProps, granularity: v })} />
                        </div>
                    </section>

                    {/* 종이 텍스처 */}
                    <section className="card-section">
                        <div className="section-header">
                            <Image />
                            <span className="section-title">종이 텍스처</span>
                        </div>
                        <div className="texture-controls">
                            <label className="texture-upload-btn">
                                📄 텍스처 이미지 불러오기
                                <input type="file" accept="image/*" onChange={handleTextureUpload}
                                    style={{ display: 'none' }} />
                            </label>
                            {paperTextureUrl && (
                                <div className="texture-preview">
                                    <img src={paperTextureUrl} alt="종이 텍스처" />
                                </div>
                            )}
                        </div>
                        <div className="toggle-row" onClick={() => setShowTexture(!showTexture)}>
                            <span className="toggle-label">텍스처 적용</span>
                            <div className={`toggle-track ${showTexture ? 'on' : 'off'}`}>
                                <div className="toggle-thumb" />
                            </div>
                        </div>
                    </section>
                </aside>

                {/* 캔버스 */}
                <section className="canvas-area">
                    <div className="canvas-frame"
                        style={{ width: gridSize * SCALE, height: gridSize * SCALE }}>
                        <canvas
                            ref={canvasRef}
                            width={gridSize * SCALE}
                            height={gridSize * SCALE}
                            onMouseDown={handleMouseDown}
                        />
                    </div>

                    <div className="status-bar">
                        <div className="status-item">
                            <span className="status-label">ENGINE</span>
                            <span className="status-value">Rust/WASM</span>
                        </div>
                        <div className="status-item">
                            <span className="status-label">COLOR</span>
                            <span className="status-value" style={{ color: activeColor }}>●</span>
                        </div>
                        <div className="status-item">
                            <span className="status-label">BRUSH</span>
                            <span className="status-value">{brush.size}px</span>
                        </div>
                        <div className="status-item">
                            <span className="status-label">GRID</span>
                            <span className="status-value">{gridSize}×{gridSize}</span>
                        </div>
                    </div>
                </section>
            </main>
        </div>
    );
}
