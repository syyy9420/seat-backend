use actix_web::{web, HttpResponse, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use rusqlite::Connection;
use std::sync::Mutex;

use crate::models::{
    ApiResponse, ChangePasswordRequest, Claims, LoginRequest, LoginResponse, RegisterRequest,
    UpdateUserRequest, UserInfo,
};

// JWT密钥
const JWT_SECRET: &[u8] = b"library-seat-reservation-secret-key-2024";

/// 生成JWT Token
fn generate_token(student_id: &str, user_id: i32, role: &str) -> String {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: student_id.to_string(),
        user_id,
        role: role.to_string(),
        exp: expiration,
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET)).unwrap()
}

/// 1. 用户注册（学生/教师注册账号）
pub async fn register(
    req: web::Json<RegisterRequest>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    // 校验输入
    if req.student_id.is_empty() || req.username.is_empty() || req.password.is_empty() || req.email.is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "学号、姓名、密码、邮箱不能为空".to_string(),
            data: None,
        });
    }

    if req.student_id.len() < 5 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "学号长度至少5个字符".to_string(),
            data: None,
        });
    }

    if req.password.len() < 6 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "密码长度至少6个字符".to_string(),
            data: None,
        });
    }

    // 加密密码
    let hashed = match hash(&req.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("密码加密失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    let created_at = Utc::now().to_rfc3339();
    let phone = req.phone.clone().unwrap_or_default();

    // 存入数据库
    let conn = db.lock().unwrap();
    match conn.execute(
        "INSERT INTO users (student_id, username, password_hash, email, phone, role, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, 'student', ?6)",
        [&req.student_id, &req.username, &hashed, &req.email, &phone, &created_at],
    ) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "注册成功，请登录".to_string(),
            data: None,
        }),
        Err(e) => {
            if e.to_string().contains("UNIQUE") {
                HttpResponse::BadRequest().json(ApiResponse::<()> {
                    success: false,
                    message: "该学号已注册".to_string(),
                    data: None,
                })
            } else {
                eprintln!("数据库错误: {}", e);
                HttpResponse::InternalServerError().json(ApiResponse::<()> {
                    success: false,
                    message: "注册失败，请稍后重试".to_string(),
                    data: None,
                })
            }
        }
    }
}

/// 2. 用户登录
pub async fn login(
    req: web::Json<LoginRequest>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    if req.student_id.is_empty() || req.password.is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<LoginResponse> {
            success: false,
            message: "学号和密码不能为空".to_string(),
            data: None,
        });
    }

    let conn = db.lock().unwrap();

    let mut stmt = match conn.prepare(
        "SELECT id, student_id, username, password_hash, email, phone, role FROM users WHERE student_id = ?1"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<LoginResponse> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    let mut rows = match stmt.query([&req.student_id]) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<LoginResponse> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    if let Some(row) = rows.next().unwrap() {
        let id: i32 = row.get(0).unwrap();
        let student_id: String = row.get(1).unwrap();
        let username: String = row.get(2).unwrap();
        let stored_hash: String = row.get(3).unwrap();
        let email: String = row.get(4).unwrap();
        let phone: Option<String> = row.get(5).unwrap();
        let role: String = row.get(6).unwrap();

        match verify(&req.password, &stored_hash) {
            Ok(true) => {
                let token = generate_token(&student_id, id, &role);
                HttpResponse::Ok().json(ApiResponse {
                    success: true,
                    message: "登录成功".to_string(),
                    data: Some(LoginResponse {
                        token,
                        user: UserInfo {
                            id,
                            student_id,
                            username,
                            email,
                            phone,
                            role,
                        },
                    }),
                })
            }
            Ok(false) => HttpResponse::Unauthorized().json(ApiResponse::<LoginResponse> {
                success: false,
                message: "密码错误".to_string(),
                data: None,
            }),
            Err(e) => {
                eprintln!("密码验证失败: {}", e);
                HttpResponse::InternalServerError().json(ApiResponse::<LoginResponse> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                })
            }
        }
    } else {
        HttpResponse::NotFound().json(ApiResponse::<LoginResponse> {
            success: false,
            message: "该学号未注册".to_string(),
            data: None,
        })
    }
}

