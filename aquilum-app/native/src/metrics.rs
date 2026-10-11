//! Замеры кадров. Пишутся в JSON-файл `--metrics`, который читает `scripts/measure.ps1`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Metrics {
    path: Option<PathBuf>,
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
    #[cfg(not(feature = "installed"))]
    live: Option<Live>,
}

#[cfg(not(feature = "installed"))]
#[derive(Default)]
struct Live {
    since: Option<Instant>,
    frames: u32,
    sums: [Duration; 4],
    worst: Duration,
    summary: String,
}

#[cfg(not(feature = "installed"))]
impl Live {
    fn frame(&mut self, parts: [Duration; 4]) -> bool {
        let since = *self.since.get_or_insert_with(Instant::now);
        self.frames += 1;
        for (sum, part) in self.sums.iter_mut().zip(parts) {
            *sum += part;
        }
        self.worst = self.worst.max(parts[3]);
        let elapsed = since.elapsed();
        if elapsed < Duration::from_secs(1) {
            return false;
        }
        let avg = |d: Duration| ms(d) / f64::from(self.frames);
        self.summary = format!(
            "{:.0} к/с · кадр {:.1} мс, худший {:.1} (сцена {:.1}, растр {:.1}, вывод {:.1})",
            f64::from(self.frames) / elapsed.as_secs_f64(),
            avg(self.sums[3]),
            ms(self.worst),
            avg(self.sums[0]),
            avg(self.sums[1]),
            avg(self.sums[2]),
        );
        *self = Live { summary: std::mem::take(&mut self.summary), ..Live::default() };
        true
    }
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
    pub fn new(threads: u16, path: Option<PathBuf>) -> Self {
        Metrics { path, threads, ..Default::default() }
    }

    pub fn window_created(&mut self, started: Instant, scale_factor: f64) {
        self.window_created_ms = ms(started.elapsed());
        self.scale_factor = scale_factor;
    }

    pub fn frame(&mut self, started: Instant, parts: [Duration; 4]) -> bool {
        let [_, raster, _, total] = parts;
        self.frames += 1;
        if self.frames == 1 {
            self.first_frame_ms = ms(started.elapsed());
        } else if self.path.is_some() {
            self.raster.push(ms(raster));
            self.total.push(ms(total));
        }
        #[cfg(not(feature = "installed"))]
        if let Some(live) = &mut self.live {
            return live.frame(parts);
        }
        false
    }

    #[cfg(not(feature = "installed"))]
    pub fn toggle_live(&mut self) {
        self.live = if self.live.is_some() { None } else { Some(Live::default()) };
    }

    pub fn summary(&self) -> Option<&str> {
        #[cfg(not(feature = "installed"))]
        if let Some(live) = &self.live {
            return Some(&live.summary);
        }
        None
    }

    pub fn write(&self) {
        let Some(path) = &self.path else { return };
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
        if let Err(e) = std::fs::write(path, &json) {
            eprintln!("не удалось записать замеры в {}: {e}", path.display());
        }
    }
}
