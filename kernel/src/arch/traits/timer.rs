// 定时器抽象特征
//
// 要求实现：
// - 系统定时器
// - 高精度定时器
// - 周期性定时器
// - 单次定时器
// - 时钟源管理
// - 时间戳计数器
// - 定时器中断

use super::super::ArchResult;

/// 定时器架构抽象特征
pub trait TimerArch: Send + Sync {
    /// 定时器对象类型
    type Timer: HardwareTimer;

    /// 时钟源对象类型
    type ClockSource: ClockSource;

    /// 初始化定时器系统
    fn init() -> ArchResult<()>;

    /// 获取系统时间（纳秒）
    fn system_time() -> u64;

    /// 获取单调时间（纳秒）
    fn monotonic_time() -> u64;

    /// 获取启动时间（纳秒）
    fn boot_time() -> u64;

    /// 获取高精度定时器
    fn high_precision_timer() -> &'static Self::Timer;

    /// 获取系统定时器
    fn system_timer() -> &'static Self::Timer;

    /// 注册定时器回调
    fn register_callback(callback: TimerCallback, data: usize) -> ArchResult<TimerHandle>;

    /// 注销定时器回调
    fn unregister_callback(handle: TimerHandle) -> ArchResult<()>;

    /// 设置单次定时器
    fn set_oneshot(
        timer: &Self::Timer,
        nanoseconds: u64,
        callback: Option<TimerCallback>,
        data: usize,
    ) -> ArchResult<()>;

    /// 设置周期性定时器
    fn set_periodic(
        timer: &Self::Timer,
        period_ns: u64,
        callback: Option<TimerCallback>,
        data: usize,
    ) -> ArchResult<()>;

    /// 取消定时器
    fn cancel(timer: &Self::Timer) -> ArchResult<()>;

    /// 获取定时器频率（Hz）
    fn timer_frequency(timer: &Self::Timer) -> u64;

    /// 获取时钟源
    fn clock_source() -> &'static Self::ClockSource;

    /// 校准时间
    fn calibrate() -> ArchResult<()>;
}

/// 硬件定时器特征
pub trait HardwareTimer: Send + Sync {
    /// 定时器名称
    fn name(&self) -> &str;

    /// 初始化定时器
    fn initialize(&self) -> ArchResult<()>;

    /// 启用定时器
    fn enable(&self);

    /// 禁用定时器
    fn disable(&self);

    /// 设置计数值
    fn set_count(&self, count: u64);

    /// 获取当前计数值
    fn current_count(&self) -> u64;

    /// 设置比较值
    fn set_compare(&self, compare: u64);

    /// 获取比较值
    fn compare_value(&self) -> u64;

    /// 设置周期
    fn set_period(&self, period_ns: u64) -> ArchResult<()>;

    /// 是否支持单次模式
    fn supports_oneshot(&self) -> bool;

    /// 是否支持周期性模式
    fn supports_periodic(&self) -> bool;

    /// 最大计数值
    fn max_count(&self) -> u64;

    /// 最小周期（纳秒）
    fn min_period_ns(&self) -> u64;

    /// 最大周期（纳秒）
    fn max_period_ns(&self) -> u64;
}

/// 时钟源特征
pub trait ClockSource: Send + Sync {
    /// 时钟源名称
    fn name(&self) -> &str;

    /// 初始化时钟源
    fn initialize(&self) -> ArchResult<()>;

    /// 读取当前时间
    fn read(&self) -> u64;

    /// 频率（Hz）
    fn frequency(&self) -> u64;

    /// 精度（纳秒）
    fn resolution_ns(&self) -> u64;

    /// 是否单调递增
    fn is_monotonic(&self) -> bool;

    /// 是否连续（不会回绕）
    fn is_continuous(&self) -> bool;

    /// 回绕周期
    fn wrap_around_period(&self) -> u64;

    /// 时钟源类型
    fn clock_type(&self) -> ClockType;
}

/// 定时器回调类型
pub type TimerCallback = fn(data: usize) -> bool;

/// 定时器句柄
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerHandle(u64);

impl TimerHandle {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(&self) -> u64 {
        self.0
    }
}

/// 时钟类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockType {
    SystemClock,
    MonotonicClock,
    RealTimeClock,
    PerformanceCounter,
    ArchitecturalTimer,
    VirtualTimer,
}
