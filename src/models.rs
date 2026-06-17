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

// ========== 座位相关结构体（基础版） ==========
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Seat {
    pub id: i32,
    pub seat_number: String,
    pub area: String,
    pub status: String, // available, occupied, reserved, disabled
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSeatRequest {
    pub seat_number: String,
    pub area: String,
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSeatRequest {
    pub seat_number: Option<String>,
    pub area: Option<String>,
    pub status: Option<String>,
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GetSeatsQuery {
    pub area: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AvailableSeatsQuery {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

// ========== 房间类型定义 ==========
/// 房间类型枚举
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum RoomType {
    Hall,   // 大厅
    Large,  // 大自习室
    Medium, // 中自习室
    Small,  // 小自习室
}

impl RoomType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoomType::Hall => "hall",
            RoomType::Large => "large",
            RoomType::Medium => "medium",
            RoomType::Small => "small",
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            RoomType::Hall => "大厅",
            RoomType::Large => "大自习室",
            RoomType::Medium => "中自习室",
            RoomType::Small => "小自习室",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hall" => Some(RoomType::Hall),
            "large" => Some(RoomType::Large),
            "medium" => Some(RoomType::Medium),
            "small" => Some(RoomType::Small),
            _ => None,
        }
    }
}

// ========== 扩展座位结构体（包含房间信息） ==========
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeatInfo {
    pub id: i32,
    pub seat_number: String,
    pub area: String,
    pub floor: i32,
    pub room_type: String,      // hall, large, medium, small
    pub room_name: String,      // 大厅, 大自习室, 中自习室, 小自习室
    pub is_near_socket: bool,
    pub is_near_window: bool,
    pub is_quiet_zone: bool,
    pub seat_type: String,
    pub status: String,
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

// ========== 扩展请求结构体 ==========
#[derive(Debug, Deserialize)]
pub struct CreateSeatRequestExtended {
    pub seat_number: String,
    pub area: String,
    pub floor: Option<i32>,
    pub room_type: Option<String>,  // hall, large, medium, small
    pub room_name: Option<String>,  // 大厅, 大自习室, 中自习室, 小自习室
    pub is_near_socket: Option<bool>,
    pub is_near_window: Option<bool>,
    pub is_quiet_zone: Option<bool>,
    pub seat_type: Option<String>,
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSeatRequestExtended {
    pub seat_number: Option<String>,
    pub area: Option<String>,
    pub floor: Option<i32>,
    pub room_type: Option<String>,
    pub room_name: Option<String>,
    pub is_near_socket: Option<bool>,
    pub is_near_window: Option<bool>,
    pub is_quiet_zone: Option<bool>,
    pub seat_type: Option<String>,
    pub status: Option<String>,
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
}

// ========== 查询参数结构体 ==========
#[derive(Debug, Deserialize)]
pub struct SeatQueryParams {
    pub area: Option<String>,
    pub floor: Option<i32>,
    pub room_type: Option<String>,   // hall, large, medium, small
    pub room_name: Option<String>,   // 大厅, 大自习室, 中自习室, 小自习室
    pub is_near_socket: Option<bool>,
    pub is_near_window: Option<bool>,
    pub is_quiet_zone: Option<bool>,
    pub seat_type: Option<String>,
    pub status: Option<String>,
}

// ========== 统计结构体 ==========
/// 楼层统计
#[derive(Debug, Serialize)]
pub struct FloorStats {
    pub floor: i32,
    pub total_seats: i32,
    pub available_seats: i32,
    pub occupied_seats: i32,
}

/// 会议室信息（保留兼容）
#[derive(Debug, Serialize)]
pub struct RoomInfo {
    pub room: String,
    pub floor: i32,
    pub total_seats: i32,
    pub available_seats: i32,
}

/// 房间统计（大厅、大中小自习室）
#[derive(Debug, Serialize)]
pub struct RoomStats {
    pub room_type: String,      // hall, large, medium, small
    pub room_name: String,      // 大厅, 大自习室, 中自习室, 小自习室
    pub floor: i32,
    pub total_seats: i32,
    pub available_seats: i32,
    pub occupied_seats: i32,
}

// ========== 预约相关结构体 ==========
#[derive(Debug, Serialize, Deserialize)]
pub struct Reservation {
    pub id: i32,
    pub user_id: i32,
    pub seat_id: i32,
    pub seat_number: String,  // 关联查询得到
    pub area: String,          // 关联查询得到
    pub start_time: String,
    pub end_time: String,
    pub status: String, // pending, active, completed, cancelled, expired
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReservationRequest {
    pub seat_id: i32,
    pub start_time: String,  // ISO 8601 格式，如 "2026-06-20T14:00:00+08:00"
    pub end_time: String,
}

// 添加预约时间检查响应
#[derive(Debug, Serialize)]
pub struct ReservationTimeCheck {
    pub is_valid: bool,
    pub message: String,
    pub max_allowed_date: String,  // 最大允许日期
    pub min_allowed_date: String,  // 最小允许日期（当天）
}

#[derive(Debug, Deserialize)]
pub struct ExtendReservationRequest {
    pub extra_hours: i32, // 延长小时数
}

// ========== 签到签退相关结构体 ==========
#[derive(Debug, Deserialize)]
pub struct CheckinRequest {
    pub reservation_id: i32,
    pub qr_code: Option<String>,      // 二维码扫码
    pub location: Option<String>,      // 签到位置
    pub lat: Option<f64>,              // 纬度
    pub lng: Option<f64>,              // 经度
}

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub reservation_id: i32,
}

#[derive(Debug, Serialize)]
pub struct AttendanceStatus {
    pub is_checked_in: bool,
    pub reservation_id: Option<i32>,
    pub seat_id: Option<i32>,
    pub seat_number: Option<String>,
    pub checkin_time: Option<String>,
    pub checkout_time: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckinResponse {
    pub reservation_id: i32,
    pub checkin_time: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub reservation_id: i32,
    pub checkout_time: String,
    pub duration_minutes: i64,
    pub message: String,
}

// ========== 暂离相关结构体（如果使用） ==========
#[derive(Debug, Deserialize)]
pub struct TemporaryLeaveRequest {
    pub reservation_id: i32,
    pub leave_duration: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TemporaryLeaveResponse {
    pub reservation_id: i32,
    pub leave_start_time: String,
    pub leave_end_time: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct LeaveStatus {
    pub is_on_leave: bool,
    pub reservation_id: Option<i32>,
    pub seat_id: Option<i32>,
    pub seat_number: Option<String>,
    pub leave_start_time: Option<String>,
    pub leave_end_time: Option<String>,
    pub remaining_minutes: Option<i64>,
}