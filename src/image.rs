// src/image.rs
use anyhow::{anyhow, Result};
use image::{ImageBuffer, ImageReader, Rgba};
use ratatui::{
    layout::Rect,
    style::Color,
    Frame,
};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;

// ============ LRU 图片缓存 ============
const MAX_CACHE: usize = 10;

struct ImageCache {
    queue: VecDeque<(String, RgbaImage)>,
}

static CACHE: Mutex<Option<ImageCache>> = Mutex::new(None);

/// 从资源系统加载图片（带 LRU 缓存）
pub fn load_image_rgba(path: &str) -> Result<RgbaImage> {
    let mut guard = CACHE.lock().unwrap();
    let cache = guard.get_or_insert_with(|| ImageCache {
        queue: VecDeque::new(),
    });

    if let Some(pos) = cache.queue.iter().position(|(k, _)| k == path) {
        let entry = cache.queue.remove(pos).unwrap();
        let img = entry.1.clone();
        cache.queue.push_back(entry);
        return Ok(img);
    }

    let img = decode_image(path)?;

    if cache.queue.len() >= MAX_CACHE {
        cache.queue.pop_front();
    }
    cache.queue.push_back((path.to_string(), img.clone()));

    Ok(img)
}

/// 直接从资源解码（不缓存，用于 logo 等一次性图片）
pub fn load_image_once(path: &str) -> Result<RgbaImage> {
    decode_image(path)
}

fn decode_image(path: &str) -> Result<RgbaImage> {
    let real_path = crate::assets::materialize(path)
        .ok_or_else(|| anyhow!("图片不存在: {}", path))?;
    let img = ImageReader::open(&real_path)
        .map_err(|e| anyhow!("打开图片失败: {}", e))?
        .with_guessed_format()
        .map_err(|e| anyhow!("识别格式失败: {}", e))?
        .decode()
        .map_err(|e| anyhow!("解码失败: {}", e))?;
    Ok(img.to_rgba8())
}

// ============ GIF 动画 ============
/// 单张 GIF 最多解析的帧数
pub const GIF_MAX_FRAMES: usize = 300;
/// 单张 GIF 展开后的内存上限（防止爆内存）
pub const GIF_MAX_TOTAL_BYTES: u64 = 32 * 1024 * 1024; // 32 MB

pub struct GifPlayer {
    frames: Vec<RgbaImage>,
    delays: Vec<Duration>,
    current: usize,
    next_frame_time: Instant,
    pub total_bytes: u64,
}

