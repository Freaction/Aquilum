use std::path::Path;
use std::sync::{Arc, OnceLock};

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use gstreamer_video::{VideoCapsBuilder, VideoFormat, VideoInfo};
use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};

const PREROLL: gst::ClockTime = gst::ClockTime::from_seconds(5);

pub struct Engine {
    playbin: gst::Element,
    sink: AppSink,
    play: bool,
    ended: bool,
    last: Option<gst::ClockTime>,
}

fn ready() -> bool {
    static INIT: OnceLock<bool> = OnceLock::new();
    *INIT.get_or_init(|| gst::init().is_ok())
}

impl Engine {
    pub fn open(path: &Path) -> Option<Engine> {
        if !ready() {
            return None;
        }
        let uri = gst::glib::filename_to_uri(std::path::absolute(path).ok()?, None).ok()?;
        let caps = VideoCapsBuilder::new().format(VideoFormat::Rgba).build();
        let sink = AppSink::builder().caps(&caps).max_buffers(1).drop(true).build();
        let playbin = gst::ElementFactory::make("playbin").property("uri", uri.as_str()).property("video-sink", &sink).build().ok()?;
        playbin.set_state(gst::State::Paused).ok()?;
        Some(Engine { playbin, sink, play: false, ended: false, last: None })
    }

    fn poll(&mut self) {
        let Some(bus) = self.playbin.bus() else { return };
        while let Some(message) = bus.pop_filtered(&[gst::MessageType::Eos, gst::MessageType::Error]) {
            if let gst::MessageView::Error(error) = message.view() {
                eprintln!("видео: {}", error.error());
            }
            self.ended = true;
            self.play = false;
        }
    }

    fn duration(&self) -> f64 {
        self.playbin.query_duration::<gst::ClockTime>().map_or(0.0, |d| d.seconds_f64())
    }

    fn seek_to(&mut self, seconds: f64) {
        let at = gst::ClockTime::from_nseconds((seconds.max(0.0) * 1e9) as u64);
        let _ = self.playbin.seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, at);
        self.last = None;
    }

    pub fn toggle(&mut self) {
        self.poll();
        if self.ended {
            self.seek_to(0.0);
            self.ended = false;
            self.play = false;
        }
        self.play = !self.play;
        let _ = self.playbin.set_state(if self.play { gst::State::Playing } else { gst::State::Paused });
    }

    pub fn seek(&mut self, fraction: f64) {
        let duration = self.duration();
        if duration > 0.0 {
            self.ended = false;
            self.seek_to(duration * fraction.clamp(0.0, 1.0));
        }
    }

    pub fn mute(&mut self) {
        let muted = self.playbin.property::<bool>("mute");
        self.playbin.set_property("mute", !muted);
    }

    pub fn status(&mut self) -> (bool, f64, f64, bool) {
        self.poll();
        let position = self.playbin.query_position::<gst::ClockTime>().map_or(0.0, |p| p.seconds_f64());
        (self.play, position, self.duration(), self.playbin.property::<bool>("mute"))
    }

    pub fn frame(&mut self) -> Option<ImageBrush> {
        let sample = self.sink.try_pull_sample(gst::ClockTime::ZERO).or_else(|| self.sink.try_pull_preroll(gst::ClockTime::ZERO))?;
        let buffer = sample.buffer()?;
        if buffer.pts().is_some() && buffer.pts() == self.last {
            return None;
        }
        self.last = buffer.pts();
        let info = VideoInfo::from_caps(sample.caps()?).ok()?;
        let (w, h, stride) = (info.width() as usize, info.height() as usize, info.stride()[0] as usize);
        let map = buffer.map_readable().ok()?;
        let mut pixels = Vec::with_capacity(w * h * 4);
        for row in map.chunks(stride).take(h) {
            pixels.extend_from_slice(row.get(..w * 4)?);
        }
        let data = ImageData { data: Blob::new(Arc::new(pixels)), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::Alpha, width: w as u32, height: h as u32 };
        Some(ImageBrush::new(data))
    }

    pub fn poster(path: &Path) -> Option<ImageBrush> {
        let mut engine = Engine::open(path)?;
        let (result, _, _) = engine.playbin.state(PREROLL);
        result.ok()?;
        engine.frame()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.playbin.set_state(gst::State::Null);
    }
}
