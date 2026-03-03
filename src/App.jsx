import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Palette, Waves, Beaker, RefreshCcw, Paintbrush, Image, Maximize, Download } from 'lucide-react';

const DISPLAY_SCALE = 2.2;

// 캔버스 비율 프리셋
const RATIO_PRESETS = [
    { name: '1:1 정방형', w: 420, h: 420, icon: '◻' },
    { name: '4:3 가로', w: 560, h: 420, icon: '▬' },
    { name: '3:4 세로', w: 420, h: 560, icon: '▮' },
    { name: '16:9 와이드', w: 672, h: 378, icon: '▭' },
    { name: '9:16 세로', w: 378, h: 672, icon: '▯' },
    { name: '3:2 가로', w: 630, h: 420, icon: '▬' },
    { name: '2:3 세로', w: 420, h: 630, icon: '▮' },
    { name: 'A4 가로', w: 594, h: 420, icon: '📄' },
    { name: 'A4 세로', w: 420, h: 594, icon: '📄' },
];

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

const BRUSH_MODES = [
    { key: 'paint', label: '기본 붓' },
    { key: 'background', label: '배경 붓' },
    { key: 'fade', label: '점진 지우개' },
    { key: 'blend', label: '블렌딩 붓' },
    { key: 'water', label: '물 번짐 붓' },
];

