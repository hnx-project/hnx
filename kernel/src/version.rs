//! 内核版本信息 - 自动生成，请勿手动修改

/// 主版本号
pub const MAJOR: u32 = 0;

/// 次版本号
pub const MINOR: u32 = 3;

/// 修订版本号
pub const PATCH: u32 = 0;

/// 预发布标签
pub const PRERELEASE: &str = "alpha.1";

/// 完整版本字符串
pub const VERSION_STRING: &str = "0.3.0-alpha.1+20260105.b0792d2";

/// 获取版本字符串
#[no_mangle]
pub extern "C" fn hnx_get_version() -> &'static str {
    VERSION_STRING
}

/// 获取主版本号
#[no_mangle]
pub extern "C" fn hnx_get_version_major() -> u32 {
    MAJOR
}

/// 获取次版本号
#[no_mangle]
pub extern "C" fn hnx_get_version_minor() -> u32 {
    MINOR
}

/// 获取修订版本号
#[no_mangle]
pub extern "C" fn hnx_get_version_patch() -> u32 {
    PATCH
}
