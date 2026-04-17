use anyhow::{anyhow, Result};
use image::{ImageBuffer, ImageReader, Rgba};
use ratatui::{
    layout::Rect,
    style::Color,
    Frame,
};
use std::path::Path;

// 重新导出给其他模块使用
// pub use image::{ImageBuffer, Rgba};

pub fn load_image(path: &Path) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let img = ImageReader::open(path)
        .map_err(|e| anyhow!("无法打开图片 {}: {}", path.display(), e))?
        .with_guessed_format()
        .map_err(|e| anyhow!("无法识别图片格式: {}", e))?
        .decode()
        .map_err(|e| anyhow!("解码图片失败: {}", e))?;
    Ok(img.to_rgba8())
}


/// 绘制背景图片（拉伸填满整个区域，不保持宽高比）
pub fn draw_background(
    frame: &mut Frame,
    area: Rect,
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) {
    let area_w = area.width as usize;
    let area_h = area.height as usize;
    let target_px_w = area_w;
    let target_px_h = area_h * 2;

    // 直接拉伸到目标尺寸
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


pub fn draw_portrait(
    frame: &mut Frame,
    area: Rect,
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    position: usize,
    _scale_percent: u8,
) {
    let (img_w, img_h) = img.dimensions();
    let area_w = area.width as usize;
    let area_h = area.height as usize;

    let target_px_h = area_h * 2;
    // 缩放图片使得高度等于 target_px_h（保持宽高比）
    let scale = target_px_h as f64 / img_h as f64;
    let target_w = (img_w as f64 * scale) as u32;
    let target_h = target_px_h as u32;
    if target_w == 0 || target_h == 0 {
        return;
    }

    // 缩放图片（使用 Triangle 算法，质量较好）
    let resized = image::imageops::resize(
        img,
        target_w,
        target_h,
        image::imageops::FilterType::Triangle,
    );

    // 水平偏移（像素）
    let offset_x = match position {
        1 => 0,  // 左对齐
        3 => (area_w as i32 - target_w as i32).max(0),  // 右对齐
        _ => (area_w as i32 - target_w as i32) / 2,    // 居中
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
            let top_alpha = pixel_top[3];
            if top_alpha < 128 {
                continue; // 透明像素跳过
            }
            let top_color = Color::Rgb(pixel_top[0], pixel_top[1], pixel_top[2]);

            let bottom_color = if y_bottom < target_h as usize {
                let pixel_bottom = resized.get_pixel(x_px as u32, y_bottom as u32);
                let bottom_alpha = pixel_bottom[3];
                if bottom_alpha < 128 {
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