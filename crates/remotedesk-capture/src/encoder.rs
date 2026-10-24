use crate::capture::CapturedFrame;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use image::{codecs::jpeg::JpegEncoder, ExtendedColorType, ImageEncoder};
use remotedesk_core::protocol::{FrameFormat, SessionPacket};
use std::io::Cursor;

/// Configuration for the streaming encoder.
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub jpeg_quality: u8,
    pub max_fps: u32,
    pub scale_percent: u8,
    pub enable_delta_skip: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            jpeg_quality: 50,
            max_fps: 30,
            scale_percent: 100,
            enable_delta_skip: true,
        }
    }
}

/// Statistics about the encoding process.
#[derive(Debug, Default, Clone, Copy)]
pub struct EncoderStats {
    pub frames_encoded: u64,
    pub frames_skipped: u64,
    pub last_encode_duration_ms: u64,
    pub last_frame_size_bytes: usize,
}

/// Frame encoder that converts captured RGBA frames into compressed network packets.
pub struct FrameEncoder {
    pub config: EncoderConfig,
    pub stats: EncoderStats,
    last_frame_hash: u64,
    last_encoded_at: std::time::Instant,
    force_keyframe: bool,
    rgb_buffer: Vec<u8>,
}

impl FrameEncoder {
    pub fn new(config: EncoderConfig) -> Self {
        Self {
            config,
            stats: EncoderStats::default(),
            last_frame_hash: 0,
            last_encoded_at: std::time::Instant::now(),
            force_keyframe: true,
            rgb_buffer: Vec::new(),
        }
    }

    /// Triggers an immediate keyframe on the next captured frame (bypassing delta-skip).
    pub fn force_next_keyframe(&mut self) {
        self.force_keyframe = true;
        self.last_frame_hash = 0;
    }

    /// Updates dynamic stream quality and scaling in real-time.
    pub fn set_quality_config(&mut self, jpeg_quality: u8, max_fps: u32, scale_percent: u8) {
        if jpeg_quality > 0 {
            self.config.jpeg_quality = jpeg_quality.min(100);
        }
        if max_fps > 0 {
            self.config.max_fps = max_fps.min(120);
        }
        if scale_percent > 0 {
            self.config.scale_percent = scale_percent.min(100);
        }
        self.force_next_keyframe();
    }

    /// Computes a fast sampled hash across the frame to detect screen changes.
    fn sample_hash(rgba: &[u8]) -> u64 {
        let step = (rgba.len() / 1024).max(1);
        let mut hash: u64 = 0xcbf29ce484222325;
        let mut i = 0;
        while i < rgba.len() {
            hash ^= rgba[i] as u64;
            hash = hash.wrapping_mul(0x100000001b3);
            i += step;
        }
        hash
    }

    /// Encodes a captured frame into a `SessionPacket::ScreenFrame`.
    /// Returns `None` if the frame is identical to the previous frame (delta skip).
    pub fn encode_to_packet(&mut self, frame: &CapturedFrame) -> Result<Option<SessionPacket>, String> {
        let start = std::time::Instant::now();

        // Send a frame if:
        // 1. Client explicitly requested keyframe (e.g. restore from minimize / focus)
        // 2. Initial session frame
        // 3. Heartbeat idle timeout (>= 1000ms since last frame) to keep client active and synchronized
        // 4. Periodic keyframe interval (every 30 encoded frames)
        let idle_timeout = self.last_encoded_at.elapsed() >= std::time::Duration::from_millis(1000);
        let is_keyframe = self.force_keyframe
            || self.stats.frames_encoded == 0
            || idle_timeout
            || (self.stats.frames_encoded % 30 == 0);

        if self.force_keyframe {
            self.force_keyframe = false;
        }

        if self.config.enable_delta_skip && !is_keyframe {
            let current_hash = Self::sample_hash(&frame.rgba);
            if current_hash == self.last_frame_hash && self.stats.frames_encoded > 0 {
                self.stats.frames_skipped += 1;
                return Ok(None);
            }
            self.last_frame_hash = current_hash;
        } else {
            self.last_frame_hash = Self::sample_hash(&frame.rgba);
        }

        // Calculate target dimensions based on scale_percent
        let (target_w, target_h) = if self.config.scale_percent > 0 && self.config.scale_percent < 100 {
            let w = ((frame.width as u64 * self.config.scale_percent as u64) / 100) as u32;
            let h = ((frame.height as u64 * self.config.scale_percent as u64) / 100) as u32;
            ((w & !1).max(320), (h & !1).max(240))
        } else {
            (frame.width, frame.height)
        };

        let pixel_count = (target_w * target_h) as usize;
        let target_len = pixel_count * 3;
        if self.rgb_buffer.len() != target_len {
            self.rgb_buffer.resize(target_len, 0);
        }

        if target_w == frame.width && target_h == frame.height {
            // 1:1 RGBA -> RGB pre-allocated buffer copy
            let src = &frame.rgba;
            let dst = &mut self.rgb_buffer;
            for i in 0..pixel_count {
                let s = i * 4;
                let d = i * 3;
                dst[d] = src[s];
                dst[d + 1] = src[s + 1];
                dst[d + 2] = src[s + 2];
            }
        } else {
            // Ultra-fast integer fixed-point downsampler directly from RGBA -> RGB in one pass
            let x_ratio = ((frame.width as u64) << 16) / target_w as u64;
            let y_ratio = ((frame.height as u64) << 16) / target_h as u64;
            let src = &frame.rgba;
            let dst = &mut self.rgb_buffer;

            for y in 0..target_h {
                let src_y = ((y as u64 * y_ratio) >> 16) as u32;
                let src_row = (src_y * frame.width * 4) as usize;
                let dst_row = (y * target_w * 3) as usize;
                for x in 0..target_w {
                    let src_x = ((x as u64 * x_ratio) >> 16) as u32;
                    let s = src_row + (src_x * 4) as usize;
                    let d = dst_row + (x * 3) as usize;
                    dst[d] = src[s];
                    dst[d + 1] = src[s + 1];
                    dst[d + 2] = src[s + 2];
                }
            }
        }

        let mut jpeg_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut jpeg_bytes);

