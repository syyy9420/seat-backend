// seat_handler.rs - 完整修改版本，支持多选功能

use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use rusqlite::{Connection, params};
use std::sync::Mutex;

use crate::models::{
    ApiResponse, 
    //CreateSeatRequest, 
    CreateSeatRequestExtended,
    //UpdateSeatRequest,
    UpdateSeatRequestExtended,
    //GetSeatsQuery, 
    Seat, 
    SeatInfo,
    FloorStats, 
    RoomInfo, 
    RoomStats,
    SeatQueryParams,
    AvailableSeatsQuery,
    SeatFeature,
};

/// 解析功能参数，支持多选（用逗号分隔）
fn parse_features(features_str: &str) -> Vec<SeatFeature> {
    features_str
        .split(',')
        .filter_map(|s| SeatFeature::from_str(s.trim()))
        .collect()
}

/// 获取所有座位列表（支持多选筛选）
pub async fn get_seats(
    query: web::Query<SeatQueryParams>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();
    
    // 检查是否有扩展列（room_type）
    let has_extended = conn.query_row(
        "SELECT 1 FROM seats WHERE room_type IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    let mut sql = if has_extended {
        String::from(
            "SELECT id, seat_number, area, floor, room_type, room_name, 
             is_near_socket, is_near_window, is_quiet_zone, seat_type, 
             status, x_coord, y_coord, created_at, updated_at 
             FROM seats WHERE 1=1"
        )
    } else {
        // 兼容旧版本
        String::from(
            "SELECT id, seat_number, area, status, x_coord, y_coord, created_at, updated_at 
             FROM seats WHERE 1=1"
        )
    };
    
    let mut params_vec: Vec<String> = Vec::new();
    
    // 公共筛选
    if let Some(area) = &query.area {
        sql.push_str(" AND area = ?");
        params_vec.push(area.clone());
    }
    
    if let Some(status) = &query.status {
        sql.push_str(" AND status = ?");
        params_vec.push(status.clone());
    }
    
    // 扩展筛选
    if has_extended {
        if let Some(floor) = &query.floor {
            sql.push_str(" AND floor = ?");
            params_vec.push(floor.to_string());
        }
        
        // 房间类型筛选（hall, large, medium, small）
        if let Some(room_type) = &query.room_type {
            sql.push_str(" AND room_type = ?");
            params_vec.push(room_type.clone());
        }
        
        // 房间名称筛选（大厅, 大自习室, 中自习室, 小自习室）
        if let Some(room_name) = &query.room_name {
            sql.push_str(" AND room_name = ?");
            params_vec.push(room_name.clone());
        }
        
        // ========== 多选功能筛选 ==========
        // 优先使用 features 参数（多选，AND 逻辑）
        if let Some(features_str) = &query.features {
            let features = parse_features(features_str);
            
            if !features.is_empty() {
                let mut feature_conditions = Vec::new();
                
                for feature in &features {
                    match feature {
                        SeatFeature::NearSocket => {
                            feature_conditions.push("is_near_socket = 1");
                        }
                        SeatFeature::NearWindow => {
                            feature_conditions.push("is_near_window = 1");
                        }
                        SeatFeature::QuietZone => {
                            feature_conditions.push("is_quiet_zone = 1");
                        }
                    }
                }
                
                // 使用 AND 连接多个条件（同时满足所有选中的功能）
                if !feature_conditions.is_empty() {
                    sql.push_str(" AND (");
                    sql.push_str(&feature_conditions.join(" AND "));
                    sql.push_str(")");
                }
            }
        } else {
            // 兼容旧版单个字段筛选（OR 逻辑，满足任意一个即可）
            let mut feature_conditions = Vec::new();
            
            if let Some(near_socket) = &query.is_near_socket {
                if *near_socket {
                    feature_conditions.push("is_near_socket = 1");
                }
            }
            
            if let Some(near_window) = &query.is_near_window {
                if *near_window {
                    feature_conditions.push("is_near_window = 1");
                }
            }
            
            if let Some(quiet_zone) = &query.is_quiet_zone {
                if *quiet_zone {
                    feature_conditions.push("is_quiet_zone = 1");
                }
            }
            
            if !feature_conditions.is_empty() {
                sql.push_str(" AND (");
                sql.push_str(&feature_conditions.join(" OR "));
                sql.push_str(")");
            }
        }
        
        if let Some(seat_type) = &query.seat_type {
            sql.push_str(" AND seat_type = ?");
            params_vec.push(seat_type.clone());
        }
        
        sql.push_str(" ORDER BY floor, room_type, seat_number");
    } else {
        sql.push_str(" ORDER BY area, seat_number");
    }
    
    let mut stmt = match conn.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<SeatInfo>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let param_refs: Vec<&str> = params_vec.iter().map(|s| s.as_str()).collect();
    
    if has_extended {
        let rows = if params_vec.is_empty() {
            match stmt.query_map([], |row| {
                Ok(SeatInfo {
                    id: row.get(0)?,
                    seat_number: row.get(1)?,
                    area: row.get(2)?,
                    floor: row.get(3)?,
                    room_type: row.get(4)?,
                    room_name: row.get(5)?,
                    is_near_socket: row.get(6).unwrap_or(false),
                    is_near_window: row.get(7).unwrap_or(false),
                    is_quiet_zone: row.get(8).unwrap_or(false),
                    seat_type: row.get(9).unwrap_or("standard".to_string()),
                    status: row.get(10)?,
                    x_coord: row.get(11)?,
                    y_coord: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            }) {
                Ok(rows) => rows.collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("查询失败: {}", e);
                    return HttpResponse::InternalServerError().json(ApiResponse::<Vec<SeatInfo>> {
                        success: false,
                        message: "服务器内部错误".to_string(),
                        data: None,
                    });
                }
            }
        } else {
            match stmt.query_map(rusqlite::params_from_iter(param_refs), |row| {
                Ok(SeatInfo {
                    id: row.get(0)?,
                    seat_number: row.get(1)?,
                    area: row.get(2)?,
                    floor: row.get(3)?,
                    room_type: row.get(4)?,
                    room_name: row.get(5)?,
                    is_near_socket: row.get(6).unwrap_or(false),
                    is_near_window: row.get(7).unwrap_or(false),
                    is_quiet_zone: row.get(8).unwrap_or(false),
                    seat_type: row.get(9).unwrap_or("standard".to_string()),
                    status: row.get(10)?,
                    x_coord: row.get(11)?,
                    y_coord: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            }) {
                Ok(rows) => rows.collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("查询失败: {}", e);
                    return HttpResponse::InternalServerError().json(ApiResponse::<Vec<SeatInfo>> {
                        success: false,
                        message: "服务器内部错误".to_string(),
                        data: None,
                    });
                }
            }
        };
        
        let seats: Vec<SeatInfo> = rows.into_iter().filter_map(|r| r.ok()).collect();
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: format!("共 {} 个座位", seats.len()),
            data: Some(seats),
        })
    } else {
        // 基础模式（兼容旧版本）
        let rows = if params_vec.is_empty() {
            match stmt.query_map([], |row| {
                Ok(Seat {
                    id: row.get(0)?,
                    seat_number: row.get(1)?,
                    area: row.get(2)?,
                    status: row.get(3)?,
                    x_coord: row.get(4)?,
                    y_coord: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            }) {
                Ok(rows) => rows.collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("查询失败: {}", e);
                    return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Seat>> {
                        success: false,
                        message: "服务器内部错误".to_string(),
                        data: None,
                    });
                }
            }
        } else {
            match stmt.query_map(rusqlite::params_from_iter(param_refs), |row| {
                Ok(Seat {
                    id: row.get(0)?,
                    seat_number: row.get(1)?,
                    area: row.get(2)?,
                    status: row.get(3)?,
                    x_coord: row.get(4)?,
                    y_coord: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            }) {
                Ok(rows) => rows.collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("查询失败: {}", e);
                    return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Seat>> {
                        success: false,
                        message: "服务器内部错误".to_string(),
                        data: None,
                    });
                }
            }
        };
        
        let seats: Vec<Seat> = rows.into_iter().filter_map(|r| r.ok()).collect();
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: format!("共 {} 个座位", seats.len()),
            data: Some(seats),
        })
    }
}

