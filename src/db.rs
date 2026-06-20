use rusqlite::{Connection, Result};

/// 初始化数据库，创建users表（图书馆座位预约系统用户表）
pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("library.db")?;
    
    // 创建用户表
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
    )?;
    
    // 可选：插入一个管理员账号（方便测试）
    conn.execute(
        "INSERT OR IGNORE INTO users (student_id, username, password_hash, email, phone, role, created_at)
         VALUES ('admin001', '管理员', '$2a$12$1mwdd8i1DP3qjs5BkDkJs.IDq5cV2BmNWJIFiSYnp9h8hRQQdsD8e', 'admin@library.com', '13800000000', 'admin', '2024-01-01T00:00:00')",
        [],
    )?;
    
    println!("✅ 数据库初始化成功，文件: library.db");
    Ok(conn)
}