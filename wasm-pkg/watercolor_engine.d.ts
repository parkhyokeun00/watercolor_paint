/* tslint:disable */
/* eslint-disable */

export class WatercolorEngine {
    free(): void;
    [Symbol.dispose](): void;
    apply_blend_brush_stroke(x0: number, y0: number, x1: number, y1: number, size: number, blend_strength: number, velocity: number): void;
    apply_brush(cx: number, cy: number, size: number, water: number, pigment_amount: number, r: number, g: number, b: number, angle: number, pressure: number): void;
    apply_brush_stroke(x0: number, y0: number, x1: number, y1: number, size: number, water: number, pigment_amount: number, r: number, g: number, b: number, velocity: number): void;
    apply_fade_brush_stroke(x0: number, y0: number, x1: number, y1: number, size: number, fade_strength: number, velocity: number): void;
    apply_water_brush_stroke(x0: number, y0: number, x1: number, y1: number, size: number, water_amount: number, flow_strength: number, velocity: number): void;
    get_height(): number;
    get_width(): number;
    load_paper_texture(data: Uint8Array, tex_w: number, tex_h: number): void;
    constructor(w: number, h: number);
    render(): Uint8Array;
    reset(): void;
    set_physics(dt: number, evaporation: number, viscosity: number, pressure: number, iterations: number): void;
    set_pigment_props(adhesion: number, granularity: number): void;
    set_show_texture(show: boolean): void;
    step(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_watercolorengine_free: (a: number, b: number) => void;
    readonly watercolorengine_apply_blend_brush_stroke: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
    readonly watercolorengine_apply_brush: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => void;
    readonly watercolorengine_apply_brush_stroke: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => void;
    readonly watercolorengine_apply_fade_brush_stroke: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
    readonly watercolorengine_apply_water_brush_stroke: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => void;
    readonly watercolorengine_get_height: (a: number) => number;
    readonly watercolorengine_get_width: (a: number) => number;
    readonly watercolorengine_load_paper_texture: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly watercolorengine_new: (a: number, b: number) => number;
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