/// 获取单个座位详细信息
pub async fn get_seat_by_id(
    seat_id: web::Path<i32>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let seat_id = seat_id.into_inner();
    let conn = db.lock().unwrap();
    
    let has_extended = conn.query_row(
        "SELECT 1 FROM seats WHERE room_type IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    if has_extended {
        let mut stmt = match conn.prepare(
            "SELECT id, seat_number, area, floor, room_type, room_name, 
             is_near_socket, is_near_window, is_quiet_zone, seat_type, 
             status, x_coord, y_coord, created_at, updated_at 
             FROM seats WHERE id = ?1"
        ) {
            Ok(stmt) => stmt,
            Err(e) => {
                eprintln!("准备查询失败: {}", e);
                return HttpResponse::InternalServerError().json(ApiResponse::<SeatInfo> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        };
        
        let mut rows = match stmt.query(params![seat_id]) {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("查询失败: {}", e);
                return HttpResponse::InternalServerError().json(ApiResponse::<SeatInfo> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        };
        
        if let Some(row) = rows.next().unwrap() {
            let seat = SeatInfo {
                id: row.get(0).unwrap(),
                seat_number: row.get(1).unwrap(),
                area: row.get(2).unwrap(),
                floor: row.get(3).unwrap(),
                room_type: row.get(4).unwrap(),
                room_name: row.get(5).unwrap(),
                is_near_socket: row.get(6).unwrap_or(false),
                is_near_window: row.get(7).unwrap_or(false),
                is_quiet_zone: row.get(8).unwrap_or(false),
                seat_type: row.get(9).unwrap_or("standard".to_string()),
                status: row.get(10).unwrap(),
                x_coord: row.get(11).unwrap(),
                y_coord: row.get(12).unwrap(),
                created_at: row.get(13).unwrap(),
                updated_at: row.get(14).unwrap(),
            };
            
            return HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: "获取成功".to_string(),
                data: Some(seat),
            });
        }
    }
    
    // 基础模式（兼容旧版本）
    let mut stmt = match conn.prepare(
        "SELECT id, seat_number, area, status, x_coord, y_coord, created_at, updated_at FROM seats WHERE id = ?1"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Seat> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let mut rows = match stmt.query(params![seat_id]) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Seat> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    if let Some(row) = rows.next().unwrap() {
        let seat = Seat {
            id: row.get(0).unwrap(),
            seat_number: row.get(1).unwrap(),
            area: row.get(2).unwrap(),
            status: row.get(3).unwrap(),
            x_coord: row.get(4).unwrap(),
            y_coord: row.get(5).unwrap(),
            created_at: row.get(6).unwrap(),
            updated_at: row.get(7).unwrap(),
        };
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "获取成功".to_string(),
            data: Some(seat),
        })
    } else {
        HttpResponse::NotFound().json(ApiResponse::<Seat> {
            success: false,
            message: "座位不存在".to_string(),
            data: None,
        })
    }
}

