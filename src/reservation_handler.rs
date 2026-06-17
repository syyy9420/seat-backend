// reservation_handler.rs - 完整修复版本

use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Utc};  // 移除未使用的 Duration
use rusqlite::{Connection, params};
use std::sync::Mutex;

use crate::models::{
    ApiResponse, CreateReservationRequest, ExtendReservationRequest, Reservation,
};

/// 创建预约
pub async fn create_reservation(
    req: web::Json<CreateReservationRequest>,
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let now = Utc::now().to_rfc3339();
    
    // 验证时间
    let start_time = match DateTime::parse_from_rfc3339(&req.start_time) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                message: "开始时间格式错误".to_string(),
                data: None,
            });
        }
    };
    
    let end_time = match DateTime::parse_from_rfc3339(&req.end_time) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                message: "结束时间格式错误".to_string(),
                data: None,
            });
        }
    };
    
    let now_time = Utc::now();
    if start_time < now_time {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "开始时间不能早于当前时间".to_string(),
            data: None,
        });
    }
    
    if end_time <= start_time {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "结束时间必须晚于开始时间".to_string(),
            data: None,
        });
    }
    
    let duration = end_time.signed_duration_since(start_time);
    if duration.num_hours() > 4 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "单次预约最长不能超过4小时".to_string(),
            data: None,
        });
    }
    
    let conn = db.lock().unwrap();
    
    // 检查座位是否存在且可用
    let seat_status: String = match conn.query_row(
        "SELECT status FROM seats WHERE id = ?1",
        params![req.seat_id],
        |row| row.get(0),
    ) {
        Ok(status) => status,
        Err(_) => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: "座位不存在".to_string(),
                data: None,
            });
        }
    };
    
    if seat_status != "available" {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "该座位当前不可用".to_string(),
            data: None,
        });
    }
    
    // 检查时间冲突
    let conflict: bool = match conn.query_row(
        "SELECT 1 FROM reservations 
         WHERE seat_id = ?1 
         AND status IN ('pending', 'active')
         AND start_time <= ?3 
         AND end_time >= ?2",
        params![req.seat_id, &req.start_time, &req.end_time],
        |_| Ok(true),
    ) {
        Ok(_) => true,
        Err(_) => false,
    };
    
    if conflict {
        return HttpResponse::Conflict().json(ApiResponse::<()> {
            success: false,
            message: "该时间段座位已被预约".to_string(),
            data: None,
        });
    }
    
    // 检查用户是否有未完成的预约
    let active_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM reservations WHERE user_id = ?1 AND status IN ('pending', 'active')",
        params![user_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    if active_count >= 3 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "您已有3个进行中的预约，请先取消或完成后再预约".to_string(),
            data: None,
        });
    }
    
    // 创建预约
    match conn.execute(
        "INSERT INTO reservations (user_id, seat_id, start_time, end_time, status, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?5)",
        params![user_id, req.seat_id, &req.start_time, &req.end_time, &now],
    ) {
        Ok(_) => {
            // 更新座位状态为 reserved
            let _ = conn.execute(
                "UPDATE seats SET status = 'reserved', updated_at = ?1 WHERE id = ?2",
                params![&now, req.seat_id],
            );
            
            HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                message: "预约成功，请在规定时间内签到".to_string(),
                data: None,
            })
        },
        Err(e) => {
            eprintln!("创建预约失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "预约失败，请稍后重试".to_string(),
                data: None,
            })
        }
    }
}

/// 获取我的预约列表
pub async fn get_my_reservations(
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let conn = db.lock().unwrap();
    
    let sql = r#"
        SELECT r.id, r.user_id, r.seat_id, r.start_time, r.end_time, r.status, r.created_at, r.updated_at,
               s.seat_number, s.area
        FROM reservations r
        JOIN seats s ON r.seat_id = s.id
        WHERE r.user_id = ?1
        ORDER BY r.created_at DESC
    "#;
    
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Reservation>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let rows = match stmt.query_map([&user_id], |row| {
        Ok(Reservation {
            id: row.get(0)?,
            user_id: row.get(1)?,
            seat_id: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            status: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
            seat_number: row.get(8)?,
            area: row.get(9)?,
        })
    }) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Reservation>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let reservations: Vec<Reservation> = rows.filter_map(|r| r.ok()).collect();
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("共 {} 条预约记录", reservations.len()),
        data: Some(reservations),
    })
}

/// 获取预约详情
pub async fn get_reservation_detail(
    path: web::Path<i32>,
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let reservation_id = path.into_inner();
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let conn = db.lock().unwrap();
    
    let sql = r#"
        SELECT r.id, r.user_id, r.seat_id, r.start_time, r.end_time, r.status, r.created_at, r.updated_at,
               s.seat_number, s.area
        FROM reservations r
        JOIN seats s ON r.seat_id = s.id
        WHERE r.id = ?1 AND r.user_id = ?2
    "#;
    
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Reservation> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let mut rows = match stmt.query(params![reservation_id, user_id]) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Reservation> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    if let Some(row) = rows.next().unwrap() {
        let reservation = Reservation {
            id: row.get(0).unwrap(),
            user_id: row.get(1).unwrap(),
            seat_id: row.get(2).unwrap(),
            start_time: row.get(3).unwrap(),
            end_time: row.get(4).unwrap(),
            status: row.get(5).unwrap(),
            created_at: row.get(6).unwrap(),
            updated_at: row.get(7).unwrap(),
            seat_number: row.get(8).unwrap(),
            area: row.get(9).unwrap(),
        };
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "获取成功".to_string(),
            data: Some(reservation),
        })
    } else {
        HttpResponse::NotFound().json(ApiResponse::<Reservation> {
            success: false,
            message: "预约记录不存在或无权限访问".to_string(),
            data: None,
        })
    }
}