impl GifPlayer {
    pub fn from_frames(frames: Vec<RgbaImage>, delays: Vec<Duration>) -> Self {
        let total_bytes: u64 = frames
            .iter()
            .map(|f| (f.width() * f.height() * 4) as u64)
            .sum();
        let first_delay = delays
            .first()
            .copied()
            .unwrap_or(Duration::from_millis(100));
        Self {
            frames,
            delays,
            current: 0,
            next_frame_time: Instant::now() + first_delay,
            total_bytes,
        }
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn current_index(&self) -> usize {
        self.current
    }

    /// 时钟前进，如果该切帧则切，返回 true
    pub fn tick(&mut self) -> bool {
        if self.frames.is_empty() {
            return false;
        }
        let now = Instant::now();
        if now >= self.next_frame_time {
            self.current = (self.current + 1) % self.frames.len();
            let delay = self
                .delays
                .get(self.current)
                .copied()
                .unwrap_or(Duration::from_millis(100));
            // 防止延迟过长（某些 GIF 最后一帧 delay=0，会导致跳帧）
            let delay = delay.max(Duration::from_millis(20));
            self.next_frame_time = now + delay;
            true
        } else {
            false
        }
    }

    pub fn current_frame(&self) -> Option<&RgbaImage> {
        self.frames.get(self.current)
    }
}

/// 从资源加载 GIF（支持内存模式和文件系统）
pub fn load_gif(path: &str) -> Result<GifPlayer> {
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;

    let real_path = crate::assets::materialize(path)
        .ok_or_else(|| anyhow!("GIF 不存在: {}", path))?;

    let file = std::fs::File::open(&real_path)
        .map_err(|e| anyhow!("打开 GIF 失败: {}", e))?;

    let decoder = GifDecoder::new(std::io::BufReader::new(file))
        .map_err(|e| anyhow!("GIF 解码器初始化失败: {}", e))?;

    let raw_frames = decoder
        .into_frames()
        .collect_frames()
        .map_err(|e| anyhow!("GIF 帧收集失败: {}", e))?;

    if raw_frames.is_empty() {
        return Err(anyhow!("GIF 没有有效帧: {}", path));
    }

    let mut frames: Vec<RgbaImage> = Vec::new();
    let mut delays: Vec<Duration> = Vec::new();
    let mut total_bytes: u64 = 0;

    for (i, frame) in raw_frames.into_iter().enumerate() {
        if i >= GIF_MAX_FRAMES {
            break;
        }

        // 帧延迟
        let delay = frame.delay();
        let (num, den) = delay.numer_denom_ms();
        let ms = if den == 0 { 100 } else { num / den };
        let ms = ms.clamp(20, 2000); // 20ms–2s 之间

        let buffer = frame.into_buffer();
        total_bytes += (buffer.width() * buffer.height() * 4) as u64;
        if total_bytes > GIF_MAX_TOTAL_BYTES {
            break;
        }

        frames.push(buffer);
        delays.push(Duration::from_millis(ms as u64));
    }

    if frames.is_empty() {
        return Err(anyhow!("GIF 无可用帧: {}", path));
    }

    Ok(GifPlayer::from_frames(frames, delays))
}

// ============ 绘制函数 ============

/// 绘制背景图片（拉伸填满整个区域，不保持宽高比）
pub fn draw_background(frame: &mut Frame, area: Rect, img: &RgbaImage) {
    let area_w = area.width as usize;
    let area_h = area.height as usize;
    let target_px_w = area_w;
    let target_px_h = area_h * 2;

    let resized = image::imageops::resize(
        img,
        target_px_w as u32,
        target_px_h as u32,
        image::imageops::FilterType::Triangle,
    );

    let buffer = frame.buffer_mut();
    for row in 0..area_h {
        let y_px = row * 2;
        if y_px >= target_px_h {
            break;
        }
        let y_bottom = y_px + 1;
        let screen_row = (area.y + row as u16) as usize;
        for col in 0..area_w {
            let pixel_top = resized.get_pixel(col as u32, y_px as u32);
            let top_color = Color::Rgb(pixel_top[0], pixel_top[1], pixel_top[2]);
            let bottom_color = if y_bottom < target_px_h {
                let pixel_bottom = resized.get_pixel(col as u32, y_bottom as u32);
                Color::Rgb(pixel_bottom[0], pixel_bottom[1], pixel_bottom[2])
            } else {
                Color::Reset
            };
            let cell = buffer.get_mut((area.x + col as u16) as u16, screen_row as u16);
            cell.set_char('▀')
                .set_fg(top_color)
                .set_bg(bottom_color);
        }
    }
}

/// 绘制立绘（支持位置和缩放，高度自动填满区域）
pub fn draw_portrait(
    frame: &mut Frame,
    area: Rect,
    img: &RgbaImage,
    position: usize,
    _scale_percent: u8,
) {
    let (img_w, img_h) = img.dimensions();
    let area_w = area.width as usize;
    let area_h = area.height as usize;

    let target_px_h = area_h * 2;
    let scale = target_px_h as f64 / img_h as f64;
    let target_w = (img_w as f64 * scale) as u32;
    let target_h = target_px_h as u32;
    if target_w == 0 || target_h == 0 {
        return;
    }

    let resized = image::imageops::resize(
        img,
        target_w,
        target_h,
        image::imageops::FilterType::Triangle,
    );

    let offset_x = match position {
        1 => 0,
        3 => (area_w as i32 - target_w as i32).max(0),
        _ => (area_w as i32 - target_w as i32) / 2,
    };

    let buffer = frame.buffer_mut();
    for row in 0..area_h {
        let y_px = row * 2;
        if y_px >= target_h as usize {
            break;
        }
        let y_bottom = y_px + 1;
        let screen_row = (area.y + row as u16) as usize;

        for col in 0..area_w {
            let x_px = (col as i32 - offset_x) as usize;
            if x_px >= target_w as usize {
                continue;
            }
            let pixel_top = resized.get_pixel(x_px as u32, y_px as u32);
            if pixel_top[3] < 128 {
                continue;
            }
            let top_color = Color::Rgb(pixel_top[0], pixel_top[1], pixel_top[2]);

            let bottom_color = if y_bottom < target_h as usize {
                let pixel_bottom = resized.get_pixel(x_px as u32, y_bottom as u32);
                if pixel_bottom[3] < 128 {
                    Color::Reset
                } else {
                    Color::Rgb(pixel_bottom[0], pixel_bottom[1], pixel_bottom[2])
                }
            } else {
                Color::Reset
            };

            let cell = buffer.get_mut((area.x + col as u16) as u16, screen_row as u16);
            cell.set_char('▀')
                .set_fg(top_color)
                .set_bg(bottom_color);
        }
    }
}

/// 自适应绘制（居中，100%）
pub fn draw_portrait_adaptive(frame: &mut Frame, area: Rect, img: &RgbaImage) {
    draw_portrait(frame, area, img, 2, 100);
}