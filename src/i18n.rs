//! Runtime locale detection: Windows + China region => Chinese UI messages.

use std::sync::OnceLock;

static IS_CN: OnceLock<bool> = OnceLock::new();

/// Windows AND appears to be in the China region => Chinese UI.
pub fn is_cn() -> bool {
    *IS_CN.get_or_init(|| {
        // manual override for testing / forced locale
        if let Ok(v) = std::env::var("ACHAT_LANG") {
            let v = v.to_lowercase();
            if v == "zh" || v == "zh-cn" || v == "cn" {
                return true;
            }
            if v == "en" {
                return false;
            }
        }
        if !cfg!(windows) {
            return false;
        }
        // 1) OS locale (e.g. zh-CN)
        if let Some(locale) = sys_locale::get_locale() {
            let l = locale.to_lowercase();
            if l.starts_with("zh") {
                return true;
            }
        }
        // 2) Environment hints
        for key in ["LANG", "LC_ALL", "LANGUAGE"] {
            if let Ok(v) = std::env::var(key) {
                let v = v.to_lowercase();
                if v.contains("zh_cn") || v.contains("zh-cn") {
                    return true;
                }
            }
        }
        // 3) Time zone hints
        if let Ok(tz) = std::env::var("TZ") {
            let tz = tz.to_lowercase();
            if tz.contains("shanghai")
                || tz.contains("hongkong")
                || tz.contains("chongqing")
                || tz.contains("urumqi")
            {
                return true;
            }
        }
        // 4) UTC+8 local offset
        use chrono::{Local, Timelike};
        let offset_secs = Local::now().offset().local_minus_utc();
        if offset_secs == 8 * 3600 {
            return true;
        }
        false
    })
}
