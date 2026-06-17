mod db;
mod models;
mod user_handler;
mod seat_handler;
mod reservation_handler;
mod attendance_handler;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use std::sync::Mutex;

// 导入所有 handler 函数
use seat_handler::{
    get_seats,
    get_seat_by_id,
    get_available_seats,
    create_seat,
    update_seat,
    delete_seat,
    get_floor_stats,
    get_rooms,
    get_room_stats,
};

use user_handler::{
    register,
    login,
    get_user_by_student_id,
    update_user_info,
    change_password,
    get_all_users,
};

use reservation_handler::{
    create_reservation,
    get_my_reservations,
    get_reservation_detail,
    cancel_reservation,
    extend_reservation,
};

use attendance_handler::{
    checkin,
    checkout,
    get_attendance_status,
};

// 健康检查接口
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("Server is running")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // 初始化数据库连接
    let db_path = "library.db";
    let conn = rusqlite::Connection::open(db_path).expect("Failed to open database");

    // 初始化数据库表
    init_database(&conn);

    let db_state = web::Data::new(Mutex::new(conn));

    println!("Server starting at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
            )
            .app_data(db_state.clone())
            // 健康检查
            .route("/health", web::get().to(health_check))
            // ========== 座位管理接口 ==========
            .route("/api/seats/available", web::get().to(get_available_seats))
            .route("/api/seats", web::get().to(get_seats))
            .route("/api/seats/{id}", web::get().to(get_seat_by_id))
            .route("/api/seats", web::post().to(create_seat))
            .route("/api/seats/{id}", web::put().to(update_seat))
            .route("/api/seats/{id}", web::delete().to(delete_seat))
            .route("/api/seats/floors", web::get().to(get_floor_stats))
            .route("/api/seats/rooms", web::get().to(get_rooms))
            .route("/api/seats/rooms/stats", web::get().to(get_room_stats))  // 新增房间统计
            // ========== 用户管理接口 ==========
            .route("/api/auth/register", web::post().to(register))
            .route("/api/auth/login", web::post().to(login))
            .route("/api/users/{student_id}", web::get().to(get_user_by_student_id))
            .route("/api/users/{student_id}", web::put().to(update_user_info))
            .route("/api/users/password/{student_id}", web::put().to(change_password))
            .route("/api/users", web::get().to(get_all_users))
            // ========== 预约管理接口 ==========
            .route("/api/reservations", web::post().to(create_reservation))
            .route("/api/reservations/{id}", web::delete().to(cancel_reservation))
            .route("/api/reservations", web::get().to(get_my_reservations))
            .route("/api/reservations/{id}", web::get().to(get_reservation_detail))
            .route("/api/reservations/{id}/extend", web::put().to(extend_reservation))
            // ========== 签到签退接口 ==========
            .route("/api/checkin", web::post().to(checkin))
            .route("/api/checkout", web::post().to(checkout))
            .route("/api/attendance/status", web::get().to(get_attendance_status))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

// 初始化数据库表
fn init_database(conn: &rusqlite::Connection) {
    // 创建 users 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            student_id TEXT UNIQUE NOT NULL,
            username TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            email TEXT NOT NULL,
            phone TEXT,
            role TEXT DEFAULT 'student',
            created_at TEXT NOT NULL
        )",
        [],
    ).expect("Failed to create users table");

    // 删除旧表并创建新 seats 表
    conn.execute(
        "DROP TABLE IF EXISTS seats",
        [],
    ).ok();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS seats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            seat_number TEXT NOT NULL UNIQUE,
            area TEXT NOT NULL,
            floor INTEGER DEFAULT 1,
            room_type TEXT DEFAULT 'hall',      -- hall, large, medium, small
            room_name TEXT,                     -- 大厅, 大自习室, 中自习室, 小自习室
            is_near_socket BOOLEAN DEFAULT 0,
            is_near_window BOOLEAN DEFAULT 0,
            is_quiet_zone BOOLEAN DEFAULT 0,
            seat_type TEXT DEFAULT 'standard',
            status TEXT NOT NULL DEFAULT 'available',
            x_coord INTEGER,
            y_coord INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    ).expect("Failed to create seats table");
    
    // 创建 reservations 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reservations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            seat_id INTEGER NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (seat_id) REFERENCES seats(id)
        )",
        [],
    ).expect("Failed to create reservations table");
    
    // 创建 attendance 签到记录表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS attendance (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            reservation_id INTEGER NOT NULL,
            seat_id INTEGER NOT NULL,
            checkin_time TEXT NOT NULL,
            checkout_time TEXT,
            location TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (reservation_id) REFERENCES reservations(id),
            FOREIGN KEY (seat_id) REFERENCES seats(id)
        )",
        [],
    ).expect("Failed to create attendance table");

    // 插入管理员账号
    let admin_hash = "$2b$12$2Yq4uQkE5ZvXxV5Yq4uQkE";
    let now = chrono::Utc::now().to_rfc3339();
    let _ = conn.execute(
        "INSERT OR IGNORE INTO users (student_id, username, password_hash, email, phone, role, created_at) 
         VALUES ('admin001', '管理员', ?1, 'admin@library.com', '13800000000', 'admin', ?2)",
        [admin_hash, &now],
    );
    
    // ========== 插入测试座位数据 ==========
    let mut seat_counter = 0;
    
    // 定义楼层和房间配置
    // (floor, room_type, room_name, count)
    let configs = vec![
        (1, "hall", "大厅", 30),
        (2, "hall", "大厅", 50),
        (2, "large", "大自习室", 3),
        (2, "medium", "中自习室", 4),
        (2, "small", "小自习室", 5),
        (3, "hall", "大厅", 50),
        (3, "large", "大自习室", 3),
        (3, "medium", "中自习室", 4),
        (3, "small", "小自习室", 5),
    ];

    for (floor, room_type, room_name, count) in configs {
        // 座位编号前缀
        let prefix = match room_type {
            "hall" => "H",
            "large" => "L",
            "medium" => "M",
            "small" => "S",
            _ => "H",
        };
        
        for i in 1..=count {
            seat_counter += 1;
            let seat_number = format!("{}{:03}", prefix, seat_counter);
            let area = format!("{}楼{}", floor, room_name);
            
            // 设置座位属性
            let near_socket = i % 3 == 0;      // 每3个有一个靠近插座
            let near_window = i % 5 == 0;      // 每5个有一个靠窗
            let quiet_zone = room_type == "large" || room_type == "medium";  // 大中自习室是静音区
            
            let _ = conn.execute(
                "INSERT INTO seats (seat_number, area, floor, room_type, room_name, 
                 is_near_socket, is_near_window, is_quiet_zone, seat_type, 
                 x_coord, y_coord, created_at, updated_at) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
                [
                    &seat_number,
                    &area,
                    &floor.to_string(),
                    room_type,
                    room_name,
                    &(near_socket as i32).to_string(),
                    &(near_window as i32).to_string(),
                    &(quiet_zone as i32).to_string(),
                    "standard",
                    &((100 + (i - 1) * 50) % 500).to_string(),
                    &((100 + ((i - 1) / 10) * 50) % 500).to_string(),
                    &now,
                ],
            );
        }
    }

    println!("✅ 数据库初始化成功，共创建 {} 个座位", seat_counter);
    println!("📊 座位分布:");
    println!("  1楼大厅: 30个");
    println!("  2楼大厅: 50个, 大自习室: 3个, 中自习室: 4个, 小自习室: 5个");
    println!("  3楼大厅: 50个, 大自习室: 3个, 中自习室: 4个, 小自习室: 5个");
    println!("  总计: 154个座位");
}