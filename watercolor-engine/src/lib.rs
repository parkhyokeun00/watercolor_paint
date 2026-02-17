use wasm_bindgen::prelude::*;

const GRID_SIZE: usize = 300;
const TOTAL: usize = GRID_SIZE * GRID_SIZE;

#[wasm_bindgen]
pub struct WatercolorEngine {
    // 유체 필드
    h: Vec<f32>,    // 수위
    u: Vec<f32>,    // x 속도
    v: Vec<f32>,    // y 속도
    p: Vec<f32>,    // 압력
    mask: Vec<f32>, // 젖은 영역 마스크

    // RGB 채널 안료
    gr: Vec<f32>,
    gg: Vec<f32>,
    gb: Vec<f32>, // 부유 (suspended)
    dr: Vec<f32>,
    dg: Vec<f32>,
    db: Vec<f32>, // 침전 (deposited)

    // 종이 텍스처
    paper_h: Vec<f32>,

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

    // 난수 시드
    seed: u32,
}

fn rng(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32) / (u32::MAX as f32)
}

/// 부드러운 clamped smoothstep
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).max(0.0).min(1.0);
    t * t * (3.0 - 2.0 * t)
}

#[wasm_bindgen]
impl WatercolorEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WatercolorEngine {
        let mut seed: u32 = 42;

