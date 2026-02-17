use wasm_bindgen::prelude::*;

const GRID_SIZE: usize = 300;
const TOTAL: usize = GRID_SIZE * GRID_SIZE;

#[wasm_bindgen]
pub struct WatercolorEngine {
    // 유체 필드
    h: Vec<f32>,    // 수위 (water height)
    u: Vec<f32>,    // x 속도
    v: Vec<f32>,    // y 속도
    p: Vec<f32>,    // 압력
    mask: Vec<f32>, // 젖은 영역 마스크

    // RGB 채널 기반 안료
    gr: Vec<f32>, // 부유 안료 R (suspended)
    gg: Vec<f32>, // 부유 안료 G
    gb: Vec<f32>, // 부유 안료 B
    dr: Vec<f32>, // 침전 안료 R (deposited)
    dg: Vec<f32>, // 침전 안료 G
    db: Vec<f32>, // 침전 안료 B

    // 종이 텍스처 (외부 로드 또는 절차적 생성)
    paper_h: Vec<f32>,
    paper_loaded: bool,

    // 렌더링 버퍼
    pixels: Vec<u8>,

    // 물리 파라미터
    dt: f32,
    evaporation: f32,
    viscosity: f32,
    pressure: f32,
    iterations: u32,

    // 안료 파라미터
    adhesion: f32,
    granularity: f32,

    // 텍스처 표시
    show_texture: bool,

    // 시드
    seed: u32,
}

// 간단한 의사 난수 생성기
fn simple_rng(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32) / (u32::MAX as f32)
}

#[wasm_bindgen]
impl WatercolorEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WatercolorEngine {
        let mut seed: u32 = 42;

        // 절차적 종이 텍스처 (외부 로드 전 기본값)
        let mut paper_h = vec![0.5f32; TOTAL];
        for i in 0..GRID_SIZE {
            for j in 0..GRID_SIZE {
                let idx = i * GRID_SIZE + j;
                paper_h[idx] = simple_rng(&mut seed) * 0.3
                    + (i as f32 * 0.08).sin() * 0.1
                    + (j as f32 * 0.12).cos() * 0.1
                    + 0.4;
            }
        }