/// 3. 修改密码
pub async fn change_password(
    req: web::Json<ChangePasswordRequest>,
    db: web::Data<Mutex<Connection>>,
    student_id: web::Path<String>,
) -> impl Responder {
    let student_id = student_id.into_inner();
    let conn = db.lock().unwrap();

    let mut stmt = match conn.prepare("SELECT id, password_hash FROM users WHERE student_id = ?1") {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    let mut rows = match stmt.query([&student_id]) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    if let Some(row) = rows.next().unwrap() {
        let stored_hash: String = row.get(1).unwrap();

        match verify(&req.old_password, &stored_hash) {
            Ok(true) => {
                let new_hash = match hash(&req.new_password, DEFAULT_COST) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("密码加密失败: {}", e);
                        return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                            success: false,
                            message: "服务器内部错误".to_string(),
                            data: None,
                        });
                    }
                };

                match conn.execute(
                    "UPDATE users SET password_hash = ?1 WHERE student_id = ?2",
                    [&new_hash, &student_id],
                ) {
                    Ok(_) => HttpResponse::Ok().json(ApiResponse::<()> {
                        success: true,
                        message: "密码修改成功".to_string(),
                        data: None,
                    }),
                    Err(e) => {
                        eprintln!("更新密码失败: {}", e);
                        HttpResponse::InternalServerError().json(ApiResponse::<()> {
                            success: false,
                            message: "密码修改失败".to_string(),
                            data: None,
                        })
                    }
                }
            }
            Ok(false) => HttpResponse::Unauthorized().json(ApiResponse::<()> {
                success: false,
                message: "原密码错误".to_string(),
                data: None,
            }),
            Err(e) => {
                eprintln!("密码验证失败: {}", e);
                HttpResponse::InternalServerError().json(ApiResponse::<()> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                })
            }
        }
    } else {
        HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: "用户不存在".to_string(),
            data: None,
        })
    }
}

/// 4. 获取用户信息（通过学号）
pub async fn get_user_by_student_id(
    student_id: web::Path<String>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let student_id = student_id.into_inner();
    let conn = db.lock().unwrap();

    let mut stmt = match conn.prepare(
        "SELECT id, student_id, username, email, phone, role FROM users WHERE student_id = ?1"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<UserInfo> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    let mut rows = match stmt.query([&student_id]) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<UserInfo> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    if let Some(row) = rows.next().unwrap() {
        let user = UserInfo {
            id: row.get(0).unwrap(),
            student_id: row.get(1).unwrap(),
            username: row.get(2).unwrap(),
            email: row.get(3).unwrap(),
            phone: row.get(4).unwrap(),
            role: row.get(5).unwrap(),
        };
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "获取成功".to_string(),
            data: Some(user),
        })
    } else {
        HttpResponse::NotFound().json(ApiResponse::<UserInfo> {
            success: false,
            message: "用户不存在".to_string(),
            data: None,
        })
    }
}

/// 5. 更新用户信息（邮箱、电话）- 简化版
pub async fn update_user_info(
    student_id: web::Path<String>,
    req: web::Json<UpdateUserRequest>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let student_id = student_id.into_inner();
    let conn = db.lock().unwrap();

    // 先检查用户是否存在
    let exists: bool = match conn.query_row(
        "SELECT 1 FROM users WHERE student_id = ?1",
        [&student_id],
        |_| Ok(()),
    ) {
        Ok(_) => true,
        Err(_) => false,
    };

    if !exists {
        return HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: "用户不存在".to_string(),
            data: None,
        });
    }

    // 分别处理每个字段的更新
    if let Some(email) = &req.email {
        if let Err(e) = conn.execute(
            "UPDATE users SET email = ?1 WHERE student_id = ?2",
            [email, &student_id],
        ) {
            eprintln!("更新邮箱失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "更新失败".to_string(),
                data: None,
            });
        }
    }

    if let Some(phone) = &req.phone {
        if let Err(e) = conn.execute(
            "UPDATE users SET phone = ?1 WHERE student_id = ?2",
            [phone, &student_id],
        ) {
            eprintln!("更新电话失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "更新失败".to_string(),
                data: None,
            });
        }
    }

    HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "更新成功".to_string(),
        data: None,
    })
}

/// 6. 获取所有用户（管理员专用）
pub async fn get_all_users(db: web::Data<Mutex<Connection>>) -> impl Responder {
    let conn = db.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT id, student_id, username, email, phone, role FROM users") {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<UserInfo>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    let rows = match stmt.query_map([], |row| {
        Ok(UserInfo {
            id: row.get(0)?,
            student_id: row.get(1)?,
            username: row.get(2)?,
            email: row.get(3)?,
            phone: row.get(4)?,
            role: row.get(5)?,
        })
    }) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<UserInfo>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };

    let users: Vec<UserInfo> = rows.filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("共 {} 位用户", users.len()),
        data: Some(users),
    })
}