/// 获取当前空闲座位
pub async fn get_available_seats(
    query: web::Query<AvailableSeatsQuery>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    
    let has_extended = conn.query_row(
        "SELECT 1 FROM seats WHERE room_type IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    let sql = if has_extended {
        r#"
            SELECT s.id, s.seat_number, s.area, s.floor, s.room_type, s.room_name,
                   s.is_near_socket, s.is_near_window, s.is_quiet_zone, s.seat_type, 
                   s.status, s.x_coord, s.y_coord, s.created_at, s.updated_at 
            FROM seats s
            WHERE s.status = 'available'
            AND NOT EXISTS (
                SELECT 1 FROM reservations r 
                WHERE r.seat_id = s.id 
                AND r.status IN ('pending', 'active')
                AND r.start_time <= ?2
                AND r.end_time >= ?1
            )
            ORDER BY s.floor, s.room_type, s.seat_number
        "#
    } else {
        r#"
            SELECT s.id, s.seat_number, s.area, s.status, s.x_coord, s.y_coord, s.created_at, s.updated_at 
            FROM seats s
            WHERE s.status = 'available'
            AND NOT EXISTS (
                SELECT 1 FROM reservations r 
                WHERE r.seat_id = s.id 
                AND r.status IN ('pending', 'active')
                AND r.start_time <= ?2
                AND r.end_time >= ?1
            )
            ORDER BY s.area, s.seat_number
        "#
    };
    
    let start_time = query.start_time.as_ref().unwrap_or(&now);
    let end_time = query.end_time.as_ref().unwrap_or(&now);
    
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<SeatInfo>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    if has_extended {
        let rows = match stmt.query_map(params![start_time, end_time], |row| {
            Ok(SeatInfo {
                id: row.get(0)?,
                seat_number: row.get(1)?,
                area: row.get(2)?,
                floor: row.get(3)?,
                room_type: row.get(4)?,
                room_name: row.get(5)?,
                is_near_socket: row.get(6).unwrap_or(false),
                is_near_window: row.get(7).unwrap_or(false),
                is_quiet_zone: row.get(8).unwrap_or(false),
                seat_type: row.get(9).unwrap_or("standard".to_string()),
                status: row.get(10)?,
                x_coord: row.get(11)?,
                y_coord: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("查询失败: {}", e);
                return HttpResponse::InternalServerError().json(ApiResponse::<Vec<SeatInfo>> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        };
        
        let seats: Vec<SeatInfo> = rows.filter_map(|r| r.ok()).collect();
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: format!("当前共有 {} 个空闲座位", seats.len()),
            data: Some(seats),
        })
    } else {
        let rows = match stmt.query_map(params![start_time, end_time], |row| {
            Ok(Seat {
                id: row.get(0)?,
                seat_number: row.get(1)?,
                area: row.get(2)?,
                status: row.get(3)?,
                x_coord: row.get(4)?,
                y_coord: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("查询失败: {}", e);
                return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Seat>> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        };
        
        let seats: Vec<Seat> = rows.filter_map(|r| r.ok()).collect();
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: format!("当前共有 {} 个空闲座位", seats.len()),
            data: Some(seats),
        })
    }
}

/// 获取楼层统计
pub async fn get_floor_stats(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();
    
    let has_floor = conn.query_row(
        "SELECT 1 FROM seats WHERE floor IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    if !has_floor {
        return HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "暂无楼层数据".to_string(),
            data: Some(Vec::<FloorStats>::new()),
        });
    }
    
    let sql = r#"
        SELECT 
            floor,
            COUNT(*) as total,
            SUM(CASE WHEN status = 'available' THEN 1 ELSE 0 END) as available,
            SUM(CASE WHEN status = 'occupied' THEN 1 ELSE 0 END) as occupied
        FROM seats
        GROUP BY floor
        ORDER BY floor
    "#;
    
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<FloorStats>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let rows = match stmt.query_map([], |row| {
        Ok(FloorStats {
            floor: row.get(0)?,
            total_seats: row.get(1)?,
            available_seats: row.get(2)?,
            occupied_seats: row.get(3)?,
        })
    }) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<FloorStats>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let stats: Vec<FloorStats> = rows.filter_map(|r| r.ok()).collect();
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("共 {} 层", stats.len()),
        data: Some(stats),
    })
}