        WatercolorEngine {
            h: vec![0.0; TOTAL],
            u: vec![0.0; TOTAL],
            v: vec![0.0; TOTAL],
            p: vec![0.0; TOTAL],
            mask: vec![0.0; TOTAL],
            gr: vec![0.0; TOTAL],
            gg: vec![0.0; TOTAL],
            gb: vec![0.0; TOTAL],
            dr: vec![0.0; TOTAL],
            dg: vec![0.0; TOTAL],
            db: vec![0.0; TOTAL],
            paper_h,
            paper_loaded: false,
            pixels: vec![255u8; TOTAL * 4],
            dt: 0.15,
            evaporation: 0.002,
            viscosity: 0.05,
            pressure: 5.0,
            iterations: 10,
            adhesion: 0.05,
            granularity: 0.8,
            show_texture: true,
            seed,
        }
    }

    /// 외부 종이 텍스처 로드 (그레이스케일 높이맵, 0~1 범위)
    pub fn load_paper_texture(&mut self, data: &[u8], width: u32, height: u32) {
        let w = width as usize;
        let h = height as usize;
        for i in 0..GRID_SIZE {
            for j in 0..GRID_SIZE {
                let src_x = (j * w) / GRID_SIZE;
                let src_y = (i * h) / GRID_SIZE;
                let src_idx = (src_y * w + src_x) * 4; // RGBA
                if src_idx + 2 < data.len() {
                    // 그레이스케일 변환 (R*0.3 + G*0.59 + B*0.11)
                    let gray = (data[src_idx] as f32 * 0.3
                        + data[src_idx + 1] as f32 * 0.59
                        + data[src_idx + 2] as f32 * 0.11)
                        / 255.0;
                    self.paper_h[i * GRID_SIZE + j] = gray;
                }
            }
        }
        self.paper_loaded = true;
    }

    /// 물리 엔진 파라미터 설정
    pub fn set_physics(
        &mut self,
        dt: f32,
        evaporation: f32,
        viscosity: f32,
        pressure: f32,
        iterations: u32,
    ) {
        self.dt = dt;
        self.evaporation = evaporation;
        self.viscosity = viscosity;
        self.pressure = pressure;
        self.iterations = iterations;
    }

    /// 안료 특성 파라미터 설정
    pub fn set_pigment_props(&mut self, adhesion: f32, granularity: f32) {
        self.adhesion = adhesion;
        self.granularity = granularity;
    }

    /// 텍스처 표시 토글
    pub fn set_show_texture(&mut self, show: bool) {
        self.show_texture = show;
    }

    /// 수채화 브러시 적용 — RGB 색상으로 직접 그리기
    /// r, g, b: 0.0~1.0 범위의 색상값
    pub fn apply_brush(
        &mut self,
        cx: i32,
        cy: i32,
        size: i32,
        water: f32,
        pigment_amount: f32,
        r: f32,
        g: f32,
        b: f32,
    ) {
        let radius = size as f32;
        let sigma = radius * 0.4;
        let sigma2 = sigma * sigma;

        for i in -size..=size {
            for j in -size..=size {
                let tx = cx + i;
                let ty = cy + j;
                if tx < 0 || tx >= GRID_SIZE as i32 || ty < 0 || ty >= GRID_SIZE as i32 {
                    continue;
                }

                let idx = ty as usize * GRID_SIZE + tx as usize;
                let dist_sq = (i * i + j * j) as f32;
                let dist = dist_sq.sqrt();
                if dist > radius {
                    continue;
                }

                // 가우시안 감쇠
                let gaussian = (-dist_sq / (2.0 * sigma2)).exp();

                // 종이 텍스처 반응
                let paper_response = 0.7 + 0.3 * (1.0 - self.paper_h[idx]);

                // Edge darkening (수채화 bloom)
                let norm_dist = dist / radius;
                let edge_factor = if norm_dist > 0.6 {
                    1.0 + (norm_dist - 0.6) * 0.8
                } else {
                    1.0
                };

                let brush_factor = gaussian * paper_response;

                // 수분
                self.h[idx] += water * brush_factor * 0.8;

                // RGB 안료 침전
                let pig_factor = pigment_amount * brush_factor * edge_factor * 0.6;
                self.gr[idx] += (1.0 - r) * pig_factor; // 감산 혼합용 (흰 종이 기반)
                self.gg[idx] += (1.0 - g) * pig_factor;
                self.gb[idx] += (1.0 - b) * pig_factor;

                self.mask[idx] = 1.0;
            }
        }
    }

    /// 브러시 보간 (이전→현재 위치 부드러운 연결)
    pub fn apply_brush_stroke(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        size: i32,
        water: f32,
        pigment_amount: f32,
        r: f32,
        g: f32,
        b: f32,
    ) {
        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        let steps = (length * 2.0).max(1.0) as i32;

        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = x0 as f32 + dx * t;
            let y = y0 as f32 + dy * t;
            let attenuation = 1.0 - t * 0.15;
            self.apply_brush(
                x as i32,
                y as i32,
                size,
                water * attenuation,
                pigment_amount * attenuation,
                r,
                g,
                b,
            );
        }
    }

    /// 1 프레임 시뮬레이션 스텝
    pub fn step(&mut self) {
        self.update_velocities();
        self.relax_divergence();
        self.move_fluid();
        self.deposition();
    }

    /// 감산 혼합 렌더링 → RGBA 픽셀 반환
    pub fn render(&mut self) -> Vec<u8> {
        for i in 0..TOTAL {
            let offset = i * 4;

            // 침전된 안료의 감산 혼합 (흰 종이 - 안료 흡수)
            let total_r = self.dr[i] + self.gr[i] * 0.3; // 부유 안료도 살짝 보임
            let total_g = self.dg[i] + self.gg[i] * 0.3;
            let total_b = self.db[i] + self.gb[i] * 0.3;

            // 흰 종이(1.0)에서 안료 흡수분 빼기
            let mut out_r = (1.0 - total_r).max(0.0).min(1.0);
            let mut out_g = (1.0 - total_g).max(0.0).min(1.0);
            let mut out_b = (1.0 - total_b).max(0.0).min(1.0);

            // 종이 텍스처 효과
            if self.show_texture {
                let tex = 0.85 + self.paper_h[i] * 0.15;
                out_r *= tex;
                out_g *= tex;
                out_b *= tex;
            }

            self.pixels[offset] = (out_r * 255.0) as u8;
            self.pixels[offset + 1] = (out_g * 255.0) as u8;
            self.pixels[offset + 2] = (out_b * 255.0) as u8;
            self.pixels[offset + 3] = 255;
        }

        self.pixels.clone()
    }

    /// 그리드 크기 반환
    pub fn grid_size(&self) -> u32 {
        GRID_SIZE as u32
    }

    /// 캔버스 초기화
    pub fn reset(&mut self) {
        self.h.iter_mut().for_each(|v| *v = 0.0);
        self.u.iter_mut().for_each(|v| *v = 0.0);
        self.v.iter_mut().for_each(|v| *v = 0.0);
        self.p.iter_mut().for_each(|v| *v = 0.0);
        self.mask.iter_mut().for_each(|v| *v = 0.0);
        self.gr.iter_mut().for_each(|v| *v = 0.0);
        self.gg.iter_mut().for_each(|v| *v = 0.0);
        self.gb.iter_mut().for_each(|v| *v = 0.0);
        self.dr.iter_mut().for_each(|v| *v = 0.0);
        self.dg.iter_mut().for_each(|v| *v = 0.0);
        self.db.iter_mut().for_each(|v| *v = 0.0);
    }
}

