// reservation_validator.rs - 移除未使用的变量

use chrono::{DateTime, Utc, Duration, NaiveDate, NaiveDateTime, Timelike};

/// 预约时间验证结果
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub message: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 验证预约时间
pub fn validate_reservation_time(
    start_time_str: &str,
    end_time_str: &str,
) -> ValidationResult {
    // 1. 解析时间
    let start_time = match DateTime::parse_from_rfc3339(start_time_str) {
        Ok(t) => t.with_timezone(&Utc),
        Err(e) => {
            return ValidationResult {
                is_valid: false,
                message: format!("开始时间格式错误: {}", e),
                start_time: None,
                end_time: None,
            };
        }
    };
    
    let end_time = match DateTime::parse_from_rfc3339(end_time_str) {
        Ok(t) => t.with_timezone(&Utc),
        Err(e) => {
            return ValidationResult {
                is_valid: false,
                message: format!("结束时间格式错误: {}", e),
                start_time: None,
                end_time: None,
            };
        }
    };
    
    let now = Utc::now();
    let now_date = now.date_naive();
    
    // 2. 检查开始时间是否在当前时间之后（允许提前15分钟）
    if start_time < now {
        return ValidationResult {
            is_valid: false,
            message: "开始时间不能早于当前时间".to_string(),
            start_time: None,
            end_time: None,
        };
    }
    
    // 3. 检查预约日期是否在三天内（当天起算）
    let start_date = start_time.date_naive();
    let max_date = now_date + Duration::days(3);
    
    if start_date < now_date {
        return ValidationResult {
            is_valid: false,
            message: "预约日期不能早于今天".to_string(),
            start_time: None,
            end_time: None,
        };
    }
    
    if start_date > max_date {
        return ValidationResult {
            is_valid: false,
            message: format!("预约日期不能超过三天（最大允许日期: {}）", max_date),
            start_time: None,
            end_time: None,
        };
    }
    
    // 4. 检查结束时间是否晚于开始时间
    if end_time <= start_time {
        return ValidationResult {
            is_valid: false,
            message: "结束时间必须晚于开始时间".to_string(),
            start_time: None,
            end_time: None,
        };
    }
    
    // 5. 检查预约时长（最长4小时，最短30分钟）
    let duration = end_time.signed_duration_since(start_time);
    if duration.num_hours() > 4 {
        return ValidationResult {
            is_valid: false,
            message: "单次预约最长不能超过4小时".to_string(),
            start_time: None,
            end_time: None,
        };
    }
    
    if duration.num_minutes() < 30 {
        return ValidationResult {
            is_valid: false,
            message: "单次预约最短不能少于30分钟".to_string(),
            start_time: None,
            end_time: None,
        };
    }
    
    // 6. 检查开始时间是否在整点或半点（可选）
    let start_minute = start_time.minute();
    if start_minute != 0 && start_minute != 30 {
        return ValidationResult {
            is_valid: false,
            message: "预约开始时间建议为整点或半点（如 14:00 或 14:30）".to_string(),
            start_time: None,
            end_time: None,
        };
    }
    
    ValidationResult {
        is_valid: true,
        message: "预约时间验证通过".to_string(),
        start_time: Some(start_time),
        end_time: Some(end_time),
    }
}

/// 获取可用预约日期（三天内）
pub fn get_available_dates() -> Vec<String> {
    let now = Utc::now();
    let dates: Vec<String> = (0..=3)
        .map(|i| {
            let date = now.date_naive() + Duration::days(i);
            date.format("%Y-%m-%d").to_string()
        })
        .collect();
    dates
}

/// 获取某个日期的可用时间段（9:00 - 21:00，每小时一个时段）
pub fn get_available_time_slots(date_str: &str) -> Vec<String> {
    let mut slots = Vec::new();
    
    // 解析日期
    let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return slots,
    };
    
    let now = Utc::now();
    let now_date = now.date_naive();
    
    // 9:00 到 21:00，每小时一个时段
    for hour in 9..21 {
        let time_str = format!("{:02}:00", hour);
        
        // 如果是今天，跳过已经过去的时间（提前30分钟）
        if date == now_date {
            let slot_time = NaiveDateTime::parse_from_str(
                &format!("{} {}", date_str, time_str),
                "%Y-%m-%d %H:%M"
            ).unwrap();
            
            let slot_datetime = DateTime::<Utc>::from_naive_utc_and_offset(
                slot_time,
                Utc
            );
            
            // 如果这个时段已经过去，跳过
            if slot_datetime < now - Duration::minutes(30) {
                continue;
            }
        }
        
        slots.push(time_str);
    }
    
    slots
}