/// 取消预约（修复版）
pub async fn cancel_reservation(
    path: web::Path<i32>,
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let reservation_id = path.into_inner();
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let now = Utc::now().to_rfc3339();
    
    // 获取数据库连接 - 声明为 mut
    let mut conn = db.lock().unwrap();
    
    // 1. 先查询预约信息，确认存在且属于该用户
    let reservation_info = match conn.query_row(
        "SELECT seat_id, start_time, status FROM reservations WHERE id = ?1 AND user_id = ?2",
        params![reservation_id, user_id],
        |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?
            ))
        }
    ) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("查询预约失败: {}", e);
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: "预约记录不存在或无权限".to_string(),
                data: None,
            });
        }
    };
    
    let (seat_id, start_time_str, status) = reservation_info;
    
    // 2. 检查状态（只有 pending 和 active 可以取消）
    if status != "pending" && status != "active" {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: format!("当前状态({})无法取消预约", status),
            data: None,
        });
    }
    
    // 3. 检查是否可以取消（开始前30分钟内不能取消）
    if let Ok(start_time) = DateTime::parse_from_rfc3339(&start_time_str) {
        let now_time = Utc::now();
        let time_to_start = start_time.signed_duration_since(now_time);
        
        if time_to_start.num_minutes() < 30 && status == "pending" {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                message: "预约开始前30分钟内不能取消，如需取消请联系管理员".to_string(),
                data: None,
            });
        }
    }
    
    // 4. 开启事务，确保原子性
    let transaction = match conn.transaction() {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("开启事务失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "取消预约失败".to_string(),
                data: None,
            });
        }
    };
    
    // 5. 更新预约状态为 cancelled
    if let Err(e) = transaction.execute(
        "UPDATE reservations SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
        params![&now, reservation_id],
    ) {
        eprintln!("更新预约状态失败: {}", e);
        let _ = transaction.rollback();
        return HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            message: "取消预约失败".to_string(),
            data: None,
        });
    }
    
    // 6. 释放座位 - 将座位状态改为 available
    if let Err(e) = transaction.execute(
        "UPDATE seats SET status = 'available', updated_at = ?1 WHERE id = ?2",
        params![&now, seat_id],
    ) {
        eprintln!("更新座位状态失败: {}", e);
        let _ = transaction.rollback();
        return HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            message: "取消预约失败".to_string(),
            data: None,
        });
    }
    
    // 7. 如果有签到记录，也更新状态（如果存在）
    let _ = transaction.execute(
        "UPDATE attendance SET checkout_time = ?1 WHERE reservation_id = ?2 AND checkout_time IS NULL",
        params![&now, reservation_id],
    );
    
    // 8. 提交事务
    if let Err(e) = transaction.commit() {
        eprintln!("提交事务失败: {}", e);
        return HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            message: "取消预约失败".to_string(),
            data: None,
        });
    }
    
    HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "预约已取消，座位已释放".to_string(),
        data: None,
    })
}

/// 续约/延长使用时间
pub async fn extend_reservation(
    path: web::Path<i32>,
    req: web::Json<ExtendReservationRequest>,
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let reservation_id = path.into_inner();
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    
    if req.extra_hours < 1 || req.extra_hours > 2 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "延长时长必须在1-2小时之间".to_string(),
            data: None,
        });
    }
    
    let (seat_id, end_time_str, status): (i32, String, String) = match conn.query_row(
        "SELECT seat_id, end_time, status FROM reservations WHERE id = ?1 AND user_id = ?2",
        params![reservation_id, user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ) {
        Ok(data) => data,
        Err(_) => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: "预约记录不存在或无权限".to_string(),
                data: None,
            });
        }
    };
    
    if status != "active" {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "只有进行中的预约才能续约".to_string(),
            data: None,
        });
    }
    
    let end_time = match DateTime::parse_from_rfc3339(&end_time_str) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "时间格式错误".to_string(),
                data: None,
            });
        }
    };
    
    let new_end_time = end_time + chrono::Duration::hours(req.extra_hours as i64);
    let new_end_time_str = new_end_time.to_rfc3339();
    
    let conflict: bool = match conn.query_row(
        "SELECT 1 FROM reservations 
         WHERE seat_id = ?1 
         AND id != ?2
         AND status IN ('pending', 'active')
         AND start_time <= ?4 
         AND end_time >= ?3",
        params![seat_id, reservation_id, &end_time_str, &new_end_time_str],
        |_| Ok(true),
    ) {
        Ok(_) => true,
        Err(_) => false,
    };
    
    if conflict {
        return HttpResponse::Conflict().json(ApiResponse::<()> {
            success: false,
            message: "延长的时间段与其他预约冲突".to_string(),
            data: None,
        });
    }
    
    match conn.execute(
        "UPDATE reservations SET end_time = ?1, updated_at = ?2 WHERE id = ?3",
        params![&new_end_time_str, &now, reservation_id],
    ) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: format!("成功延长{}小时", req.extra_hours),
            data: None,
        }),
        Err(e) => {
            eprintln!("续约失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "续约失败".to_string(),
                data: None,
            })
        }
    }
}