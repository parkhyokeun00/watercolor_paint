use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WatercolorEngine {
    width: usize,
    height: usize,
    total: usize,

    // 유체 필드
    h: Vec<f32>, // 수위
    u: Vec<f32>, // x 속도
    v: Vec<f32>, // y 속도
    p: Vec<f32>, // 압력
    mask: Vec<f32>,

    // RGB 안료
    gr: Vec<f32>,
    gg: Vec<f32>,
    gb: Vec<f32>,
    dr: Vec<f32>,
    dg: Vec<f32>,
    db: Vec<f32>,

    // 종이 텍스처
    paper_h: Vec<f32>,
    paper_render: Vec<f32>,

    // 렌더링 버퍼
    pixels: Vec<u8>,

    // 물리
    dt: f32,
    evaporation: f32,
    viscosity: f32,
    pressure: f32,
    iterations: u32,

    // 안료
    adhesion: f32,
    granularity: f32,

    show_texture: bool,
    seed: u32,
}

fn rng(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32) / (u32::MAX as f32)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).max(0.0).min(1.0);
    t * t * (3.0 - 2.0 * t)
}

#[wasm_bindgen]
impl WatercolorEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(w: u32, h: u32) -> WatercolorEngine {
        let width = w as usize;
        let height = h as usize;
        let total = width * height;
        let mut seed: u32 = 42;

        let mut paper_h = vec![0.5f32; total];
        for i in 0..height {
            for j in 0..width {
                let idx = i * width + j;
                let n1 = rng(&mut seed) * 0.25;
                let n2 = (i as f32 * 0.05).sin() * (j as f32 * 0.07).cos() * 0.08;
                let n3 = (i as f32 * 0.15).cos() * (j as f32 * 0.11).sin() * 0.04;
                let n4 = ((i as f32 * 0.3).sin() + (j as f32 * 0.25).cos()) * 0.02;
                paper_h[idx] = (0.45 + n1 + n2 + n3 + n4).max(0.0).min(1.0);
            }
        }
        let paper_render = paper_h.clone();

