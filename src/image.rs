use anyhow::{anyhow, Result};
use image::{ImageBuffer, ImageReader, Rgba};
use ratatui::{
    layout::Rect,
    style::Color,
    Frame,
};
use std::path::Path;

/// 加载图片为 RGBA8 格式
pub fn load_image(path: &Path) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let img = ImageReader::open(path)
        .map_err(|e| anyhow!("无法打开图片 {}: {}", path.display(), e))?
        .with_guessed_format()
        .map_err(|e| anyhow!("无法识别图片格式: {}", e))?
        .decode()
        .map_err(|e| anyhow!("解码图片失败: {}", e))?;
    Ok(img.to_rgba8())
}


/// 在指定区域绘制图片（使用半块字符，居中缩放）
pub fn draw_portrait(
    frame: &mut Frame,
    area: Rect,
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) {
    let (img_w, img_h) = img.dimensions();
    let area_w = area.width as usize;
    let area_h = area.height as usize;

    let target_px_w = area_w;
    let target_px_h = area_h * 2;

    let scale_w = target_px_w as f64 / img_w as f64;
    let scale_h = target_px_h as f64 / img_h as f64;
    let scale = scale_w.min(scale_h);
    if scale <= 0.0 {
        return;
    }

    let new_w = (img_w as f64 * scale) as u32;
    let new_h = (img_h as f64 * scale) as u32;
    if new_w == 0 || new_h == 0 {
        return;
    }

    let resized = image::imageops::resize(
        img,
        new_w,
        new_h,
        image::imageops::FilterType::Triangle,
    );

    let char_h = (new_h + 1) / 2;
    let offset_y = if char_h as usize > area_h {
        0
    } else {
        (area_h - char_h as usize) / 2
    };
    let offset_x = (area_w as i32 - new_w as i32) / 2;

    let buffer = frame.buffer_mut();
    for row in 0..area_h {
        let row_in_img = row as i32 - offset_y as i32;
        if row_in_img < 0 {
            continue;
        }
        let y_pixel_top = (row_in_img as usize) * 2;
        if y_pixel_top >= new_h as usize {
            continue;
        }
        let y_pixel_bottom = y_pixel_top + 1;
        let screen_row = (area.y + row as u16) as usize;

        for col in 0..area_w {
            let x_pixel = col as i32 - offset_x;
            if x_pixel < 0 || x_pixel as usize >= new_w as usize {
                continue;
            }
            let x_pixel = x_pixel as usize;
            let pixel_top = resized.get_pixel(x_pixel as u32, y_pixel_top as u32);
            let top_color = Color::Rgb(pixel_top[0], pixel_top[1], pixel_top[2]);

            let bottom_color = if y_pixel_bottom < new_h as usize {
                let pixel_bottom = resized.get_pixel(x_pixel as u32, y_pixel_bottom as u32);
                Color::Rgb(pixel_bottom[0], pixel_bottom[1], pixel_bottom[2])
            } else {
                Color::Black
            };

            let cell = buffer.get_mut((area.x + col as u16) as u16, screen_row as u16);
            cell.set_char('▀')
                .set_fg(top_color)
                .set_bg(bottom_color);
        }
    }
}