/// 获取会议室列表（兼容旧版本）
pub async fn get_rooms(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();
    
    // 优先使用 room_type 和 room_name
    let has_room_type = conn.query_row(
        "SELECT 1 FROM seats WHERE room_type IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    if has_room_type {
        let sql = r#"
            SELECT DISTINCT
                room_name,
                floor,
                COUNT(*) as total,
                SUM(CASE WHEN status = 'available' THEN 1 ELSE 0 END) as available
            FROM seats
            WHERE room_name IS NOT NULL AND room_name != ''
            GROUP BY room_name, floor
            ORDER BY floor, room_name
        "#;
        
        let mut stmt = match conn.prepare(sql) {
            Ok(stmt) => stmt,
            Err(e) => {
                eprintln!("准备查询失败: {}", e);
                return HttpResponse::InternalServerError().json(ApiResponse::<Vec<RoomInfo>> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        };
        
        let rows = match stmt.query_map([], |row| {
            Ok(RoomInfo {
                room: row.get(0)?,
                floor: row.get(1)?,
                total_seats: row.get(2)?,
                available_seats: row.get(3)?,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("查询失败: {}", e);
                return HttpResponse::InternalServerError().json(ApiResponse::<Vec<RoomInfo>> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        };
        
        let rooms: Vec<RoomInfo> = rows.filter_map(|r| r.ok()).collect();
        
        return HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: format!("共 {} 个房间", rooms.len()),
            data: Some(rooms),
        });
    }
    
    // 兼容旧版本（room 字段）
    let has_room = conn.query_row(
        "SELECT 1 FROM seats WHERE room IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    if !has_room {
        return HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "暂无房间数据".to_string(),
            data: Some(Vec::<RoomInfo>::new()),
        });
    }
    
    let sql = r#"
        SELECT 
            room,
            floor,
            COUNT(*) as total,
            SUM(CASE WHEN status = 'available' THEN 1 ELSE 0 END) as available
        FROM seats
        WHERE room IS NOT NULL AND room != ''
        GROUP BY room, floor
        ORDER BY floor, room
    "#;
    
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<RoomInfo>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let rows = match stmt.query_map([], |row| {
        Ok(RoomInfo {
            room: row.get(0)?,
            floor: row.get(1)?,
            total_seats: row.get(2)?,
            available_seats: row.get(3)?,
        })
    }) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<RoomInfo>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let rooms: Vec<RoomInfo> = rows.filter_map(|r| r.ok()).collect();
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("共 {} 个房间", rooms.len()),
        data: Some(rooms),
    })
}