        WatercolorEngine {
            width,
            height,
            total,
            h: vec![0.0; total],
            u: vec![0.0; total],
            v: vec![0.0; total],
            p: vec![0.0; total],
            mask: vec![0.0; total],
            gr: vec![0.0; total],
            gg: vec![0.0; total],
            gb: vec![0.0; total],
            dr: vec![0.0; total],
            dg: vec![0.0; total],
            db: vec![0.0; total],
            paper_h,
            paper_render,
            pixels: vec![255u8; total * 4],
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

    pub fn get_width(&self) -> u32 {
        self.width as u32
    }
    pub fn get_height(&self) -> u32 {
        self.height as u32
    }

    pub fn load_paper_texture(&mut self, data: &[u8], tex_w: u32, tex_h: u32) {
        let tw = tex_w as usize;
        let th = tex_h as usize;
        for i in 0..self.height {
            for j in 0..self.width {
                let src_x = (j * tw) / self.width;
                let src_y = (i * th) / self.height;
                let src_idx = (src_y * tw + src_x) * 4;
                if src_idx + 2 < data.len() {
                    let gray = (data[src_idx] as f32 * 0.3
                        + data[src_idx + 1] as f32 * 0.59
                        + data[src_idx + 2] as f32 * 0.11)
                        / 255.0;
                    self.paper_h[i * self.width + j] = gray;
                }
            }
        }
        self.rebuild_paper_render_map();
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

    pub fn apply_brush(
        &mut self,
        cx: i32,
        cy: i32,
        size: f32,
        water: f32,
        pigment_amount: f32,
        r: f32,
        g: f32,
        b: f32,
        angle: f32,
        pressure: f32,
    ) {
        let w = self.width as i32;
        let h = self.height as i32;
        let radius = size.max(0.5);
        let isize = radius.ceil() as i32;
        let sigma = radius * 0.45;
        let sigma2 = sigma * sigma;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let aspect = 0.7;

        for di in -isize..=isize {
            for dj in -isize..=isize {
                let tx = cx + di;
                let ty = cy + dj;
                if tx < 0 || tx >= w || ty < 0 || ty >= h {
                    continue;
                }

                let idx = ty as usize * self.width + tx as usize;
                let fi = di as f32;
                let fj = dj as f32;
                let rot_x = fi * cos_a + fj * sin_a;
                let rot_y = (-fi * sin_a + fj * cos_a) / aspect;
                let dist_sq = rot_x * rot_x + rot_y * rot_y;
                let dist = dist_sq.sqrt();
                if dist > radius {
                    continue;
                }

                let gaussian = (-dist_sq / (2.0 * sigma2)).exp();
                let bristle_noise = {
                    let hash =
                        ((tx as u32).wrapping_mul(73856093)) ^ ((ty as u32).wrapping_mul(19349663));
                    0.6 + (hash as f32) / (u32::MAX as f32) * 0.4
                };
                let paper_val = self.paper_h[idx];
                let paper_response = 0.6 + 0.4 * (1.0 - paper_val);
                let norm_dist = dist / radius;
                let edge_factor = 1.0 + smoothstep(0.5, 0.95, norm_dist) * 0.6;
                let wetness = self.h[idx].min(1.0);
                let wet_spread = 1.0 + wetness * 0.4;
                let brush_factor = gaussian * paper_response * bristle_noise * pressure;

                self.h[idx] += water * brush_factor * wet_spread * 0.7;
                let pig_factor = pigment_amount * brush_factor * edge_factor * 0.5;
                self.gr[idx] += (1.0 - r) * pig_factor;
                self.gg[idx] += (1.0 - g) * pig_factor;
                self.gb[idx] += (1.0 - b) * pig_factor;
                self.mask[idx] = 1.0;
            }
        }
    }

    pub fn apply_brush_stroke(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        size: f32,
        water: f32,
        pigment_amount: f32,
        r: f32,
        g: f32,
        b: f32,
        velocity: f32,
    ) {
        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);
        let pressure = (1.0 / (1.0 + velocity * 0.08)).max(0.2).min(1.0);
        let step_size = (size * 0.3).max(0.5);
        let steps = (length / step_size).ceil().max(1.0) as i32;

        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = x0 as f32 + dx * t;
            let y = y0 as f32 + dy * t;
            let attenuation = 1.0 - t * 0.2;
            let jx = ((x * 17.0 + y * 31.0).sin() * 0.5) as i32;
            let jy = ((x * 23.0 + y * 13.0).cos() * 0.5) as i32;
            self.apply_brush(
                x as i32 + jx,
                y as i32 + jy,
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

    pub fn apply_fade_brush_stroke(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        size: f32,
        fade_strength: f32,
        velocity: f32,
    ) {
        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);
        let pressure = (1.0 / (1.0 + velocity * 0.08)).max(0.2).min(1.0);
        let step_size = (size * 0.3).max(0.5);
        let steps = (length / step_size).ceil().max(1.0) as i32;

        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = x0 as f32 + dx * t;
            let y = y0 as f32 + dy * t;
            let attenuation = 1.0 - t * 0.2;
            let jx = ((x * 17.0 + y * 31.0).sin() * 0.5) as i32;
            let jy = ((x * 23.0 + y * 13.0).cos() * 0.5) as i32;
            self.apply_fade_brush(
                x as i32 + jx,
                y as i32 + jy,
                size,
                fade_strength * attenuation,
                angle,
                pressure * attenuation,
            );
        }
    }

    pub fn apply_blend_brush_stroke(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        size: f32,
        blend_strength: f32,
        velocity: f32,
    ) {
        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);
        let pressure = (1.0 / (1.0 + velocity * 0.08)).max(0.2).min(1.0);
        let step_size = (size * 0.3).max(0.5);
        let steps = (length / step_size).ceil().max(1.0) as i32;

        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = x0 as f32 + dx * t;
            let y = y0 as f32 + dy * t;
            let attenuation = 1.0 - t * 0.2;
            let jx = ((x * 17.0 + y * 31.0).sin() * 0.5) as i32;
            let jy = ((x * 23.0 + y * 13.0).cos() * 0.5) as i32;
            self.apply_blend_brush(
                x as i32 + jx,
                y as i32 + jy,
                size,
                blend_strength * attenuation,
                angle,
                pressure * attenuation,
            );
        }
    }

    pub fn apply_water_brush_stroke(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        size: f32,
        water_amount: f32,
        flow_strength: f32,
        velocity: f32,
    ) {
        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);
        let pressure = (1.0 / (1.0 + velocity * 0.08)).max(0.2).min(1.0);
        let step_size = (size * 0.3).max(0.5);
        let steps = (length / step_size).ceil().max(1.0) as i32;

        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let x = x0 as f32 + dx * t;
            let y = y0 as f32 + dy * t;
            let attenuation = 1.0 - t * 0.2;
            let jx = ((x * 17.0 + y * 31.0).sin() * 0.5) as i32;
            let jy = ((x * 23.0 + y * 13.0).cos() * 0.5) as i32;
            self.apply_water_brush(
                x as i32 + jx,
                y as i32 + jy,
                size,
                water_amount * attenuation,
                flow_strength * attenuation,
                angle,
                pressure * attenuation,
            );
        }
    }

    pub fn step(&mut self) {
        self.update_velocities();
        self.relax_divergence();
        self.move_fluid();
        self.deposition();
        self.capillary_flow();
        self.back_run();
    }

    pub fn render(&mut self) -> Vec<u8> {
        for i in 0..self.total {
            let offset = i * 4;
            let total_r = self.dr[i] + self.gr[i] * 0.4;
            let total_g = self.dg[i] + self.gg[i] * 0.4;
            let total_b = self.db[i] + self.gb[i] * 0.4;

            let mut out_r = (1.0 - total_r).max(0.0).min(1.0);
            let mut out_g = (1.0 - total_g).max(0.0).min(1.0);
            let mut out_b = (1.0 - total_b).max(0.0).min(1.0);

            let wetness = self.h[i].min(1.0);
            if wetness > 0.05 {
                let gloss = 1.0 + wetness * 0.08;
                out_r = (out_r * gloss).min(1.0);
                out_g = (out_g * gloss).min(1.0);
                out_b = (out_b * gloss).min(1.0);
            }

            if self.show_texture {
                let paper = self.paper_render[i];
                let has_paint = (total_r + total_g + total_b).min(1.0);
                // 칠해진 영역일수록 텍스처 대비를 줄여 가이드 라인 잔상을 완화
                let tex_fade = (1.0 - has_paint * 0.85).max(0.12);
                let tex_strength = (0.03 + self.granularity * 0.08) * tex_fade;
                // 0.5 중심 대비 방식으로 밝은 라인 편향을 줄임
                let tex = 1.0 + (paper - 0.5) * tex_strength * 1.8;
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
    fn rebuild_paper_render_map(&mut self) {
        // 물리용 거친 텍스처(paper_h)는 유지하고, 렌더용은 부드럽게 재구성
        // 하드 라인이 그대로 보이지 않도록 3x3 박스 블러 + 대비 압축 적용
        let w = self.width;
        let h = self.height;
        if self.paper_render.len() != self.total {
            self.paper_render = vec![0.5; self.total];
        }
        for i in 0..h {
            for j in 0..w {
                let mut sum = 0.0f32;
                let mut cnt = 0.0f32;
                let y0 = i.saturating_sub(1);
                let y1 = (i + 1).min(h - 1);
                let x0 = j.saturating_sub(1);
                let x1 = (j + 1).min(w - 1);
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        sum += self.paper_h[y * w + x];
                        cnt += 1.0;
                    }
                }
                let avg = sum / cnt.max(1.0);
                let compressed = 0.5 + (avg - 0.5) * 0.35;
                self.paper_render[i * w + j] = compressed.max(0.0).min(1.0);
            }
        }
    }

    pub fn apply_fade_brush(
        &mut self,
        cx: i32,
        cy: i32,
        size: f32,
        fade_strength: f32,
        angle: f32,
        pressure: f32,
    ) {
        let w = self.width as i32;
        let h = self.height as i32;
        let radius = size.max(0.5);
        let isize = radius.ceil() as i32;
        let sigma = radius * 0.45;
        let sigma2 = sigma * sigma;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let aspect = 0.7;
        let strength = fade_strength.max(0.0).min(1.0);

        for di in -isize..=isize {
            for dj in -isize..=isize {
                let tx = cx + di;
                let ty = cy + dj;
                if tx < 0 || tx >= w || ty < 0 || ty >= h {
                    continue;
                }

                let idx = ty as usize * self.width + tx as usize;
                let fi = di as f32;
                let fj = dj as f32;
                let rot_x = fi * cos_a + fj * sin_a;
                let rot_y = (-fi * sin_a + fj * cos_a) / aspect;
                let dist_sq = rot_x * rot_x + rot_y * rot_y;
                let dist = dist_sq.sqrt();
                if dist > radius {
                    continue;
                }

                let gaussian = (-dist_sq / (2.0 * sigma2)).exp();
                let paper_val = self.paper_h[idx];
                let paper_response = 0.7 + 0.3 * (1.0 - paper_val);
                let wetness = self.h[idx].min(1.0);
                let wet_boost = 1.0 + wetness * 0.6;
                let fade = (strength * gaussian * paper_response * pressure * wet_boost * 0.5)
                    .max(0.0)
                    .min(0.75);
                let keep = 1.0 - fade;

                self.gr[idx] *= keep;
                self.gg[idx] *= keep;
                self.gb[idx] *= keep;
                self.dr[idx] *= 1.0 - fade * 0.8;
                self.dg[idx] *= 1.0 - fade * 0.8;
                self.db[idx] *= 1.0 - fade * 0.8;
                self.h[idx] *= 1.0 - fade * 0.25;
                self.mask[idx] *= 1.0 - fade * 0.5;
            }
        }
    }

    pub fn apply_blend_brush(
        &mut self,
        cx: i32,
        cy: i32,
        size: f32,
        blend_strength: f32,
        angle: f32,
        pressure: f32,
    ) {
        let w = self.width as i32;
        let h = self.height as i32;
        let radius = size.max(0.5);
        let isize = radius.ceil() as i32;
        let sigma = radius * 0.45;
        let sigma2 = sigma * sigma;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let aspect = 0.7;
        let strength = blend_strength.max(0.0).min(1.0);

        let src_gr = self.gr.clone();
        let src_gg = self.gg.clone();
        let src_gb = self.gb.clone();
        let src_dr = self.dr.clone();
        let src_dg = self.dg.clone();
        let src_db = self.db.clone();

        for di in -isize..=isize {
            for dj in -isize..=isize {
                let tx = cx + di;
                let ty = cy + dj;
                if tx <= 1 || tx >= w - 1 || ty <= 1 || ty >= h - 1 {
                    continue;
                }

                let idx = ty as usize * self.width + tx as usize;
                let fi = di as f32;
                let fj = dj as f32;
                let rot_x = fi * cos_a + fj * sin_a;
                let rot_y = (-fi * sin_a + fj * cos_a) / aspect;
                let dist_sq = rot_x * rot_x + rot_y * rot_y;
                let dist = dist_sq.sqrt();
                if dist > radius {
                    continue;
                }

                let gaussian = (-dist_sq / (2.0 * sigma2)).exp();
                let blend = (strength * gaussian * pressure).max(0.0).min(0.85);
                if blend <= 0.001 {
                    continue;
                }

                let mut sum_gr = 0.0;
                let mut sum_gg = 0.0;
                let mut sum_gb = 0.0;
                let mut sum_dr = 0.0;
                let mut sum_dg = 0.0;
                let mut sum_db = 0.0;
                let mut count: f32 = 0.0;
                for ny in (ty - 1)..=(ty + 1) {
                    for nx in (tx - 1)..=(tx + 1) {
                        let nidx = ny as usize * self.width + nx as usize;
                        sum_gr += src_gr[nidx];
                        sum_gg += src_gg[nidx];
                        sum_gb += src_gb[nidx];
                        sum_dr += src_dr[nidx];
                        sum_dg += src_dg[nidx];
                        sum_db += src_db[nidx];
                        count += 1.0;
                    }
                }
                let inv = 1.0 / count.max(1.0);
                let avg_gr = sum_gr * inv;
                let avg_gg = sum_gg * inv;
                let avg_gb = sum_gb * inv;
                let avg_dr = sum_dr * inv;
                let avg_dg = sum_dg * inv;
                let avg_db = sum_db * inv;

                self.gr[idx] = self.gr[idx] * (1.0 - blend) + avg_gr * blend;
                self.gg[idx] = self.gg[idx] * (1.0 - blend) + avg_gg * blend;
                self.gb[idx] = self.gb[idx] * (1.0 - blend) + avg_gb * blend;
                self.dr[idx] = self.dr[idx] * (1.0 - blend) + avg_dr * blend;
                self.dg[idx] = self.dg[idx] * (1.0 - blend) + avg_dg * blend;
                self.db[idx] = self.db[idx] * (1.0 - blend) + avg_db * blend;
                self.h[idx] += blend * 0.04;
                self.mask[idx] = 1.0;
            }
        }
    }

    pub fn apply_water_brush(
        &mut self,
        cx: i32,
        cy: i32,
        size: f32,
        water_amount: f32,
        flow_strength: f32,
        angle: f32,
        pressure: f32,
    ) {
        let w = self.width as i32;
        let h = self.height as i32;
        let radius = size.max(0.5);
        let isize = radius.ceil() as i32;
        let sigma = radius * 0.45;
        let sigma2 = sigma * sigma;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let aspect = 0.7;
        let flow = flow_strength.max(0.0).min(2.0);

        for di in -isize..=isize {
            for dj in -isize..=isize {
                let tx = cx + di;
                let ty = cy + dj;
                if tx <= 1 || tx >= w - 1 || ty <= 1 || ty >= h - 1 {
                    continue;
                }

                let idx = ty as usize * self.width + tx as usize;
                let fi = di as f32;
                let fj = dj as f32;
                let rot_x = fi * cos_a + fj * sin_a;
                let rot_y = (-fi * sin_a + fj * cos_a) / aspect;
                let dist_sq = rot_x * rot_x + rot_y * rot_y;
                let dist = dist_sq.sqrt();
                if dist > radius {
                    continue;
                }

                let gaussian = (-dist_sq / (2.0 * sigma2)).exp();
                let radial_x = fi / (radius + 0.001);
                let radial_y = fj / (radius + 0.001);
                let edge = smoothstep(0.2, 1.0, dist / radius);
                let add_water = water_amount * gaussian * pressure * 0.45;
                self.h[idx] += add_water;
                self.u[idx] += radial_x * flow * edge * 0.04;
                self.v[idx] += radial_y * flow * edge * 0.04;

                let lift = (gaussian * flow * 0.06).min(0.2);
                let move_r = self.dr[idx] * lift;
                let move_g = self.dg[idx] * lift;
                let move_b = self.db[idx] * lift;
                self.dr[idx] -= move_r;
                self.dg[idx] -= move_g;
                self.db[idx] -= move_b;
                self.gr[idx] += move_r;
                self.gg[idx] += move_g;
                self.gb[idx] += move_b;
                self.mask[idx] = 1.0;
            }
        }
    }

    fn update_velocities(&mut self) {
        let w = self.width;
        let friction = 1.0 - self.viscosity;
        for i in 1..(self.height - 1) {
            for j in 1..(w - 1) {
                let idx = i * w + j;
                if self.h[idx] < 0.001 {
                    continue;
                }
                let dhdx = (self.h[idx + 1] + self.paper_h[idx + 1])
                    - (self.h[idx - 1] + self.paper_h[idx - 1]);
                let dhdy = (self.h[idx + w] + self.paper_h[idx + w])
                    - (self.h[idx - w] + self.paper_h[idx - w]);
                self.u[idx] += -1.5 * dhdx * self.dt;
                self.v[idx] += -1.5 * dhdy * self.dt;
                self.u[idx] *= friction;
                self.v[idx] *= friction;
            }
        }
    }

    fn relax_divergence(&mut self) {
        let w = self.width;
        for _ in 0..self.iterations {
            for i in 1..(self.height - 1) {
                for j in 1..(w - 1) {
                    let idx = i * w + j;
                    let div = (self.u[idx + 1] - self.u[idx - 1] + self.v[idx + w]
                        - self.v[idx - w])
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
        let w = self.width;
        let h = self.height;
        let mut next_h = vec![0.0f32; self.total];
        let mut next_r = vec![0.0f32; self.total];
        let mut next_g = vec![0.0f32; self.total];
        let mut next_b = vec![0.0f32; self.total];

        for i in 1..(h - 1) {
            for j in 1..(w - 1) {
                let idx = i * w + j;
                if self.h[idx] <= 0.0 {
                    continue;
                }

                let pi = (i as f32 - self.u[idx] * self.dt)
                    .max(1.0)
                    .min((h - 2) as f32);
                let pj = (j as f32 - self.v[idx] * self.dt)
                    .max(1.0)
                    .min((w - 2) as f32);

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

                let idx00 = i0 * w + j0;
                let idx10 = i1 * w + j0;
                let idx01 = i0 * w + j1;
                let idx11 = i1 * w + j1;

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

    fn deposition(&mut self) {
        for i in 0..self.total {
            if self.h[i] < 0.01 {
                continue;
            }
            let speed = (self.u[i] * self.u[i] + self.v[i] * self.v[i]).sqrt();
            let paper_val = self.paper_h[i];
            let dep_rate = self.adhesion
                * (1.0 / (speed + 0.5))
                * (1.0 + self.granularity * (1.0 - paper_val) * 1.5);
            let rate = (dep_rate * self.dt).min(0.5);
            let (ar, ag, ab) = (self.gr[i] * rate, self.gg[i] * rate, self.gb[i] * rate);
            self.dr[i] += ar;
            self.dg[i] += ag;
            self.db[i] += ab;
            self.gr[i] -= ar;
            self.gg[i] -= ag;
            self.gb[i] -= ab;
        }
    }

    fn capillary_flow(&mut self) {
        let w = self.width;
        for i in 1..(self.height - 1) {
            for j in 1..(w - 1) {
                let idx = i * w + j;
                if self.h[idx] < 0.02 {
                    continue;
                }
                let neighbors = [idx - w, idx + w, idx - 1, idx + 1];
                for &ni in &neighbors {
                    if self.h[ni] < self.h[idx] {
                        let diff = (self.h[idx] - self.h[ni]) * 0.02;
                        let cap_rate = diff * (1.0 - self.paper_h[ni] * 0.5);
                        if cap_rate > 0.0001 {
                            self.h[ni] += cap_rate;
                            self.h[idx] -= cap_rate;
                            let ratio = (cap_rate * 0.3) / (self.h[idx] + 0.001);
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

    fn back_run(&mut self) {
        let w = self.width;
        for i in 1..(self.height - 1) {
            for j in 1..(w - 1) {
                let idx = i * w + j;
                if self.h[idx] < 0.005 || self.h[idx] > 0.05 {
                    continue;
                }
                let neighbors = [idx - w, idx + w, idx - 1, idx + 1];
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
