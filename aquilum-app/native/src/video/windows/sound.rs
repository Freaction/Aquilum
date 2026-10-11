use windows::Win32::Media::Audio::{
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, IAudioClient, IAudioRenderClient,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, WAVEFORMATEX, eConsole, eRender,
};
use windows::Win32::Media::MediaFoundation::{
    IMFSourceReader, MF_MT_AUDIO_AVG_BYTES_PER_SECOND, MF_MT_AUDIO_BITS_PER_SAMPLE, MF_MT_AUDIO_BLOCK_ALIGNMENT, MF_MT_AUDIO_NUM_CHANNELS,
    MF_MT_AUDIO_SAMPLES_PER_SECOND, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE, MF_SOURCE_READER_ALL_STREAMS, MF_SOURCE_READER_FIRST_AUDIO_STREAM,
    MF_SOURCE_READERF_ENDOFSTREAM, MFAudioFormat_PCM, MFCreateMediaType, MFCreateSourceReaderFromURL, MFMediaType_Audio,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::core::HSTRING;

const STREAM: u32 = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;
const BUFFER: i64 = 2_000_000;

pub struct Sound {
    reader: IMFSourceReader,
    client: IAudioClient,
    render: IAudioRenderClient,
    volume: ISimpleAudioVolume,
    rate: u32,
    block: usize,
    capacity: u32,
    pending: Vec<u8>,
    written: u64,
    skip: Option<f64>,
    ended: bool,
    running: bool,
}

impl Sound {
    pub fn open(source: &HSTRING) -> Option<Sound> {
        unsafe {
            let reader = MFCreateSourceReaderFromURL(source, None).ok()?;
            reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false).ok()?;
            reader.SetStreamSelection(STREAM, true).ok()?;
            let native = reader.GetNativeMediaType(STREAM, 0).ok()?;
            let rate = native.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND).ok()?;
            let channels = native.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).ok()?.clamp(1, 8);
            let block = channels * 2;
            let kind = MFCreateMediaType().ok()?;
            kind.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).ok()?;
            kind.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM).ok()?;
            kind.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).ok()?;
            kind.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, rate).ok()?;
            kind.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, channels).ok()?;
            kind.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, block).ok()?;
            kind.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, rate * block).ok()?;
            reader.SetCurrentMediaType(STREAM, None, &kind).ok()?;

            let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = devices.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).ok()?;
            let format = WAVEFORMATEX {
                wFormatTag: 1,
                nChannels: channels as u16,
                nSamplesPerSec: rate,
                nAvgBytesPerSec: rate * block,
                nBlockAlign: block as u16,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            let flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
            client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, BUFFER, 0, &format, None).ok()?;
            let capacity = client.GetBufferSize().ok()?;
            let render: IAudioRenderClient = client.GetService().ok()?;
            let volume: ISimpleAudioVolume = client.GetService().ok()?;
            Some(Sound { reader, client, render, volume, rate, block: block as usize, capacity, pending: Vec::new(), written: 0, skip: None, ended: false, running: false })
        }
    }

    fn padding(&self) -> u32 {
        unsafe { self.client.GetCurrentPadding().unwrap_or(0) }
    }

    fn read(&mut self) {
        unsafe {
            let (mut flags, mut time) = (0u32, 0i64);
            let mut sample = None;
            if self.reader.ReadSample(STREAM, 0, None, Some(&mut flags), Some(&mut time), Some(&mut sample)).is_err() || flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                self.ended = true;
                return;
            }
            let Some(buffer) = sample.and_then(|s| s.ConvertToContiguousBuffer().ok()) else { return };
            let (mut data, mut length) = (std::ptr::null_mut(), 0u32);
            if buffer.Lock(&mut data, None, Some(&mut length)).is_err() {
                return;
            }
            let bytes = std::slice::from_raw_parts(data, length as usize);
            let from = match self.skip.take() {
                Some(target) if (time as f64 / 1e7) < target => {
                    let frames = ((target - time as f64 / 1e7) * f64::from(self.rate)) as usize;
                    (frames * self.block).min(bytes.len())
                }
                _ => 0,
            };
            self.pending.extend_from_slice(&bytes[from..]);
            let _ = buffer.Unlock();
        }
    }

    pub fn fill(&mut self) {
        let free = self.capacity.saturating_sub(self.padding()) as usize;
        while self.pending.len() < free * self.block && !self.ended {
            self.read();
        }
        let frames = (self.pending.len() / self.block).min(free);
        if frames == 0 {
            return;
        }
        unsafe {
            let Ok(target) = self.render.GetBuffer(frames as u32) else { return };
            std::ptr::copy_nonoverlapping(self.pending.as_ptr(), target, frames * self.block);
            let _ = self.render.ReleaseBuffer(frames as u32, 0);
        }
        self.pending.drain(..frames * self.block);
        self.written += frames as u64;
        if !self.running {
            self.running = unsafe { self.client.Start().is_ok() };
        }
    }

    pub fn played(&self) -> f64 {
        self.written.saturating_sub(u64::from(self.padding())) as f64 / f64::from(self.rate)
    }

    pub fn finished(&self) -> bool {
        self.ended && self.pending.is_empty() && self.padding() == 0
    }

    pub fn pause(&mut self) {
        if self.running {
            unsafe {
                let _ = self.client.Stop();
            }
            self.running = false;
        }
    }

    pub fn seek(&mut self, seconds: f64) {
        self.pause();
        unsafe {
            let _ = self.client.Reset();
        }
        self.pending.clear();
        self.written = 0;
        self.ended = false;
        self.skip = Some(seconds);
        super::seek(&self.reader, seconds);
    }

    pub fn mute(&self, muted: bool) {
        unsafe {
            let _ = self.volume.SetMute(muted, std::ptr::null());
        }
    }
}

impl Drop for Sound {
    fn drop(&mut self) {
        self.pause();
    }
}