/// 获取房间统计（大厅、大中小自习室）
pub async fn get_room_stats(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();
    
    let has_room_type = conn.query_row(
        "SELECT 1 FROM seats WHERE room_type IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    if !has_room_type {
        return HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "暂无房间类型数据".to_string(),
            data: Some(Vec::<RoomStats>::new()),
        });
    }
    
    let sql = r#"
        SELECT 
            room_type,
            room_name,
            floor,
            COUNT(*) as total,
            SUM(CASE WHEN status = 'available' THEN 1 ELSE 0 END) as available,
            SUM(CASE WHEN status = 'occupied' THEN 1 ELSE 0 END) as occupied
        FROM seats
        GROUP BY room_type, room_name, floor
        ORDER BY floor, 
            CASE room_type
                WHEN 'hall' THEN 1
                WHEN 'large' THEN 2
                WHEN 'medium' THEN 3
                WHEN 'small' THEN 4
                ELSE 5
            END
    "#;
    
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<RoomStats>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let rows = match stmt.query_map([], |row| {
        Ok(RoomStats {
            room_type: row.get(0)?,
            room_name: row.get(1)?,
            floor: row.get(2)?,
            total_seats: row.get(3)?,
            available_seats: row.get(4)?,
            occupied_seats: row.get(5)?,
        })
    }) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<RoomStats>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let stats: Vec<RoomStats> = rows.filter_map(|r| r.ok()).collect();
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("共 {} 个房间类型", stats.len()),
        data: Some(stats),
    })
}

