use owo_colors::OwoColorize;
use std::fmt::Write;

pub fn apply_gradient(s: &str, start_rgb: (u8, u8, u8), end_rgb: (u8, u8, u8)) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    if n <= 1 {
        return s
            .truecolor(start_rgb.0, start_rgb.1, start_rgb.2)
            .to_string();
    }

    let mut result = String::with_capacity(s.len() * 20);
    for (i, &c) in chars.iter().enumerate() {
        let t = i as f32 / (n - 1) as f32;
        let r = (start_rgb.0 as f32 * (1.0 - t) + end_rgb.0 as f32 * t) as u8;
        let g = (start_rgb.1 as f32 * (1.0 - t) + end_rgb.1 as f32 * t) as u8;
        let b = (start_rgb.2 as f32 * (1.0 - t) + end_rgb.2 as f32 * t) as u8;
        let _ = write!(result, "{}", c.truecolor(r, g, b));
    }
    result
}

pub fn draw_gradient_bar(
    width: usize,
    percentage: f64,
    start_rgb: (u8, u8, u8),
    end_rgb: (u8, u8, u8),
) -> String {
    let filled = ((percentage / 100.0) * width as f64).round() as usize;
    let mut result = String::with_capacity(width * 20 + 8);
    result.push('▕');
    for i in 0..width {
        if i < filled {
            let t = i as f32 / (width.max(1) - 1).max(1) as f32;
            let r = (start_rgb.0 as f32 * (1.0 - t) + end_rgb.0 as f32 * t) as u8;
            let g = (start_rgb.1 as f32 * (1.0 - t) + end_rgb.1 as f32 * t) as u8;
            let b = (start_rgb.2 as f32 * (1.0 - t) + end_rgb.2 as f32 * t) as u8;
            let _ = write!(result, "{}", "█".truecolor(r, g, b));
        } else {
            result.push(' ');
        }
    }
    result.push('▏');
    result
}

pub fn format_size(bytes: u64, use_si: bool) -> String {
    let divisor = if use_si { 1000.0 } else { 1024.0 };
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= divisor && unit_idx < units.len() - 1 {
        size /= divisor;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, units[unit_idx])
    } else {
        if size.fract() == 0.0 {
            format!("{:.0} {}", size, units[unit_idx])
        } else {
            format!("{:.2} {}", size, units[unit_idx])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_binary() {
        assert_eq!(format_size(0, false), "0 B");
        assert_eq!(format_size(1023, false), "1023 B");
        assert_eq!(format_size(1024, false), "1 KB");
        assert_eq!(format_size(1024 * 1024, false), "1 MB");
        assert_eq!(format_size(1500, false), "1.46 KB");
    }

    #[test]
    fn test_format_size_si() {
        assert_eq!(format_size(0, true), "0 B");
        assert_eq!(format_size(999, true), "999 B");
        assert_eq!(format_size(1000, true), "1 KB");
        assert_eq!(format_size(1000 * 1000, true), "1 MB");
        assert_eq!(format_size(1500, true), "1.50 KB");
    }

    #[test]
    fn test_apply_gradient() {
        let s = "test";
        let grad = apply_gradient(s, (0, 0, 0), (255, 255, 255));
        assert!(grad.contains('t'));
        assert!(grad.contains('e'));
        assert!(grad.contains('s'));
        assert!(grad.len() > s.len());
    }

    #[test]
    fn test_draw_gradient_bar() {
        let bar = draw_gradient_bar(10, 50.0, (0, 0, 0), (255, 255, 255));
        assert!(bar.contains("▕"));
        assert!(bar.contains("▏"));
        assert!(bar.contains("█"));
    }
}
