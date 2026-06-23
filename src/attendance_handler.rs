// attendance_handler.rs - 修复所有类型错误

use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Utc};  // 移除未使用的 Duration
use rusqlite::Connection;
use std::sync::Mutex;

use crate::models::{
    ApiResponse, AttendanceStatus, CheckinRequest, CheckoutRequest, CheckinResponse, CheckoutResponse,
};

/// 签到
pub async fn checkin(
    req: web::Json<CheckinRequest>,
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let now = Utc::now().to_rfc3339();
    let conn = db.lock().unwrap();
    
    let (seat_id, start_time_str, status): (i32, String, String) = match conn.query_row(
        "SELECT seat_id, start_time, status FROM reservations WHERE id = ?1 AND user_id = ?2",
        [&req.reservation_id, &user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ) {
        Ok(data) => data,
        Err(_) => {
            return HttpResponse::NotFound().json(ApiResponse::<CheckinResponse> {
                success: false,
                message: "预约记录不存在或无权限".to_string(),
                data: None,
            });
        }
    };
    
    if status != "pending" {
        return HttpResponse::BadRequest().json(ApiResponse::<CheckinResponse> {
            success: false,
            message: format!("当前状态({})无法签到", status),
            data: None,
        });
    }
    
    let start_time = match DateTime::parse_from_rfc3339(&start_time_str) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiResponse::<CheckinResponse> {
                success: false,
                message: "时间格式错误".to_string(),
                data: None,
            });
        }
    };
    
    let now_time = Utc::now();
    let time_diff = start_time.signed_duration_since(now_time);
    
    if time_diff.num_minutes() > 15 {
        return HttpResponse::BadRequest().json(ApiResponse::<CheckinResponse> {
            success: false,
            message: "签到时间未到，请准时签到".to_string(),
            data: None,
        });
    }
    
    if time_diff.num_minutes() < -30 {
        let _ = conn.execute(
            "UPDATE reservations SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
            (&now, &req.reservation_id),
        );
        let _ = conn.execute(
            "UPDATE seats SET status = 'available', updated_at = ?1 WHERE id = ?2",
            (&now, &seat_id),
        );
        
        return HttpResponse::BadRequest().json(ApiResponse::<CheckinResponse> {
            success: false,
            message: "签到时间已过30分钟，预约已自动取消".to_string(),
            data: None,
        });
    }
    
    match conn.execute(
        "UPDATE reservations SET status = 'active', updated_at = ?1 WHERE id = ?2",
        (&now, &req.reservation_id),
    ) {
        Ok(_) => {
            let _ = conn.execute(
                "UPDATE seats SET status = 'occupied', updated_at = ?1 WHERE id = ?2",
                (&now, &seat_id),
            );
            
            let _ = conn.execute(
                "INSERT INTO attendance (user_id, reservation_id, seat_id, checkin_time, location) 
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (&user_id, &req.reservation_id, &seat_id, &now, &req.location),
            );
            
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: "签到成功".to_string(),
                data: Some(CheckinResponse {
                    reservation_id: req.reservation_id,
                    checkin_time: now,
                    message: "请遵守图书馆规定，保持安静".to_string(),
                }),
            })
        },
        Err(e) => {
            eprintln!("签到失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<CheckinResponse> {
                success: false,
                message: "签到失败，请稍后重试".to_string(),
                data: None,
            })
        }
    }
}

