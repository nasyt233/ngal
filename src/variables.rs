use std::collections::HashMap;
use regex::Regex;

pub struct Variables {
    map: HashMap<String, String>,
}

impl Variables {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.map.insert(name.to_string(), value.to_string());
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        self.map.get(name)
    }

    /// 将文本中的{}替换为实际值
    pub fn interpolate(&self, text: &str) -> String {
        let re = Regex::new(r"\{([^{}]+)\}").unwrap();
        let mut result = text.to_string();
        for cap in re.captures_iter(text) {
            let var_name = &cap[1];
            if let Some(value) = self.get(var_name) {
                result = result.replace(&format!("{{{}}}", var_name), value);
            }
        }
        result
    }

    /// 序列化用于存档
    pub fn serialize(&self) -> HashMap<String, String> {
        self.map.clone()
    }

    /// 从存档恢复
    pub fn deserialize(&mut self, data: HashMap<String, String>) {
        self.map = data;
    }
}

impl Default for Variables {
    fn default() -> Self {
        Self::new()
    }
}