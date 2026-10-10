//! Замеры кадров. Пишутся в JSON-файл `--metrics`, который читает `scripts/measure.ps1`.

use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Metrics {
    threads: u16,
    scale_factor: f64,
    window_created_ms: f64,
    first_frame_ms: f64,
    pub frames: u64,
    /// Анимационные тики Masonry; кадр из них получается только при видимых изменениях.
    pub anim_ticks: u64,
    /// Кадры после первого: растеризация и полный кадр (с переносом в буфер и present), мс.
    raster: Vec<f64>,
    total: Vec<f64>,
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn avg(v: &[f64]) -> f64 {
    if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn p95(v: &[f64]) -> f64 {
    let mut sorted = v.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted.get(((sorted.len().max(1) - 1) as f64 * 0.95).round() as usize).copied().unwrap_or(0.0)
}

impl Metrics {
    pub fn new(threads: u16) -> Self {
        Metrics { threads, ..Default::default() }
    }

    pub fn window_created(&mut self, started: Instant, scale_factor: f64) {
        self.window_created_ms = ms(started.elapsed());
        self.scale_factor = scale_factor;
    }

    pub fn frame(&mut self, started: Instant, raster: Duration, total: Duration) {
        self.frames += 1;
        if self.frames == 1 {
            self.first_frame_ms = ms(started.elapsed());
        } else {
            self.raster.push(ms(raster));
            self.total.push(ms(total));
        }
    }

    pub fn write(&self, path: Option<&Path>) {
        let json = format!(
            concat!(
                "{{\"threads\":{},\"scaleFactor\":{},\"windowCreatedMs\":{:.1},\"firstFrameMs\":{:.1},",
                "\"frames\":{},\"animTicks\":{},\"rasterAvgMs\":{:.2},\"rasterP95Ms\":{:.2},",
                "\"frameAvgMs\":{:.2},\"frameP95Ms\":{:.2}}}"
            ),
            self.threads,
            self.scale_factor,
            self.window_created_ms,
            self.first_frame_ms,
            self.frames,
            self.anim_ticks,
            avg(&self.raster),
            p95(&self.raster),
            avg(&self.total),
            p95(&self.total),
        );
        match path {
            Some(path) => {
                if let Err(e) = std::fs::write(path, &json) {
                    eprintln!("не удалось записать замеры в {}: {e}", path.display());
                }
            }
            None => eprintln!("{json}"),
        }
    }
}