        // Encode RGB -> JPEG
        let encoder = JpegEncoder::new_with_quality(&mut cursor, self.config.jpeg_quality);
        encoder
            .write_image(
                &self.rgb_buffer,
                target_w,
                target_h,
                ExtendedColorType::Rgb8,
            )
            .map_err(|e| format!("JPEG encoding failed: {}", e))?;

        let duration = start.elapsed().as_millis() as u64;
        self.stats.frames_encoded += 1;
        self.stats.last_encode_duration_ms = duration;
        self.stats.last_frame_size_bytes = jpeg_bytes.len();
        self.last_encoded_at = std::time::Instant::now();

        let base64_payload = BASE64_STANDARD.encode(&jpeg_bytes);

        Ok(Some(SessionPacket::ScreenFrame {
            frame_id: frame.frame_id,
            timestamp_ms: frame.timestamp_ms,
            monitor_idx: frame.monitor_idx,
            screen_w: frame.width,
            screen_h: frame.height,
            tile_x: 0,
            tile_y: 0,
            tile_w: target_w,
            tile_h: target_h,
            is_keyframe,
            format: FrameFormat::Jpeg,
            payload_base64: base64_payload,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_rgba_to_jpeg_packet() {
        let mut encoder = FrameEncoder::new(EncoderConfig::default());
        let frame = CapturedFrame {
            frame_id: 1,
            timestamp_ms: 1000,
            monitor_idx: 0,
            width: 64,
            height: 64,
            rgba: vec![120u8; 64 * 64 * 4],
            cursor_shape: "default".to_string(),
        };

        let packet = encoder.encode_to_packet(&frame).unwrap();
        assert!(packet.is_some());
        if let Some(SessionPacket::ScreenFrame { screen_w, screen_h, payload_base64, .. }) = packet {
            assert_eq!(screen_w, 64);
            assert_eq!(screen_h, 64);
            assert!(!payload_base64.is_empty());
        }

        // Second identical frame should be skipped when delta skip is enabled
        let skipped = encoder.encode_to_packet(&frame).unwrap();
        assert!(skipped.is_none());
        assert_eq!(encoder.stats.frames_skipped, 1);

        // Force keyframe must immediately bypass delta-skip and encode
        encoder.force_next_keyframe();
        let forced = encoder.encode_to_packet(&frame).unwrap();
        assert!(forced.is_some());
        if let Some(SessionPacket::ScreenFrame { is_keyframe, .. }) = forced {
            assert!(is_keyframe);
        }
    }

    #[test]
    fn test_downscaling_and_quality_config() {
        let mut config = EncoderConfig::default();
        config.scale_percent = 50;
        let mut encoder = FrameEncoder::new(config);
        let frame = CapturedFrame {
            frame_id: 2,
            timestamp_ms: 2000,
            monitor_idx: 0,
            width: 640,
            height: 480,
            rgba: vec![200u8; 640 * 480 * 4],
            cursor_shape: "default".to_string(),
        };

        let packet = encoder.encode_to_packet(&frame).unwrap().unwrap();
        if let SessionPacket::ScreenFrame { screen_w, screen_h, tile_w, tile_h, .. } = packet {
            assert_eq!(screen_w, 640);
            assert_eq!(screen_h, 480);
            assert_eq!(tile_w, 320);
            assert_eq!(tile_h, 240);
        } else {
            panic!("Expected ScreenFrame packet");
        }
    }
}