/// 管理员添加座位 - 使用扩展版本
pub async fn create_seat(
    req: web::Json<CreateSeatRequestExtended>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let now = Utc::now().to_rfc3339();
    let conn = db.lock().unwrap();
    
    let has_extended = conn.query_row(
        "SELECT 1 FROM seats WHERE room_type IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    if has_extended {
        let floor = req.floor.unwrap_or(1);
        let room_type = req.room_type.clone().unwrap_or_else(|| "hall".to_string());
        let room_name = req.room_name.clone().unwrap_or_else(|| "大厅".to_string());
        let is_near_socket = req.is_near_socket.unwrap_or(false);
        let is_near_window = req.is_near_window.unwrap_or(false);
        let is_quiet_zone = req.is_quiet_zone.unwrap_or(false);
        let seat_type = req.seat_type.clone().unwrap_or_else(|| "standard".to_string());
        
        match conn.execute(
            "INSERT INTO seats (seat_number, area, floor, room_type, room_name, 
             is_near_socket, is_near_window, is_quiet_zone, seat_type, 
             status, x_coord, y_coord, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'available', ?10, ?11, ?12, ?12)",
            params![
                &req.seat_number, &req.area, &floor, 
                &room_type, &room_name,
                &(is_near_socket as i32), &(is_near_window as i32), 
                &(is_quiet_zone as i32), &seat_type,
                &req.x_coord, &req.y_coord, &now
            ],
        ) {
            Ok(_) => HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                message: "座位添加成功".to_string(),
                data: None,
            }),
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    HttpResponse::BadRequest().json(ApiResponse::<()> {
                        success: false,
                        message: "座位号已存在".to_string(),
                        data: None,
                    })
                } else {
                    eprintln!("添加座位失败: {}", e);
                    HttpResponse::InternalServerError().json(ApiResponse::<()> {
                        success: false,
                        message: "添加座位失败".to_string(),
                        data: None,
                    })
                }
            }
        }
    } else {
        // 使用基础字段（兼容旧版本）
        match conn.execute(
            "INSERT INTO seats (seat_number, area, status, x_coord, y_coord, created_at, updated_at) 
             VALUES (?1, ?2, 'available', ?3, ?4, ?5, ?5)",
            params![&req.seat_number, &req.area, &req.x_coord, &req.y_coord, &now],
        ) {
            Ok(_) => HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                message: "座位添加成功".to_string(),
                data: None,
            }),
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    HttpResponse::BadRequest().json(ApiResponse::<()> {
                        success: false,
                        message: "座位号已存在".to_string(),
                        data: None,
                    })
                } else {
                    eprintln!("添加座位失败: {}", e);
                    HttpResponse::InternalServerError().json(ApiResponse::<()> {
                        success: false,
                        message: "添加座位失败".to_string(),
                        data: None,
                    })
                }
            }
        }
    }
}

