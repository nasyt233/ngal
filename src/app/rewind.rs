// src/app/rewind.rs
use super::{App, Snapshot};

impl App {
    /// 记录当前状态到回退栈
    pub(crate) fn push_snapshot(&mut self) {
        const MAX_SNAPSHOTS: usize = 100;
        let snap = Snapshot {
            state: self.state.clone(),
            current_file: self.current_file.clone(),
            variables: self.variables.serialize(),
            current_background: self.current_background.clone(),
            current_image_params: self.current_image_params.clone(),
            current_bgm: self.current_bgm.clone(),
            target_text: self.target_text.clone(),
        };
        if self.snapshot_stack.len() >= MAX_SNAPSHOTS {
            self.snapshot_stack.remove(0);
        }
        self.snapshot_stack.push(snap);
    }
    
    pub fn rewind_dialogue(&mut self) {
        let snap = match self.snapshot_stack.pop() {
            Some(s) => s,
            None => return,
        };
    
        self.state = snap.state;
        self.current_file = snap.current_file;
        self.variables.deserialize(snap.variables);
        self.current_image_params = snap.current_image_params;
        self.current_background = snap.current_background;
    
        // GIF 状态不恢复，重置为 None
        self.current_image_gif = None;
        self.current_background_gif = None;
    
        // BGM 变更时重新播放
        let snap_bgm = snap.current_bgm.clone();
        if self.current_bgm != snap_bgm {
            match snap_bgm {
                Some(f) => self.play_bgm(&f),
                None => self.stop_bgm(),
            }
        }
    
        self.stop_voice();
    
        self.target_text = snap.target_text;
        self.display_text = self.target_text.clone();
        self.last_char_time = std::time::Instant::now();
    }
}