// === 내부 시뮬레이션 메서드 ===
impl WatercolorEngine {
    fn update_velocities(&mut self) {
        let friction = 1.0 - self.viscosity;
        for i in 1..(GRID_SIZE - 1) {
            for j in 1..(GRID_SIZE - 1) {
                let idx = i * GRID_SIZE + j;
                let dhdx = (self.h[idx + 1] + self.paper_h[idx + 1])
                    - (self.h[idx - 1] + self.paper_h[idx - 1]);
                let dhdy = (self.h[idx + GRID_SIZE] + self.paper_h[idx + GRID_SIZE])
                    - (self.h[idx - GRID_SIZE] + self.paper_h[idx - GRID_SIZE]);
                self.u[idx] += -1.5 * dhdx * self.dt;
                self.v[idx] += -1.5 * dhdy * self.dt;
                self.u[idx] *= friction;
                self.v[idx] *= friction;
            }
        }
    }

    fn relax_divergence(&mut self) {
        for _ in 0..self.iterations {
            for i in 1..(GRID_SIZE - 1) {
                for j in 1..(GRID_SIZE - 1) {
                    let idx = i * GRID_SIZE + j;
                    let div = (self.u[idx + 1] - self.u[idx - 1] + self.v[idx + GRID_SIZE]
                        - self.v[idx - GRID_SIZE])
                        * 0.5;
                    self.p[idx] -= div * self.pressure;
                    if self.h[idx] > 0.01 {
                        self.p[idx] -= self.evaporation * (1.0 - self.mask[idx]) * 5.0;
                    }
                }
            }
        }
    }

    fn move_fluid(&mut self) {
        let mut next_h = vec![0.0f32; TOTAL];
        let mut next_r = vec![0.0f32; TOTAL];
        let mut next_g = vec![0.0f32; TOTAL];
        let mut next_b = vec![0.0f32; TOTAL];

        for i in 1..(GRID_SIZE - 1) {
            for j in 1..(GRID_SIZE - 1) {
                let idx = i * GRID_SIZE + j;
                if self.h[idx] <= 0.0 {
                    continue;
                }

                let prev_i = (i as f32 - self.u[idx] * self.dt)
                    .max(0.0)
                    .min((GRID_SIZE - 1) as f32);
                let prev_j = (j as f32 - self.v[idx] * self.dt)
                    .max(0.0)
                    .min((GRID_SIZE - 1) as f32);
                let prev_idx = (prev_i.round() as usize) * GRID_SIZE + (prev_j.round() as usize);

                next_h[idx] = self.h[prev_idx] * (1.0 - self.evaporation);
                next_r[idx] = self.gr[prev_idx];
                next_g[idx] = self.gg[prev_idx];
                next_b[idx] = self.gb[prev_idx];
            }
        }

        self.h.copy_from_slice(&next_h);
        self.gr.copy_from_slice(&next_r);
        self.gg.copy_from_slice(&next_g);
        self.gb.copy_from_slice(&next_b);
    }

    fn deposition(&mut self) {
        for i in 0..TOTAL {
            if self.h[i] < 0.01 {
                continue;
            }
            let speed = (self.u[i] * self.u[i] + self.v[i] * self.v[i]).sqrt();
            let dep_rate = self.adhesion
                * (1.0 / (speed + 1.0))
                * (1.0 + self.granularity * (1.0 - self.paper_h[i]));
            let rate = dep_rate * self.dt;

            let amt_r = self.gr[i] * rate;
            let amt_g = self.gg[i] * rate;
            let amt_b = self.gb[i] * rate;

            self.dr[i] += amt_r;
            self.dg[i] += amt_g;
            self.db[i] += amt_b;
            self.gr[i] -= amt_r;
            self.gg[i] -= amt_g;
            self.gb[i] -= amt_b;
        }
    }
}
