pub trait UartDriver {
    fn init(&self);
    fn putc(&self, c: u8);
    fn getc(&self) -> Option<u8>;
}
