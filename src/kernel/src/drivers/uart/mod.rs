pub mod r#trait;
pub mod pl011;

pub fn default() -> &'static dyn r#trait::UartDriver {
    match env!("HNX_UART_DEFAULT") {
        "pl011" => &pl011::PL011,
        _ => &pl011::PL011
    }
}
