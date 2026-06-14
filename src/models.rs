use serde::{Deserialize, Serialize};

// ========== 请求结构体 ==========

/// 注册请求（图书馆用户）
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub student_id: String,  // 学号/工号
    pub username: String,
    pub password: String,
    pub email: String,
    pub phone: Option<String>,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub student_id: String,
    pub password: String,
}

/// 修改密码请求
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 更新用户信息请求
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
}

// ========== 响应结构体 ==========

/// 统一响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

/// 登录成功后返回的用户信息
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// 用户信息（不含密码）
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i32,
    pub student_id: String,
    pub username: String,
    pub email: String,
    pub phone: Option<String>,
    pub role: String,
}

// ========== JWT Claims ==========

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // 学号
    pub user_id: i32,     // 用户ID
    pub role: String,     // 角色：admin / student
    pub exp: usize,       // 过期时间戳
}