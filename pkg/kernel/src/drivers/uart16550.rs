use core::fmt;
use x86_64::instructions::port::Port;

/// A port-mapped UART 16550 serial interface.
pub struct SerialPort {
    data: Port<u8>,
    interrupt_enable: Port<u8>,
    interrupt_identification_fifo_control: Port<u8>,
    line_control: Port<u8>,
    line_status: Port<u8>,
    scratch: Port<u8>,
}

bitflags! {
    struct LineControl:u8{
        const DLAB = 0b10000000;
        const OneStopBit = 0b00000000;
        const NoneParity = 0b00000000;
        const EightCharLength = 0b00000011;
    }
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self {
            data: Port::new(port),
            interrupt_enable: Port::new(port + 1),
            interrupt_identification_fifo_control: Port::new(port + 2),
            line_control: Port::new(port + 3),
            line_status: Port::new(port + 5),
            scratch: Port::new(port + 7),
        }
    }

    /// Initializes the serial port.
    pub fn init(&mut self) {
        unsafe {
            self.interrupt_enable.write(0x00); // Disable all interrupts
            self.line_control.write(LineControl::DLAB.bits()); // Enable DLAB (set baud rate divisor)
            self.data.write(0x03); // Set divisor to 3 (lo byte) 38400 baud
            self.interrupt_enable.write(0x00); // Set divisor to 3 (hi byte) 38400 baud
            self.line_control.write(
                (LineControl::EightCharLength | LineControl::NoneParity | LineControl::OneStopBit)
                    .bits(),
            ); // 8 bits, no parity, one stop bit
            self.interrupt_identification_fifo_control.write(0xC7); // Enable FIFO, clear them, with 14-byte threshold
            self.scratch.write(0xAE);
            self.interrupt_enable.write(0x01);
        }
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, data: u8) {
        unsafe {
            while self.line_status.read() & 0x20 == 0 {}
            self.data.write(data);
        }
    }

    /// Receives a byte on the serial port no wait.
    pub fn receive(&mut self) -> Option<u8> {
        unsafe {
            if self.line_status.read() & 1 != 0 {
                Some(self.data.read())
            } else {
                None
            }
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}
