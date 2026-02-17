/* @ts-self-types="./watercolor_engine.d.ts" */

export class WatercolorEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WatercolorEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_watercolorengine_free(ptr, 0);
    }
    /**
     * 고급 수채화 브러시 — 붓털 노이즈 + 종이 반응 + 타원형 스탬프
     * @param {number} cx
     * @param {number} cy
     * @param {number} size
     * @param {number} water
     * @param {number} pigment_amount
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} angle
     * @param {number} pressure
     */
    apply_brush(cx, cy, size, water, pigment_amount, r, g, b, angle, pressure) {
        wasm.watercolorengine_apply_brush(this.__wbg_ptr, cx, cy, size, water, pigment_amount, r, g, b, angle, pressure);
    }
    /**
     * 고급 스트로크 보간 — 속도 감응 + 안료 소진 + 방향 추적
     * @param {number} x0
     * @param {number} y0
     * @param {number} x1
     * @param {number} y1
     * @param {number} size
     * @param {number} water
     * @param {number} pigment_amount
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} velocity
     */
    apply_brush_stroke(x0, y0, x1, y1, size, water, pigment_amount, r, g, b, velocity) {
        wasm.watercolorengine_apply_brush_stroke(this.__wbg_ptr, x0, y0, x1, y1, size, water, pigment_amount, r, g, b, velocity);
    }
    /**
     * @returns {number}
     */
    grid_size() {
        const ret = wasm.watercolorengine_grid_size(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * 외부 종이 텍스처 로드
     * @param {Uint8Array} data
     * @param {number} width
     * @param {number} height
     */
    load_paper_texture(data, width, height) {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.watercolorengine_load_paper_texture(this.__wbg_ptr, ptr0, len0, width, height);
    }
    constructor() {
        const ret = wasm.watercolorengine_new();
        this.__wbg_ptr = ret >>> 0;
        WatercolorEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * 렌더링 — 고급 감산 혼합 + 젖은 영역 반짝임 + 과립화
     * @returns {Uint8Array}
     */
    render() {
        const ret = wasm.watercolorengine_render(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    reset() {
        wasm.watercolorengine_reset(this.__wbg_ptr);
    }
    /**
     * @param {number} dt
     * @param {number} evaporation
     * @param {number} viscosity
     * @param {number} pressure
     * @param {number} iterations
     */
    set_physics(dt, evaporation, viscosity, pressure, iterations) {
        wasm.watercolorengine_set_physics(this.__wbg_ptr, dt, evaporation, viscosity, pressure, iterations);
    }
    /**
     * @param {number} adhesion
     * @param {number} granularity
     */
    set_pigment_props(adhesion, granularity) {
        wasm.watercolorengine_set_pigment_props(this.__wbg_ptr, adhesion, granularity);
    }
    /**
     * @param {boolean} show
     */
    set_show_texture(show) {
        wasm.watercolorengine_set_show_texture(this.__wbg_ptr, show);
    }
    /**
     * 시뮬레이션 스텝
     */
    step() {
        wasm.watercolorengine_step(this.__wbg_ptr);
    }
}
if (Symbol.dispose) WatercolorEngine.prototype[Symbol.dispose] = WatercolorEngine.prototype.free;

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_be289d5034ed271b: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./watercolor_engine_bg.js": import0,
    };
}

const WatercolorEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_watercolorengine_free(ptr >>> 0, 1));

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('watercolor_engine_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
