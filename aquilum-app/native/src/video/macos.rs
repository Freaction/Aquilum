use std::path::Path;
use std::sync::Arc;

use masonry::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AllocAnyThread, MainThreadMarker};
use objc2_av_foundation::{AVPlayer, AVPlayerItem, AVPlayerItemVideoOutput};
use objc2_core_foundation::CFString;
use objc2_core_media::CMTime;
use objc2_core_video::{
    CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferGetHeight, CVPixelBufferGetWidth, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress, kCVPixelBufferPixelFormatTypeKey, kCVPixelFormatType_32BGRA,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

const TIMESCALE: i32 = 600;
const END: f64 = 0.05;

pub struct Engine {
    player: Retained<AVPlayer>,
    item: Retained<AVPlayerItem>,
    output: Retained<AVPlayerItemVideoOutput>,
    play: bool,
}

fn seconds(time: CMTime) -> f64 {
    let value = unsafe { time.seconds() };
    if value.is_finite() { value } else { 0.0 }
}

impl Engine {
    pub fn open(path: &Path) -> Option<Engine> {
        let mtm = MainThreadMarker::new()?;
        let absolute = std::path::absolute(path).ok()?;
        unsafe {
            let url = NSURL::fileURLWithPath(&NSString::from_str(&absolute.to_string_lossy()));
            let item = AVPlayerItem::playerItemWithURL(&url, mtm);
            let key: &NSString = &*(kCVPixelBufferPixelFormatTypeKey as *const CFString).cast::<NSString>();
            let format = NSNumber::new_u32(kCVPixelFormatType_32BGRA);
            let value: &AnyObject = format.as_ref();
            let attributes = NSDictionary::from_slices(&[key], &[value]);
            let output = AVPlayerItemVideoOutput::initWithPixelBufferAttributes(AVPlayerItemVideoOutput::alloc(), Some(&attributes));
            item.addOutput(&output);
            let player = AVPlayer::playerWithPlayerItem(Some(&item), mtm);
            Some(Engine { player, item, output, play: false })
        }
    }

    fn ended(&self) -> bool {
        let duration = unsafe { seconds(self.item.duration()) };
        duration > 0.0 && unsafe { seconds(self.player.currentTime()) } >= duration - END
    }

    pub fn toggle(&mut self) {
        if self.ended() {
            unsafe { self.player.seekToTime(CMTime::with_seconds(0.0, TIMESCALE)) };
            self.play = false;
        }
        self.play = !self.play;
        unsafe {
            if self.play { self.player.play() } else { self.player.pause() }
        }
    }

    pub fn seek(&mut self, fraction: f64) {
        let duration = unsafe { seconds(self.item.duration()) };
        if duration > 0.0 {
            unsafe { self.player.seekToTime(CMTime::with_seconds(duration * fraction.clamp(0.0, 1.0), TIMESCALE)) };
        }
    }

    pub fn mute(&mut self) {
        unsafe { self.player.setMuted(!self.player.isMuted()) };
    }

    pub fn status(&mut self) -> (bool, f64, f64, bool) {
        if self.play && self.ended() {
            self.play = false;
        }
        unsafe { (self.play, seconds(self.player.currentTime()), seconds(self.item.duration()), self.player.isMuted()) }
    }

    pub fn frame(&mut self) -> Option<ImageBrush> {
        unsafe {
            let time = self.item.currentTime();
            if !self.output.hasNewPixelBufferForItemTime(time) {
                return None;
            }
            let buffer = self.output.copyPixelBufferForItemTime_itemTimeForDisplay(time, std::ptr::null_mut())?;
            if CVPixelBufferLockBaseAddress(&buffer, CVPixelBufferLockFlags::ReadOnly) != 0 {
                return None;
            }
            let (w, h, stride) = (CVPixelBufferGetWidth(&buffer), CVPixelBufferGetHeight(&buffer), CVPixelBufferGetBytesPerRow(&buffer));
            let base = CVPixelBufferGetBaseAddress(&buffer).cast::<u8>();
            let mut pixels = Vec::with_capacity(w * h * 4);
            if !base.is_null() {
                for row in 0..h {
                    pixels.extend_from_slice(std::slice::from_raw_parts(base.add(row * stride), w * 4));
                }
            }
            CVPixelBufferUnlockBaseAddress(&buffer, CVPixelBufferLockFlags::ReadOnly);
            if pixels.is_empty() || w == 0 || h == 0 {
                return None;
            }
            let data = ImageData { data: Blob::new(Arc::new(pixels)), format: ImageFormat::Bgra8, alpha_type: ImageAlphaType::Alpha, width: w as u32, height: h as u32 };
            Some(ImageBrush::new(data))
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe { self.player.pause() };
    }
}

#[allow(deprecated)]
pub fn poster(path: &Path) -> Option<ImageBrush> {
    use objc2_av_foundation::{AVAsset, AVAssetImageGenerator};
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_core_graphics::{CGBitmapContextCreate, CGBitmapInfo, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo};
    const SIZE: f64 = 1024.0;
    let absolute = std::path::absolute(path).ok()?;
    unsafe {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&absolute.to_string_lossy()));
        let generator = AVAssetImageGenerator::assetImageGeneratorWithAsset(&AVAsset::assetWithURL(&url));
        generator.setAppliesPreferredTrackTransform(true);
        generator.setMaximumSize(CGSize::new(SIZE, SIZE));
        let image = generator.copyCGImageAtTime_actualTime_error(CMTime::with_seconds(0.0, TIMESCALE), std::ptr::null_mut()).ok()?;
        let (w, h) = (CGImage::width(Some(&image)), CGImage::height(Some(&image)));
        if w == 0 || h == 0 {
            return None;
        }
        let mut pixels = vec![0u8; w * h * 4];
        let space = CGColorSpace::new_device_rgb()?;
        let info = CGImageAlphaInfo::PremultipliedLast.0 | CGBitmapInfo::ByteOrder32Big.0;
        let context = CGBitmapContextCreate(pixels.as_mut_ptr().cast(), w, h, 8, w * 4, Some(&space), info)?;
        CGContext::draw_image(Some(&context), CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(w as f64, h as f64)), Some(&image));
        drop(context);
        let data = ImageData { data: Blob::new(Arc::new(pixels)), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::AlphaPremultiplied, width: w as u32, height: h as u32 };
        Some(ImageBrush::new(data))
    }
}