        // 절차적 종이 텍스처 (multi-octave noise로 더 자연스럽게)
        let mut paper_h = vec![0.5f32; TOTAL];
        for i in 0..GRID_SIZE {
            for j in 0..GRID_SIZE {
                let idx = i * GRID_SIZE + j;
                // 다중 옥타브 노이즈
                let n1 = rng(&mut seed) * 0.25;
                let n2 = (i as f32 * 0.05).sin() * (j as f32 * 0.07).cos() * 0.08;
                let n3 = (i as f32 * 0.15).cos() * (j as f32 * 0.11).sin() * 0.04;
                let n4 = ((i as f32 * 0.3).sin() + (j as f32 * 0.25).cos()) * 0.02;
                paper_h[idx] = (0.45 + n1 + n2 + n3 + n4).max(0.0).min(1.0);
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

    /// 외부 종이 텍스처 로드
    pub fn load_paper_texture(&mut self, data: &[u8], width: u32, height: u32) {
        let w = width as usize;
        let h = height as usize;
        for i in 0..GRID_SIZE {
            for j in 0..GRID_SIZE {
                let src_x = (j * w) / GRID_SIZE;
                let src_y = (i * h) / GRID_SIZE;
                let src_idx = (src_y * w + src_x) * 4;
                if src_idx + 2 < data.len() {
                    let gray = (data[src_idx] as f32 * 0.3
                        + data[src_idx + 1] as f32 * 0.59
                        + data[src_idx + 2] as f32 * 0.11)
                        / 255.0;
                    self.paper_h[i * GRID_SIZE + j] = gray;
                }
            }
        }
    }

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

    pub fn set_pigment_props(&mut self, adhesion: f32, granularity: f32) {
        self.adhesion = adhesion;
        self.granularity = granularity;
    }

    pub fn set_show_texture(&mut self, show: bool) {
        self.show_texture = show;
    }

    /// 고급 수채화 브러시 — 붓털 노이즈 + 종이 반응 + 타원형 스탬프
    pub fn apply_brush(
        &mut self,
        cx: i32,
        cy: i32,
        size: f32, // float 크기 (속도 감응)
        water: f32,
        pigment_amount: f32,
        r: f32,
        g: f32,
        b: f32,
        angle: f32,    // 이동 방향 각도 (라디안)
        pressure: f32, // 필압 (0~1)
    ) {
        let radius = size.max(0.5);
        let isize = radius.ceil() as i32;
        let sigma = radius * 0.45;
        let sigma2 = sigma * sigma;

        // 붓 방향에 따른 타원형: 이동 방향으로 약간 늘어남
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let aspect = 0.7; // 짧은 축 비율

        for i in -isize..=isize {
            for j in -isize..=isize {
                let tx = cx + i;
                let ty = cy + j;
                if tx < 0 || tx >= GRID_SIZE as i32 || ty < 0 || ty >= GRID_SIZE as i32 {
                    continue;
                }

                let idx = ty as usize * GRID_SIZE + tx as usize;
                let fi = i as f32;
                let fj = j as f32;

                // 타원 좌표 변환 (회전 + 비율)
                let rot_x = fi * cos_a + fj * sin_a;
                let rot_y = (-fi * sin_a + fj * cos_a) / aspect;
                let dist_sq = rot_x * rot_x + rot_y * rot_y;
                let dist = dist_sq.sqrt();
                if dist > radius {
                    continue;
                }

                // 가우시안 감쇠
                let gaussian = (-dist_sq / (2.0 * sigma2)).exp();

                // 붓털 노이즈 (종이 위치 기반 결정적 노이즈)
                let bristle_noise = {
                    let hash =
                        ((tx as u32).wrapping_mul(73856093)) ^ ((ty as u32).wrapping_mul(19349663));
                    let n = (hash as f32) / (u32::MAX as f32);
                    0.6 + n * 0.4 // 0.6 ~ 1.0
                };

                // 종이 텍스처 반응: 골에 잉크가 고임
                let paper_val = self.paper_h[idx];
                let paper_response = 0.6 + 0.4 * (1.0 - paper_val);

                // Edge darkening (수채화 bloom) — 부드러운 smoothstep
                let norm_dist = dist / radius;
                let edge_factor = 1.0 + smoothstep(0.5, 0.95, norm_dist) * 0.6;

                // Wet-on-wet: 이미 젖은 영역은 안료가 더 잘 퍼짐
                let wetness = self.h[idx].min(1.0);
                let wet_spread = 1.0 + wetness * 0.4;

                // 최종 브러시 팩터
                let brush_factor = gaussian * paper_response * bristle_noise * pressure;

                // 수분
                self.h[idx] += water * brush_factor * wet_spread * 0.7;

                // 안료 (감산 혼합)
                let pig_factor = pigment_amount * brush_factor * edge_factor * 0.5;
                self.gr[idx] += (1.0 - r) * pig_factor;
                self.gg[idx] += (1.0 - g) * pig_factor;
                self.gb[idx] += (1.0 - b) * pig_factor;

                self.mask[idx] = 1.0;
            }
        }
    }

    /// 고급 스트로크 보간 — 속도 감응 + 안료 소진 + 방향 추적
    pub fn apply_brush_stroke(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        size: f32, // base size (JS에서 속도 감응 적용)
        water: f32,
        pigment_amount: f32,
        r: f32,
        g: f32,
        b: f32,
        velocity: f32, // 마우스 속도
    ) {
        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);

        // 속도에 따른 필압 (빠르면 필압 낮아짐 = 붓이 스쳐감)
        let pressure = (1.0 / (1.0 + velocity * 0.08)).max(0.2).min(1.0);

        // 보간 밀도: 붓 크기에 비례 (작은 붓 = 더 촘촘한 보간)
        let step_size = (size * 0.3).max(0.5);
        let steps = (length / step_size).ceil().max(1.0) as i32;

        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = x0 as f32 + dx * t;
            let y = y0 as f32 + dy * t;

            // 안료 소진: 스트로크 내에서 자연스러운 감쇠
            let attenuation = 1.0 - t * 0.2;
            // 약간의 떨림 (실제 붓의 흔들림)
            let jitter_x = ((x * 17.0 + y * 31.0).sin() * 0.5) as i32;
            let jitter_y = ((x * 23.0 + y * 13.0).cos() * 0.5) as i32;

            self.apply_brush(
                x as i32 + jitter_x,
                y as i32 + jitter_y,
                size,
                water * attenuation,
                pigment_amount * attenuation,
                r,
                g,
                b,
                angle,
                pressure * attenuation,
            );
        }
    }

    /// 시뮬레이션 스텝
    pub fn step(&mut self) {
        self.update_velocities();
        self.relax_divergence();
        self.move_fluid();
        self.deposition();
        self.capillary_flow();
        self.back_run();
    }