/// 签退
pub async fn checkout(
    req: web::Json<CheckoutRequest>,
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let now = Utc::now().to_rfc3339();
    let conn = db.lock().unwrap();
    
    let (seat_id, start_time_str, status): (i32, String, String) = match conn.query_row(
        "SELECT seat_id, start_time, status FROM reservations WHERE id = ?1 AND user_id = ?2",
        [&req.reservation_id, &user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ) {
        Ok(data) => data,
        Err(_) => {
            return HttpResponse::NotFound().json(ApiResponse::<CheckoutResponse> {
                success: false,
                message: "预约记录不存在或无权限".to_string(),
                data: None,
            });
        }
    };
    
    if status != "active" {
        return HttpResponse::BadRequest().json(ApiResponse::<CheckoutResponse> {
            success: false,
            message: format!("当前状态({})无法签退", status),
            data: None,
        });
    }
    
    let _ = conn.execute(
        "UPDATE attendance SET checkout_time = ?1 WHERE user_id = ?2 AND reservation_id = ?3",
        (&now, &user_id, &req.reservation_id),
    );
    
    match conn.execute(
        "UPDATE reservations SET status = 'completed', updated_at = ?1 WHERE id = ?2",
        (&now, &req.reservation_id),
    ) {
        Ok(_) => {
            let _ = conn.execute(
                "UPDATE seats SET status = 'available', updated_at = ?1 WHERE id = ?2",
                (&now, &seat_id),
            );
            
            let start_time = DateTime::parse_from_rfc3339(&start_time_str).unwrap();
            let duration = Utc::now().signed_duration_since(start_time);
            
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: "签退成功，感谢您的使用".to_string(),
                data: Some(CheckoutResponse {
                    reservation_id: req.reservation_id,
                    checkout_time: now,
                    duration_minutes: duration.num_minutes(),
                    message: format!("本次使用时长: {}分钟", duration.num_minutes()),
                }),
            })
        },
        Err(e) => {
            eprintln!("签退失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<CheckoutResponse> {
                success: false,
                message: "签退失败".to_string(),
                data: None,
            })
        }
    }
}

/// 获取当前签到状态
pub async fn get_attendance_status(
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let conn = db.lock().unwrap();
    
    let sql = r#"
        SELECT r.id, r.seat_id, s.seat_number, r.start_time, r.end_time, a.checkin_time, a.checkout_time
        FROM reservations r
        JOIN seats s ON r.seat_id = s.id
        LEFT JOIN attendance a ON a.reservation_id = r.id AND a.user_id = r.user_id
        WHERE r.user_id = ?1 AND r.status = 'active'
        LIMIT 1
    "#;
    
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<AttendanceStatus> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let mut rows = match stmt.query([&user_id]) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<AttendanceStatus> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    if let Some(row) = rows.next().unwrap() {
        let status = AttendanceStatus {
            is_checked_in: true,
            reservation_id: Some(row.get(0).unwrap()),
            seat_id: Some(row.get(1).unwrap()),
            seat_number: row.get(2).unwrap(),
            checkin_time: row.get(5).unwrap(),
            checkout_time: row.get(6).unwrap(),
            start_time: row.get(3).unwrap(),
            end_time: row.get(4).unwrap(),
        };
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "当前正在使用中".to_string(),
            data: Some(status),
        })
    } else {
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "当前未签到".to_string(),
            data: Some(AttendanceStatus {
                is_checked_in: false,
                reservation_id: None,
                seat_id: None,
                seat_number: None,
                checkin_time: None,
                checkout_time: None,
                start_time: None,
                end_time: None,
            }),
        })
    }
}

/// 自动签退任务
pub async fn auto_checkout(db: web::Data<Mutex<Connection>>) {
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    
    let expired_reservations: Vec<(i32, i32)> = match conn.prepare(
        "SELECT id, seat_id FROM reservations WHERE status = 'active' AND end_time <= ?1"
    ) {
        Ok(mut stmt) => {
            let rows = stmt.query_map([&now], |row| {
                Ok((row.get(0)?, row.get(1)?))
            });
            match rows {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(e) => {
                    eprintln!("查询过期预约失败: {}", e);
                    vec![]
                }
            }
        },
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            vec![]
        }
    };
    
    for (reservation_id, seat_id) in expired_reservations {
        let _ = conn.execute(
            "UPDATE reservations SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            (&now, &reservation_id),
        );
        let _ = conn.execute(
            "UPDATE seats SET status = 'available', updated_at = ?1 WHERE id = ?2",
            (&now, &seat_id),
        );
        println!("自动签退预约: {}", reservation_id);
    }
}