/* tslint:disable */
/* eslint-disable */

export class WatercolorEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * 고급 수채화 브러시 — 붓털 노이즈 + 종이 반응 + 타원형 스탬프
     */
    apply_brush(cx: number, cy: number, size: number, water: number, pigment_amount: number, r: number, g: number, b: number, angle: number, pressure: number): void;
    /**
     * 고급 스트로크 보간 — 속도 감응 + 안료 소진 + 방향 추적
     */
    apply_brush_stroke(x0: number, y0: number, x1: number, y1: number, size: number, water: number, pigment_amount: number, r: number, g: number, b: number, velocity: number): void;
    grid_size(): number;
    /**
     * 외부 종이 텍스처 로드
     */
    load_paper_texture(data: Uint8Array, width: number, height: number): void;
    constructor();
    /**
     * 렌더링 — 고급 감산 혼합 + 젖은 영역 반짝임 + 과립화
     */
    render(): Uint8Array;
    reset(): void;
    set_physics(dt: number, evaporation: number, viscosity: number, pressure: number, iterations: number): void;
    set_pigment_props(adhesion: number, granularity: number): void;
    set_show_texture(show: boolean): void;
    /**
     * 시뮬레이션 스텝
     */
    step(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_watercolorengine_free: (a: number, b: number) => void;
    readonly watercolorengine_apply_brush: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => void;
    readonly watercolorengine_apply_brush_stroke: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => void;
    readonly watercolorengine_grid_size: (a: number) => number;
    readonly watercolorengine_load_paper_texture: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly watercolorengine_new: () => number;
    readonly watercolorengine_render: (a: number) => [number, number];
    readonly watercolorengine_reset: (a: number) => void;
    readonly watercolorengine_set_physics: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly watercolorengine_set_pigment_props: (a: number, b: number, c: number) => void;
    readonly watercolorengine_set_show_texture: (a: number, b: number) => void;
    readonly watercolorengine_step: (a: number) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
