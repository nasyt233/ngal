
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

    /// 插值文本中的 {变量} 占位符（用于对话文本）
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

    /// 获取变量值作为 f64，如果无法转换则返回 0.0
    pub fn get_number(&self, name: &str) -> f64 {
        self.map.get(name)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0)
    }

    /// 计算算术表达式，支持 + - * / 和括号
    /// 变量名可以直接写（不带花括号），也支持 {变量} 语法
    pub fn eval_expr(&self, expr: &str) -> Option<String> {
        let mut replaced = expr.to_string();
        
        for (key, _) in self.map.iter() {
            let val = self.get_number(key);
            let val_str = if val.fract() == 0.0 {
                format!("{:.0}", val)
            } else {
                format!("{}", val)
            };
            replaced = replaced.replace(&format!("{{{}}}", key), &val_str);
        }
        
        
        let mut keys: Vec<&String> = self.map.keys().collect();
        keys.sort_by_key(|k| -(k.len() as isize));
        for key in keys {
            let val = self.get_number(key);
            let val_str = if val.fract() == 0.0 {
                format!("{:.0}", val)
            } else {
                format!("{}", val)
            };
            
            
            
            
            let mut new_replaced = String::new();
            let chars: Vec<char> = replaced.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if i + key.len() <= chars.len() {
                    let substr: String = chars[i..i+key.len()].iter().collect();
                    if substr == *key {
                        
                        let prev_ok = i == 0 || !chars[i-1].is_alphanumeric() && chars[i-1] != '_';
                        let next_ok = i + key.len() == chars.len() || 
                                      !chars[i+key.len()].is_alphanumeric() && chars[i+key.len()] != '_';
                        if prev_ok && next_ok {
                            new_replaced.push_str(&val_str);
                            i += key.len();
                            continue;
                        }
                    }
                }
                new_replaced.push(chars[i]);
                i += 1;
            }
            replaced = new_replaced;
        }
        
        eval_math_expression(&replaced).map(|v| {
            if v.fract() == 0.0 {
                format!("{:.0}", v)
            } else {
                format!("{}", v)
            }
        })
    }
    
    /// 条件判断，支持 > < >= <= == !=
    /// 变量名可以直接写（不带花括号），也支持 {变量} 语法
    pub fn eval_condition(&self, cond: &str) -> bool {
        
        
        let ops = vec![">=", "<=", "==", "!=", ">", "<"];
        for op in ops {
            if let Some(pos) = cond.find(op) {
                let left = cond[..pos].trim();
                let right = cond[pos+op.len()..].trim();
                
                if let (Some(l_str), Some(r_str)) = (self.eval_expr(left), self.eval_expr(right)) {
                    if let (Ok(l), Ok(r)) = (l_str.parse::<f64>(), r_str.parse::<f64>()) {
                        return match op {
                            ">" => l > r,
                            "<" => l < r,
                            ">=" => l >= r,
                            "<=" => l <= r,
                            "==" => (l - r).abs() < 1e-9,
                            "!=" => (l - r).abs() >= 1e-9,
                            _ => false,
                        };
                    } else {
                        
                        return match op {
                            "==" => l_str == r_str,
                            "!=" => l_str != r_str,
                            _ => false, 
                        };
                    }
                }
            }
        }
        false
    }

    pub fn serialize(&self) -> HashMap<String, String> {
        self.map.clone()
    }

    pub fn deserialize(&mut self, data: HashMap<String, String>) {
        self.map = data;
    }
}

impl Default for Variables {
    fn default() -> Self {
        Self::new()
    }
}

/// 简单表达式求值（支持 + - * / 和括号）
fn eval_math_expression(expr: &str) -> Option<f64> {
    let expr: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
    if expr.is_empty() {
        return None;
    }
    let chars: Vec<char> = expr.chars().collect();
    let mut pos = 0;
    parse_expr(&chars, &mut pos)
}

fn parse_expr(chars: &[char], pos: &mut usize) -> Option<f64> {
    let mut left = parse_term(chars, pos)?;
    while *pos < chars.len() {
        let op = chars[*pos];
        if op == '+' || op == '-' {
            *pos += 1;
            let right = parse_term(chars, pos)?;
            left = if op == '+' { left + right } else { left - right };
        } else {
            break;
        }
    }
    Some(left)
}

fn parse_term(chars: &[char], pos: &mut usize) -> Option<f64> {
    let mut left = parse_factor(chars, pos)?;
    while *pos < chars.len() {
        let op = chars[*pos];
        if op == '*' || op == '/' {
            *pos += 1;
            let right = parse_factor(chars, pos)?;
            left = if op == '*' { left * right } else { left / right };
        } else {
            break;
        }
    }
    Some(left)
}

fn parse_factor(chars: &[char], pos: &mut usize) -> Option<f64> {
    if *pos >= chars.len() {
        return None;
    }
    let ch = chars[*pos];
    if ch == '(' {
        *pos += 1;
        let val = parse_expr(chars, pos)?;
        if *pos < chars.len() && chars[*pos] == ')' {
            *pos += 1;
            Some(val)
        } else {
            None
        }
    } else if ch.is_digit(10) || ch == '.' {
        let mut num_str = String::new();
        while *pos < chars.len() && (chars[*pos].is_digit(10) || chars[*pos] == '.') {
            num_str.push(chars[*pos]);
            *pos += 1;
        }
        num_str.parse::<f64>().ok()
    } else {
        None
    }
}