    /// 렌더링 — 고급 감산 혼합 + 젖은 영역 반짝임 + 과립화
    pub fn render(&mut self) -> Vec<u8> {
        for i in 0..TOTAL {
            let offset = i * 4;

            // 침전 + 부유 안료 합산
            let total_r = self.dr[i] + self.gr[i] * 0.4;
            let total_g = self.dg[i] + self.gg[i] * 0.4;
            let total_b = self.db[i] + self.gb[i] * 0.4;

            // 감산 혼합
            let mut out_r = (1.0 - total_r).max(0.0).min(1.0);
            let mut out_g = (1.0 - total_g).max(0.0).min(1.0);
            let mut out_b = (1.0 - total_b).max(0.0).min(1.0);

            // 젖은 영역 광택 효과 (wet gloss)
            let wetness = self.h[i].min(1.0);
            if wetness > 0.05 {
                let gloss = 1.0 + wetness * 0.08;
                out_r = (out_r * gloss).min(1.0);
                out_g = (out_g * gloss).min(1.0);
                out_b = (out_b * gloss).min(1.0);
            }

            // 종이 텍스처
            if self.show_texture {
                let paper = self.paper_h[i];
                // 안료가 있는 곳은 텍스처 효과 더 강하게 (과립 효과)
                let has_paint = (total_r + total_g + total_b).min(1.0);
                let tex_strength = 0.06 + has_paint * self.granularity * 0.1;
                let tex = 1.0 - tex_strength + paper * tex_strength * 2.0;
                out_r *= tex;
                out_g *= tex;
                out_b *= tex;
            }

            self.pixels[offset] = (out_r * 255.0).min(255.0).max(0.0) as u8;
            self.pixels[offset + 1] = (out_g * 255.0).min(255.0).max(0.0) as u8;
            self.pixels[offset + 2] = (out_b * 255.0).min(255.0).max(0.0) as u8;
            self.pixels[offset + 3] = 255;
        }
        self.pixels.clone()
    }