function hexToRgb(hex) {
    const r = parseInt(hex.slice(1, 3), 16) / 255;
    const g = parseInt(hex.slice(3, 5), 16) / 255;
    const b = parseInt(hex.slice(5, 7), 16) / 255;
    return { r, g, b };
}

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
    const canvasAreaRef = useRef(null);
    const canvasRef = useRef(null);
    const cursorCanvasRef = useRef(null);
    const engineRef = useRef(null);
    const wasmModuleRef = useRef(null);

    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [activeColor, setActiveColor] = useState('#1e3a8a');
    const [brush, setBrush] = useState({ size: 8, water: 2.5, pigment: 0.6, speedSensitivity: 0.5 });
    const [backgroundBrush, setBackgroundBrush] = useState({ size: 90, water: 1.4, pigment: 0.35 });
    const [brushMode, setBrushMode] = useState('paint');
    const [fadeStrength, setFadeStrength] = useState(0.35);
    const [blendStrength, setBlendStrength] = useState(0.45);
    const [waterFlow, setWaterFlow] = useState(1.0);
    const [physics, setPhysics] = useState({
        dt: 0.15, evaporation: 0.002, viscosity: 0.05,
        pressure: 5.0, iterations: 10,
    });
    const [pigmentProps, setPigmentProps] = useState({ adhesion: 0.05, granularity: 0.8 });
    const [isSimulating, setIsSimulating] = useState(true);
    const [showTexture, setShowTexture] = useState(true);
    const [silhouetteStrength, setSilhouetteStrength] = useState(0.85);
    const [edgeBleedStrength, setEdgeBleedStrength] = useState(0.35);
    const [canvasWidth, setCanvasWidth] = useState(420);
    const [canvasHeight, setCanvasHeight] = useState(420);
    const [canvasViewport, setCanvasViewport] = useState({ w: 0, h: 0 });
    const [selectedRatio, setSelectedRatio] = useState('1:1 정방형');
    const [freeCanvasSize, setFreeCanvasSize] = useState({ w: 420, h: 420 });
    const [paperTextureUrl, setPaperTextureUrl] = useState('');
    const [freePaintMode, setFreePaintMode] = useState(false);

    const lastPosRef = useRef(null);
    const lastTimeRef = useRef(null);
    const velocityRef = useRef(0);
    const dynamicSizeRef = useRef(8);

    // WASM 초기화 (width, height 있으면 재생성)
    const initEngine = useCallback(async (w, h) => {
        try {
            let wasm = wasmModuleRef.current;
            if (!wasm) {
                wasm = await import('../wasm-pkg/watercolor_engine.js');
                await wasm.default();
                wasmModuleRef.current = wasm;
            }
            const engine = new wasm.WatercolorEngine(w, h);
            engineRef.current = engine;
            setCanvasWidth(w);
            setCanvasHeight(h);
            setLoading(false);
        } catch (err) {
            console.error('WASM 초기화 실패:', err);
            setError(err.message || String(err));
        }
    }, []);

    useEffect(() => {
        const initial = RATIO_PRESETS[0];
        initEngine(initial.w, initial.h);
    }, [initEngine]);

    useEffect(() => {
        const node = canvasAreaRef.current;
        if (!node) return;
        const update = () => {
            setCanvasViewport({ w: node.clientWidth, h: node.clientHeight });
        };
        update();
        if (typeof ResizeObserver !== 'undefined') {
            const obs = new ResizeObserver(update);
            obs.observe(node);
            return () => obs.disconnect();
        }
        window.addEventListener('resize', update);
        return () => window.removeEventListener('resize', update);
    }, []);

    // 비율 변경
    const handleRatioChange = useCallback((preset) => {
        setSelectedRatio(preset.name);
        setFreeCanvasSize({ w: preset.w, h: preset.h });
        setIsSimulating(false);
        setTimeout(async () => {
            await initEngine(preset.w, preset.h);
            setIsSimulating(true);
        }, 50);
    }, [initEngine]);

    const handleFreeCanvasApply = useCallback(() => {
        const w = Math.max(128, Math.min(1600, Math.floor(freeCanvasSize.w || 0)));
        const h = Math.max(128, Math.min(1600, Math.floor(freeCanvasSize.h || 0)));
        setSelectedRatio('프리 캔버스');
        setFreeCanvasSize({ w, h });
        setIsSimulating(false);
        setTimeout(async () => {
            await initEngine(w, h);
            setIsSimulating(true);
        }, 50);
    }, [freeCanvasSize, initEngine]);

    // 종이 텍스처 로드
    const loadPaperTexture = useCallback((url) => {
        const engine = engineRef.current;
        if (!engine) return;
        const img = new window.Image();
        img.crossOrigin = 'anonymous';
        img.onload = () => {
            const tmpC = document.createElement('canvas');
            tmpC.width = img.width; tmpC.height = img.height;
            const ctx = tmpC.getContext('2d');
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

    useEffect(() => {
        const e = engineRef.current;
        if (!e || !e.set_silhouette_controls) return;
        const appliedSilhouetteStrength = freePaintMode ? 0 : silhouetteStrength;
        const appliedEdgeBleedStrength = freePaintMode ? 0 : edgeBleedStrength;
        e.set_silhouette_controls(appliedSilhouetteStrength, appliedEdgeBleedStrength);
    }, [silhouetteStrength, edgeBleedStrength, freePaintMode]);

    // 렌더링
    const renderFrame = useCallback(() => {
        const engine = engineRef.current;
        const canvas = canvasRef.current;
        if (!engine || !canvas) return;
        const ctx = canvas.getContext('2d');
        const w = canvasWidth;
        const h = canvasHeight;
        const pixelArray = engine.render();
        const pixelData = new Uint8ClampedArray(pixelArray.buffer || pixelArray);
        const imageData = new ImageData(pixelData, w, h);
        const tmp = document.createElement('canvas');
        tmp.width = w; tmp.height = h;
        tmp.getContext('2d').putImageData(imageData, 0, 0);
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.imageSmoothingEnabled = true;
        ctx.imageSmoothingQuality = 'high';
        ctx.drawImage(tmp, 0, 0, canvas.width, canvas.height);
    }, [canvasWidth, canvasHeight]);

    // 커서 프리뷰
    const drawCursor = useCallback((clientX, clientY) => {
        const cursorCanvas = cursorCanvasRef.current;
        const mainCanvas = canvasRef.current;
        if (!cursorCanvas || !mainCanvas) return;
        const ctx = cursorCanvas.getContext('2d');
        ctx.clearRect(0, 0, cursorCanvas.width, cursorCanvas.height);
        const rect = mainCanvas.getBoundingClientRect();
        const x = clientX - rect.left;
        const y = clientY - rect.top;
        if (x < 0 || y < 0 || x > rect.width || y > rect.height) return;
        const dynSize = dynamicSizeRef.current * DISPLAY_SCALE;
        ctx.beginPath();
        ctx.arc(x, y, dynSize, 0, Math.PI * 2);
        ctx.strokeStyle = activeColor + '88';
        ctx.lineWidth = 1.5;
        ctx.stroke();
        ctx.closePath();
        ctx.beginPath();
        ctx.arc(x, y, 1.5, 0, Math.PI * 2);
        ctx.fillStyle = activeColor;
        ctx.fill();
        ctx.closePath();
    }, [activeColor]);

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

    // 속도 계산 + 동적 브러시 크기
    const calcVelocityAndSize = useCallback((x, y, now) => {
        let velocity = 0;
        if (lastPosRef.current && lastTimeRef.current) {
            const dx = x - lastPosRef.current.x;
            const dy = y - lastPosRef.current.y;
            const dt = (now - lastTimeRef.current) / 1000;
            if (dt > 0) velocity = Math.sqrt(dx * dx + dy * dy) / dt;
        }
        velocityRef.current = velocityRef.current * 0.6 + velocity * 0.4;
        if (brushMode === 'background') {
            dynamicSizeRef.current = backgroundBrush.size;
            return { velocity: velocityRef.current, dynSize: dynamicSizeRef.current };
        }
        const sens = brush.speedSensitivity;
        const speedFactor = 1.0 / (1.0 + velocityRef.current * sens * 0.01);
        const minSize = brush.size * 0.3;
        dynamicSizeRef.current = minSize + (brush.size - minSize) * speedFactor;
        return { velocity: velocityRef.current, dynSize: dynamicSizeRef.current };
    }, [brushMode, backgroundBrush.size, brush.size, brush.speedSensitivity]);

    // 브러시 상호작용
    const handleInteraction = useCallback((e, isFirst) => {
        const engine = engineRef.current;
        const canvas = canvasRef.current;
        if (!engine || !canvas) return;
        const rect = canvas.getBoundingClientRect();
        const scaleX = canvasWidth / rect.width;
        const scaleY = canvasHeight / rect.height;
        const x = Math.floor((e.clientX - rect.left) * scaleX);
        const y = Math.floor((e.clientY - rect.top) * scaleY);
        const { r, g, b } = hexToRgb(activeColor);
        const now = performance.now();
        const { velocity, dynSize } = calcVelocityAndSize(x, y, now);

        if (brushMode === 'paint') {
            if (isFirst || !lastPosRef.current) {
                engine.apply_brush(x, y, dynSize, brush.water, brush.pigment, r, g, b, 0.0, 1.0);
            } else {
                engine.apply_brush_stroke(
                    lastPosRef.current.x, lastPosRef.current.y,
                    x, y, dynSize, brush.water, brush.pigment, r, g, b, velocity);
            }
        } else if (brushMode === 'background') {
            if (isFirst || !lastPosRef.current) {
                engine.apply_background_brush_stroke(
                    x, y,
                    x, y, dynSize, backgroundBrush.water, backgroundBrush.pigment, r, g, b);
            } else {
                engine.apply_background_brush_stroke(
                    lastPosRef.current.x, lastPosRef.current.y,
                    x, y, dynSize, backgroundBrush.water, backgroundBrush.pigment, r, g, b);
            }
        } else if (brushMode === 'fade') {
            if (isFirst || !lastPosRef.current) {
                engine.apply_fade_brush_stroke(
                    x, y,
                    x, y, dynSize, fadeStrength, velocity);
            } else {
                engine.apply_fade_brush_stroke(
                    lastPosRef.current.x, lastPosRef.current.y,
                    x, y, dynSize, fadeStrength, velocity);
            }
        } else if (brushMode === 'blend') {
            if (isFirst || !lastPosRef.current) {
                engine.apply_blend_brush_stroke(
                    x, y,
                    x, y, dynSize, blendStrength, velocity);
            } else {
                engine.apply_blend_brush_stroke(
                    lastPosRef.current.x, lastPosRef.current.y,
                    x, y, dynSize, blendStrength, velocity);
            }
        } else if (brushMode === 'water') {
            if (isFirst || !lastPosRef.current) {
                engine.apply_water_brush_stroke(
                    x, y,
                    x, y, dynSize, brush.water, waterFlow, velocity);
            } else {
                engine.apply_water_brush_stroke(
                    lastPosRef.current.x, lastPosRef.current.y,
                    x, y, dynSize, brush.water, waterFlow, velocity);
            }
        }
        lastPosRef.current = { x, y };
        lastTimeRef.current = now;
        drawCursor(e.clientX, e.clientY);
    }, [brush, backgroundBrush, brushMode, fadeStrength, blendStrength, waterFlow, activeColor, canvasWidth, canvasHeight, calcVelocityAndSize, drawCursor]);

    const handleMouseDown = useCallback((e) => {
        velocityRef.current = 0;
        dynamicSizeRef.current = brushMode === 'background' ? backgroundBrush.size : brush.size;
        handleInteraction(e, true);
        const draw = (me) => handleInteraction(me, false);
        const stop = () => {
            lastPosRef.current = null;
            lastTimeRef.current = null;
            velocityRef.current = 0;
            dynamicSizeRef.current = brushMode === 'background' ? backgroundBrush.size : brush.size;
            window.removeEventListener('mousemove', draw);
            window.removeEventListener('mouseup', stop);
        };
        window.addEventListener('mousemove', draw);
        window.addEventListener('mouseup', stop);
    }, [handleInteraction, brushMode, brush.size, backgroundBrush.size]);

    const handleCanvasMouseMove = useCallback((e) => drawCursor(e.clientX, e.clientY), [drawCursor]);
    const handleCanvasMouseLeave = useCallback(() => {
        const cc = cursorCanvasRef.current;
        if (cc) cc.getContext('2d').clearRect(0, 0, cc.width, cc.height);
    }, []);

    const handleReset = useCallback(() => {
        const e = engineRef.current;
        if (!e) return;
        e.reset();
        renderFrame();
    }, [renderFrame]);

    const handleDownload = useCallback(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const link = document.createElement('a');
        const now = new Date();
        const ts = `${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, '0')}${String(now.getDate()).padStart(2, '0')}_${String(now.getHours()).padStart(2, '0')}${String(now.getMinutes()).padStart(2, '0')}${String(now.getSeconds()).padStart(2, '0')}`;
        link.download = `watercolor_${ts}.png`;
        link.href = canvas.toDataURL('image/png');
        link.click();
    }, []);

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

    const displayW = Math.round(canvasWidth * DISPLAY_SCALE);
    const displayH = Math.round(canvasHeight * DISPLAY_SCALE);
    const hasViewport = canvasViewport.w > 40 && canvasViewport.h > 40;
    const availW = hasViewport ? Math.max(1, canvasViewport.w - 24) : displayW;
    const availH = hasViewport ? Math.max(1, canvasViewport.h - 24) : displayH;
    const fitScale = hasViewport ? Math.min(1, availW / displayW, availH / displayH) : 1;
    const frameW = Math.max(1, Math.round(displayW * fitScale));
    const frameH = Math.max(1, Math.round(displayH * fitScale));

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
                <div className="header-actions">
                    {/* Actions moved to sidebar */}
                </div>
            </header>

            <main className="main-area">
                <aside className="sidebar">
                    {/* 캔버스 비율 */}
                    <section className="card-section">
                        <div className="section-header">
                            <Maximize />
                            <span className="section-title">캔버스 비율</span>
                        </div>
                        <div className="ratio-grid">
                            {RATIO_PRESETS.map((preset) => (
                                <button
                                    key={preset.name}
                                    className={`ratio-btn ${selectedRatio === preset.name ? 'active' : ''}`}
                                    onClick={() => handleRatioChange(preset)}
                                    title={`${preset.w}×${preset.h}`}
                                >
                                    <span className="ratio-icon">{preset.icon}</span>
                                    <span className="ratio-label">{preset.name}</span>
                                </button>
                            ))}
                        </div>
                        <div className="ratio-info">
                            <span>{canvasWidth}×{canvasHeight}px</span>
                        </div>
                        <div className="free-canvas-row">
                            <input
                                type="number"
                                min={128}
                                max={1600}
                                className="free-canvas-input"
                                value={freeCanvasSize.w}
                                onChange={(e) => setFreeCanvasSize({ ...freeCanvasSize, w: parseInt(e.target.value || '0', 10) })}
                                title="가로 픽셀"
                            />
                            <span className="free-canvas-sep">×</span>
                            <input
                                type="number"
                                min={128}
                                max={1600}
                                className="free-canvas-input"
                                value={freeCanvasSize.h}
                                onChange={(e) => setFreeCanvasSize({ ...freeCanvasSize, h: parseInt(e.target.value || '0', 10) })}
                                title="세로 픽셀"
                            />
                            <button className="free-canvas-apply" onClick={handleFreeCanvasApply}>프리 캔버스</button>
                        </div>
                    </section>

                    {/* 색상 선택 */}
                    <section>
                        <div className="section-header">
                            <Palette />
                            <span className="section-title">색상 선택</span>
                        </div>
                        <div className="color-picker-area">
                            <input type="color" value={activeColor}
                                onChange={(e) => setActiveColor(e.target.value)}
                                className="color-picker-input" />
                            <span className="color-hex">{activeColor.toUpperCase()}</span>
                        </div>
                        <div className="preset-palette">
                            {COLOR_PRESETS.map((preset) => (
                                <button key={preset.color}
                                    className={`preset-swatch ${activeColor === preset.color ? 'active' : ''}`}
                                    style={{ backgroundColor: preset.color }}
                                    title={preset.name}
                                    onClick={() => setActiveColor(preset.color)} />
                            ))}
                        </div>
                    </section>

                    {/* 브러시 설정 */}
                    <section className="card-section">
                        <div className="section-header">
                            <Paintbrush />
                            <span className="section-title">브러시 설정</span>
                        </div>
                        <div className="ratio-grid">
                            {BRUSH_MODES.map((mode) => (
                                <button
                                    key={mode.key}
                                    className={`ratio-btn ${brushMode === mode.key ? 'active' : ''}`}
                                    onClick={() => setBrushMode(mode.key)}
                                >
                                    <span className="ratio-label">{mode.label}</span>
                                </button>
                            ))}
                        </div>
                        <div className="slider-group">
                            {brushMode !== 'background' && (
                                <ControlSlider label="붓 크기" value={brush.size} min={1} max={25} step={1}
                                    onChange={(v) => setBrush({ ...brush, size: v })} />
                            )}
                            {brushMode === 'background' && (
                                <ControlSlider label="배경 붓 크기" value={backgroundBrush.size} min={20} max={220} step={2}
                                    onChange={(v) => setBackgroundBrush({ ...backgroundBrush, size: v })} />
                            )}
                            {brushMode !== 'background' && (
                                <ControlSlider label="수분량" value={brush.water} min={0.1} max={5.0} step={0.1}
                                    onChange={(v) => setBrush({ ...brush, water: v })} />
                            )}
                            {brushMode === 'background' && (
                                <ControlSlider label="배경 수분량" value={backgroundBrush.water} min={0.1} max={3.0} step={0.1}
                                    onChange={(v) => setBackgroundBrush({ ...backgroundBrush, water: v })} />
                            )}
                            {brushMode === 'paint' && (
                                <ControlSlider label="안료 농도" value={brush.pigment} min={0.05} max={2.0} step={0.05}
                                    onChange={(v) => setBrush({ ...brush, pigment: v })} />
                            )}
                            {brushMode === 'background' && (
                                <ControlSlider label="배경 안료 농도" value={backgroundBrush.pigment} min={0.05} max={0.9} step={0.05}
                                    onChange={(v) => setBackgroundBrush({ ...backgroundBrush, pigment: v })} />
                            )}
                            {brushMode === 'fade' && (
                                <ControlSlider label="지우기 강도" value={fadeStrength} min={0.05} max={1.0} step={0.05}
                                    onChange={setFadeStrength} />
                            )}
                            {brushMode === 'blend' && (
                                <ControlSlider label="블렌딩 강도" value={blendStrength} min={0.05} max={1.0} step={0.05}
                                    onChange={setBlendStrength} />
                            )}
                            {brushMode === 'water' && (
                                <ControlSlider label="번짐 강도" value={waterFlow} min={0.1} max={2.0} step={0.1}
                                    onChange={setWaterFlow} />
                            )}
                            {brushMode !== 'background' && (
                                <ControlSlider label="속도 감응" value={brush.speedSensitivity} min={0} max={1.0} step={0.05}
                                    onChange={(v) => setBrush({ ...brush, speedSensitivity: v })} />
                            )}
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
                                <div className="texture-preview"><img src={paperTextureUrl} alt="종이 텍스처" /></div>
                            )}
                        </div>
                        <div className="toggle-row" onClick={() => setShowTexture(!showTexture)}>
                            <span className="toggle-label">텍스처 적용</span>
                            <div className={`toggle-track ${showTexture ? 'on' : 'off'}`}>
                                <div className="toggle-thumb" />
                            </div>
                        </div>
                        <div className="toggle-row" onClick={() => setFreePaintMode(!freePaintMode)}>
                            <span className="toggle-label">자유 색칠 모드 (구버전)</span>
                            <div className={`toggle-track ${freePaintMode ? 'on' : 'off'}`}>
                                <div className="toggle-thumb" />
                            </div>
                        </div>
                        <div className="slider-group">
                            <ControlSlider
                                label="실루엣 강도"
                                value={silhouetteStrength}
                                min={0}
                                max={1.5}
                                step={0.05}
                                onChange={setSilhouetteStrength}
                            />
                            <ControlSlider
                                label="외곽 번짐 강도"
                                value={edgeBleedStrength}
                                min={0}
                                max={2.0}
                                step={0.05}
                                onChange={setEdgeBleedStrength}
                            />
                        </div>
                    </section>

                    {/* 작업 관리 */}
                    <section className="card-section">
                        <div className="sidebar-actions">
                            <button className="btn-download-full" onClick={handleDownload}>
                                <Download size={18} /> 이미지 다운로드
                            </button>
                            <button className="btn-reset-full" onClick={handleReset}>
                                <RefreshCcw size={18} /> 캔버스 초기화
                            </button>
                        </div>
                    </section>
                </aside>

                {/* 캔버스 */}
                <section className="canvas-area" ref={canvasAreaRef}>
                    <div className="canvas-frame" style={{ width: frameW, height: frameH }}>
                        <canvas ref={canvasRef} width={displayW} height={displayH}
                            onMouseDown={handleMouseDown} />
                        <canvas ref={cursorCanvasRef} className="cursor-canvas"
                            width={displayW} height={displayH}
                            onMouseMove={handleCanvasMouseMove}
                            onMouseLeave={handleCanvasMouseLeave}
                            onMouseDown={handleMouseDown} />
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
                            <span className="status-value">{BRUSH_MODES.find((m) => m.key === brushMode)?.label}</span>
                        </div>
                        <div className="status-item">
                            <span className="status-label">SIZE</span>
                            <span className="status-value">{brushMode === 'background' ? backgroundBrush.size : brush.size}px</span>
                        </div>
                        <div className="status-item">
                            <span className="status-label">CANVAS</span>
                            <span className="status-value">{canvasWidth}×{canvasHeight}</span>
                        </div>
                        <div className="status-item">
                            <span className="status-label">PAINT</span>
                            <span className="status-value">{freePaintMode ? '자유(구버전)' : '실루엣 제약'}</span>
                        </div>
                    </div>
                </section>
            </main>
        </div>
    );
}
