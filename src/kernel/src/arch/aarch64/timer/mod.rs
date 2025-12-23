pub fn init() {
    unsafe {
        let mut frq: u64 = 0;
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) frq);
        let ms = env!("HNX_TIMER_DEFAULT_MS").parse().unwrap_or(500u64);
        let ticks = frq / 1000 * ms;
        core::arch::asm!(
            "msr cntp_tval_el0, {t}\n msr cntp_ctl_el0, {c}",
            t = in(reg) ticks,
            c = in(reg) 1u64,
        );
    }
}

pub fn set_next_tick(ms: u64) {
    unsafe {
        let mut frq: u64 = 0;
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) frq);
        let ticks = frq / 1000 * ms;
        core::arch::asm!("msr cntp_tval_el0, {t}", t = in(reg) ticks);
    }
}

pub fn now_us() -> u64 {
    unsafe {
        let mut frq: u64 = 0;
        let mut cnt: u64 = 0;
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) frq);
        core::arch::asm!("mrs {c}, cntpct_el0", c = out(reg) cnt);
        if frq == 0 {
            return 0;
        }
        (cnt * 1_000_000u64) / frq
    }
}