    pub fn grid_size(&self) -> u32 {
        GRID_SIZE as u32
    }

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

// === 내부 시뮬레이션 ===
impl WatercolorEngine {
    fn update_velocities(&mut self) {
        let friction = 1.0 - self.viscosity;
        for i in 1..(GRID_SIZE - 1) {
            for j in 1..(GRID_SIZE - 1) {
                let idx = i * GRID_SIZE + j;
                if self.h[idx] < 0.001 {
                    continue;
                }
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

    /// 유체 이동 — 쌍선형 보간 (bilinear interpolation)
    fn move_fluid(&mut self) {
        let mut next_h = vec![0.0f32; TOTAL];
        let mut next_r = vec![0.0f32; TOTAL];
        let mut next_g = vec![0.0f32; TOTAL];
        let mut next_b = vec![0.0f32; TOTAL];

        let gs = GRID_SIZE as f32;
        for i in 1..(GRID_SIZE - 1) {
            for j in 1..(GRID_SIZE - 1) {
                let idx = i * GRID_SIZE + j;
                if self.h[idx] <= 0.0 {
                    continue;
                }

                let pi = (i as f32 - self.u[idx] * self.dt).max(1.0).min(gs - 2.0);
                let pj = (j as f32 - self.v[idx] * self.dt).max(1.0).min(gs - 2.0);

                // 쌍선형 보간
                let i0 = pi.floor() as usize;
                let j0 = pj.floor() as usize;
                let i1 = i0 + 1;
                let j1 = j0 + 1;
                let si = pi - i0 as f32;
                let sj = pj - j0 as f32;

                let w00 = (1.0 - si) * (1.0 - sj);
                let w10 = si * (1.0 - sj);
                let w01 = (1.0 - si) * sj;
                let w11 = si * sj;

                let idx00 = i0 * GRID_SIZE + j0;
                let idx10 = i1 * GRID_SIZE + j0;
                let idx01 = i0 * GRID_SIZE + j1;
                let idx11 = i1 * GRID_SIZE + j1;

                next_h[idx] = (self.h[idx00] * w00
                    + self.h[idx10] * w10
                    + self.h[idx01] * w01
                    + self.h[idx11] * w11)
                    * (1.0 - self.evaporation);
                next_r[idx] = self.gr[idx00] * w00
                    + self.gr[idx10] * w10
                    + self.gr[idx01] * w01
                    + self.gr[idx11] * w11;
                next_g[idx] = self.gg[idx00] * w00
                    + self.gg[idx10] * w10
                    + self.gg[idx01] * w01
                    + self.gg[idx11] * w11;
                next_b[idx] = self.gb[idx00] * w00
                    + self.gb[idx10] * w10
                    + self.gb[idx01] * w01
                    + self.gb[idx11] * w11;
            }
        }

        self.h.copy_from_slice(&next_h);
        self.gr.copy_from_slice(&next_r);
        self.gg.copy_from_slice(&next_g);
        self.gb.copy_from_slice(&next_b);
    }

    /// 안료 침전 — 과립화 강화
    fn deposition(&mut self) {
        for i in 0..TOTAL {
            if self.h[i] < 0.01 {
                continue;
            }
            let speed = (self.u[i] * self.u[i] + self.v[i] * self.v[i]).sqrt();
            let paper_val = self.paper_h[i];
            // 종이 골에 더 많이 침전 + 속도 역비례
            let dep_rate = self.adhesion
                * (1.0 / (speed + 0.5))
                * (1.0 + self.granularity * (1.0 - paper_val) * 1.5);
            let rate = (dep_rate * self.dt).min(0.5);

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

    /// 모세관 확산 — 젖은 영역 가장자리로 물이 스며듦
    fn capillary_flow(&mut self) {
        for i in 1..(GRID_SIZE - 1) {
            for j in 1..(GRID_SIZE - 1) {
                let idx = i * GRID_SIZE + j;
                if self.h[idx] < 0.02 {
                    continue;
                }

                // 4방향 이웃 검사 — 건조한 이웃에 소량 흡수
                let neighbors = [idx - GRID_SIZE, idx + GRID_SIZE, idx - 1, idx + 1];
                for &ni in &neighbors {
                    if self.h[ni] < self.h[idx] {
                        let diff = (self.h[idx] - self.h[ni]) * 0.02;
                        // 종이 텍스처에 따라 흡수율 다름
                        let cap_rate = diff * (1.0 - self.paper_h[ni] * 0.5);
                        if cap_rate > 0.0001 {
                            self.h[ni] += cap_rate;
                            self.h[idx] -= cap_rate;
                            // 안료도 미량 이동
                            let pig_move = cap_rate * 0.3;
                            let ratio = pig_move / (self.h[idx] + 0.001);
                            self.gr[ni] += self.gr[idx] * ratio;
                            self.gg[ni] += self.gg[idx] * ratio;
                            self.gb[ni] += self.gb[idx] * ratio;
                            self.mask[ni] = (self.mask[ni] + 0.1).min(1.0);
                        }
                    }
                }
            }
        }
    }

    /// 역류 효과 — 마르면서 안료가 가장자리로 밀림 (cauliflower effect)
    fn back_run(&mut self) {
        for i in 1..(GRID_SIZE - 1) {
            for j in 1..(GRID_SIZE - 1) {
                let idx = i * GRID_SIZE + j;
                // 물이 거의 마른 영역 (0.005 ~ 0.05)
                if self.h[idx] < 0.005 || self.h[idx] > 0.05 {
                    continue;
                }

                let neighbors = [idx - GRID_SIZE, idx + GRID_SIZE, idx - 1, idx + 1];

                // 가장 젖은 이웃 방향으로 안료 약간 이동
                let mut max_h = self.h[idx];
                let mut max_ni = idx;
                for &ni in &neighbors {
                    if self.h[ni] > max_h {
                        max_h = self.h[ni];
                        max_ni = ni;
                    }
                }

                if max_ni != idx {
                    let push = 0.005;
                    self.dr[max_ni] += self.gr[idx] * push;
                    self.dg[max_ni] += self.gg[idx] * push;
                    self.db[max_ni] += self.gb[idx] * push;
                }
            }
        }
    }
}
