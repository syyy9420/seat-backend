mod db;
mod models;
mod user_handler;

use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use std::sync::Mutex;
use actix_cors::Cors;
use rusqlite::Connection;

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("✅ 图书馆座位预约系统 - 用户服务运行正常")
}

async fn ping() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "user-backend",
        "version": "0.1.0"
    }))
}
mod seat_handler;
mod reservation_handler;
mod attendance_handler;
mod reservation_validator;


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
    get_available_dates_api,
    get_available_time_slots_api,
    check_reservation_time,
    get_daily_reservation_status,
};

use attendance_handler::{
    checkin,
    checkout,
    get_attendance_status,
};


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let conn = match db::init_db() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ 数据库初始化失败: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("数据库错误: {}", e),
            ));
        }
    };

    let db_state = web::Data::new(Mutex::new(conn));

    println!("═══════════════════════════════════════════════════════════════");
    println!("📚 图书馆座位预约系统 - 用户管理服务");
    println!("🚀 服务已启动");
    println!("📡 监听地址: http://127.0.0.1:8080");
    println!();
    println!("📋 可用接口:");
    println!("   GET  /health                    - 健康检查");
    println!("   GET  /api/ping                  - 测试接口");
    println!("   POST /api/register              - 用户注册（学号/姓名/密码/邮箱）");
    println!("   POST /api/login                 - 用户登录（学号/密码）");
    println!("   POST /api/change-pwd/{{studentId}} - 修改密码");
    println!("   GET  /api/user/{{studentId}}      - 获取用户信息");
    println!("   PUT  /api/user/{{studentId}}      - 更新用户信息");
    println!("   GET  /api/users                 - 获取所有用户（管理员）");
    println!("═══════════════════════════════════════════════════════════════");

    // 初始化数据库连接
    let db_path = "library.db";
    let conn = rusqlite::Connection::open(db_path).expect("Failed to open database");


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
            .route("/health", web::get().to(health_check))
            .route("/api/ping", web::get().to(ping))
            .route("/api/register", web::post().to(user_handler::register))
            .route("/api/login", web::post().to(user_handler::login))
            .route("/api/change-pwd/{student_id}", web::post().to(user_handler::change_password))
            .route("/api/user/{student_id}", web::get().to(user_handler::get_user_by_student_id))
            .route("/api/user/{student_id}", web::put().to(user_handler::update_user_info))
            .route("/api/users", web::get().to(user_handler::get_all_users))
            .route("/api/me", web::get().to(user_handler::get_current_user))
    
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
            // ========== 预约管理接口 ==========
            .route("/api/reservations/dates", web::get().to(get_available_dates_api))           // 获取可用日期
            .route("/api/reservations/timeslots", web::get().to(get_available_time_slots_api)) // 获取可用时间段
            .route("/api/reservations/check-time", web::get().to(check_reservation_time))      // 验证时间
            .route("/api/reservations", web::post().to(create_reservation))
            .route("/api/reservations", web::get().to(get_my_reservations))
            .route("/api/reservations/{id}", web::get().to(get_reservation_detail))
            .route("/api/reservations/{id}", web::delete().to(cancel_reservation))
            .route("/api/reservations/{id}/extend", web::put().to(extend_reservation))
            .route("/api/reservations/daily-status", web::get().to(get_daily_reservation_status)) // 获取每日预约状态
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

