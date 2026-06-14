mod db;
mod models;
mod user_handler;

use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use std::sync::Mutex;
// use rusqlite::Connection;

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

    HttpServer::new(move || {
        App::new()
            .app_data(db_state.clone())
            .route("/health", web::get().to(health_check))
            .route("/api/ping", web::get().to(ping))
            .route("/api/register", web::post().to(user_handler::register))
            .route("/api/login", web::post().to(user_handler::login))
            .route("/api/change-pwd/{student_id}", web::post().to(user_handler::change_password))
            .route("/api/user/{student_id}", web::get().to(user_handler::get_user_by_student_id))
            .route("/api/user/{student_id}", web::put().to(user_handler::update_user_info))
            .route("/api/users", web::get().to(user_handler::get_all_users))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}