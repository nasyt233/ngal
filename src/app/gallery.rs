// src/app/gallery.rs
use crate::image::{self, GifPlayer, RgbaImage};

pub struct GalleryEntry {
    pub filename: String,
    pub path: String,
    pub is_gif: bool,
    pub size_kb: u64,
}

pub struct GalleryState {
    pub images: Vec<GalleryEntry>,
    pub selected: usize,
    pub current_gif: Option<GifPlayer>,
    pub current_static: Option<RgbaImage>,
    pub error: Option<String>,
    pub list_area: ratatui::layout::Rect,
}

impl GalleryState {
    pub fn new() -> Self {
        let mut images = Vec::new();

        let filenames = crate::assets::list_dir_recursive("assets/portraits");
    
        for name in filenames {
            if name.starts_with('.') {
                continue;
            }
    
            let ext = std::path::Path::new(&name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
    
            let supported = matches!(
                ext.as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
            );
            if !supported {
                continue;
            }
    
            let is_gif = ext == "gif";
            let path = format!("assets/portraits/{}", name);
    
            // 大小：优先从内存读，回退文件系统
            let size_kb = crate::assets::read_bytes(&path)
                .map(|b| (b.len() / 1024) as u64)
                .unwrap_or(0);
    
            images.push(GalleryEntry {
                filename: name,
                path,
                is_gif,
                size_kb,
            });
        }
    
        images.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    
        let mut state = Self {
            images,
            selected: 0,
            current_gif: None,
            current_static: None,
            error: None,
            list_area: ratatui::layout::Rect::default(),
        };
        state.load_current();
        state
    }

    pub fn load_current(&mut self) {
        self.current_gif = None;
        self.current_static = None;
        self.error = None;

        let entry = match self.images.get(self.selected) {
            Some(e) => e,
            None => return,
        };

        if entry.is_gif {
            match image::load_gif(&entry.path) {
                Ok(gif) => self.current_gif = Some(gif),
                Err(e) => self.error = Some(format!("GIF 加载失败: {}", e)),
            }
        } else {
            match image::load_image_rgba(&entry.path) {
                Ok(img) => self.current_static = Some(img),
                Err(e) => self.error = Some(format!("图片加载失败: {}", e)),
            }
        }
    }

    pub fn next(&mut self) {
        if self.images.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.images.len();
        self.load_current();
    }

    pub fn prev(&mut self) {
        if self.images.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.images.len() - 1;
        } else {
            self.selected -= 1;
        }
        self.load_current();
    }

    pub fn page_down(&mut self, n: usize) {
        if self.images.is_empty() {
            return;
        }
        self.selected = (self.selected + n).min(self.images.len() - 1);
        self.load_current();
    }

    pub fn page_up(&mut self, n: usize) {
        if self.images.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(n);
        self.load_current();
    }

    pub fn first(&mut self) {
        if !self.images.is_empty() && self.selected != 0 {
            self.selected = 0;
            self.load_current();
        }
    }

    pub fn last(&mut self) {
        if !self.images.is_empty() && self.selected != self.images.len() - 1 {
            self.selected = self.images.len() - 1;
            self.load_current();
        }
    }

    pub fn tick_gif(&mut self) {
        if let Some(gif) = &mut self.current_gif {
            gif.tick();
        }
    }

    pub fn current_frame(&self) -> Option<&RgbaImage> {
        if let Some(gif) = &self.current_gif {
            gif.current_frame()
        } else {
            self.current_static.as_ref()
        }
    }

    /// 当前帧的尺寸信息（用于信息栏）
    pub fn current_dimensions(&self) -> Option<(u32, u32)> {
        self.current_frame().map(|f| (f.width(), f.height()))
    }

    /// 当前 GIF 帧数（非 GIF 返回 None）
    pub fn current_gif_frames(&self) -> Option<usize> {
        self.current_gif.as_ref().map(|g| g.frame_count())
    }

    /// 当前 GIF 帧号
    pub fn current_gif_index(&self) -> Option<usize> {
        self.current_gif.as_ref().map(|g| g.current_index())
    }
}