/// 管理员修改座位信息 - 使用扩展版本
pub async fn update_seat(
    seat_id: web::Path<i32>,
    req: web::Json<UpdateSeatRequestExtended>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let seat_id = seat_id.into_inner();
    let now = Utc::now().to_rfc3339();
    let conn = db.lock().unwrap();
    
    let has_extended = conn.query_row(
        "SELECT 1 FROM seats WHERE room_type IS NOT NULL LIMIT 1",
        [],
        |_| Ok(()),
    ).is_ok();
    
    // 基础字段更新
    if let Some(seat_number) = &req.seat_number {
        let _ = conn.execute(
            "UPDATE seats SET seat_number = ?1, updated_at = ?2 WHERE id = ?3",
            params![seat_number, &now, seat_id],
        );
    }
    
    if let Some(area) = &req.area {
        let _ = conn.execute(
            "UPDATE seats SET area = ?1, updated_at = ?2 WHERE id = ?3",
            params![area, &now, seat_id],
        );
    }
    
    if let Some(status) = &req.status {
        let _ = conn.execute(
            "UPDATE seats SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, &now, seat_id],
        );
    }
    
    if let Some(x_coord) = &req.x_coord {
        let _ = conn.execute(
            "UPDATE seats SET x_coord = ?1, updated_at = ?2 WHERE id = ?3",
            params![x_coord, &now, seat_id],
        );
    }
    
    if let Some(y_coord) = &req.y_coord {
        let _ = conn.execute(
            "UPDATE seats SET y_coord = ?1, updated_at = ?2 WHERE id = ?3",
            params![y_coord, &now, seat_id],
        );
    }
    
    // 扩展字段更新
    if has_extended {
        if let Some(floor) = &req.floor {
            let _ = conn.execute(
                "UPDATE seats SET floor = ?1, updated_at = ?2 WHERE id = ?3",
                params![floor, &now, seat_id],
            );
        }
        
        if let Some(room_type) = &req.room_type {
            let _ = conn.execute(
                "UPDATE seats SET room_type = ?1, updated_at = ?2 WHERE id = ?3",
                params![room_type, &now, seat_id],
            );
        }
        
        if let Some(room_name) = &req.room_name {
            let _ = conn.execute(
                "UPDATE seats SET room_name = ?1, updated_at = ?2 WHERE id = ?3",
                params![room_name, &now, seat_id],
            );
        }
        
        if let Some(is_near_socket) = &req.is_near_socket {
            let _ = conn.execute(
                "UPDATE seats SET is_near_socket = ?1, updated_at = ?2 WHERE id = ?3",
                params![&(*is_near_socket as i32), &now, seat_id],
            );
        }
        
        if let Some(is_near_window) = &req.is_near_window {
            let _ = conn.execute(
                "UPDATE seats SET is_near_window = ?1, updated_at = ?2 WHERE id = ?3",
                params![&(*is_near_window as i32), &now, seat_id],
            );
        }
        
        if let Some(is_quiet_zone) = &req.is_quiet_zone {
            let _ = conn.execute(
                "UPDATE seats SET is_quiet_zone = ?1, updated_at = ?2 WHERE id = ?3",
                params![&(*is_quiet_zone as i32), &now, seat_id],
            );
        }
        
        if let Some(seat_type) = &req.seat_type {
            let _ = conn.execute(
                "UPDATE seats SET seat_type = ?1, updated_at = ?2 WHERE id = ?3",
                params![seat_type, &now, seat_id],
            );
        }
    }
    
    // 检查座位是否存在
    let exists: bool = conn.query_row(
        "SELECT 1 FROM seats WHERE id = ?1",
        params![seat_id],
        |_| Ok(()),
    ).is_ok();
    
    if exists {
        HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "座位更新成功".to_string(),
            data: None,
        })
    } else {
        HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: "座位不存在".to_string(),
            data: None,
        })
    }
}

/// 管理员删除座位
pub async fn delete_seat(
    seat_id: web::Path<i32>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let seat_id = seat_id.into_inner();
    let conn = db.lock().unwrap();
    
    // 检查是否有正在进行的预约
    let has_active_reservation: bool = match conn.query_row(
        "SELECT 1 FROM reservations WHERE seat_id = ?1 AND status IN ('pending', 'active') LIMIT 1",
        params![seat_id],
        |_| Ok(true),
    ) {
        Ok(_) => true,
        Err(_) => false,
    };
    
    if has_active_reservation {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "该座位有正在进行的预约，无法删除".to_string(),
            data: None,
        });
    }
    
    match conn.execute("DELETE FROM seats WHERE id = ?1", params![seat_id]) {
        Ok(affected) if affected > 0 => HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "座位删除成功".to_string(),
            data: None,
        }),
        Ok(_) => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: "座位不存在".to_string(),
            data: None,
        }),
        Err(e) => {
            eprintln!("删除座位失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "删除座位失败".to_string(),
                data: None,
            })
        }
    }
}