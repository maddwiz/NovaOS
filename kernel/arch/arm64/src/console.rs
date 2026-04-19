use crate::bootinfo::{FramebufferFormat, NovaBootInfoV1};

pub trait ConsoleSink {
    fn write_str(&mut self, s: &str);

    fn write_line(&mut self, s: &str) {
        self.write_str(s);
        self.write_str("\n");
    }

    fn log(&mut self, level: LogLevel, message: &str) {
        self.write_str("[");
        self.write_str(level.as_str());
        self.write_str("] ");
        self.write_line(message);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

pub struct NullConsole;

impl ConsoleSink for NullConsole {
    fn write_str(&mut self, _s: &str) {}
}

pub struct BootConsole {
    framebuffer: Option<FramebufferConsole>,
    trace: TraceConsole,
}

impl BootConsole {
    pub fn from_boot_info(boot_info: &NovaBootInfoV1) -> Self {
        Self {
            framebuffer: FramebufferConsole::from_boot_info(boot_info),
            trace: TraceConsole::new(),
        }
    }
}

impl ConsoleSink for BootConsole {
    fn write_str(&mut self, s: &str) {
        self.trace.write_str(s);

        if let Some(framebuffer) = self.framebuffer.as_mut() {
            framebuffer.write_str(s);
        }
    }
}

pub struct FramebufferConsole {
    base: *mut u32,
    width: usize,
    height: usize,
    stride: usize,
    cursor_x: usize,
    cursor_y: usize,
    foreground: u32,
    background: u32,
}

impl FramebufferConsole {
    const GLYPH_WIDTH: usize = 5;
    const GLYPH_HEIGHT: usize = 7;
    const GLYPH_ADVANCE_X: usize = 6;
    const GLYPH_ADVANCE_Y: usize = 8;
    const MARGIN_X: usize = 1;
    const MARGIN_Y: usize = 1;

    pub fn from_boot_info(boot_info: &NovaBootInfoV1) -> Option<Self> {
        Self::new(
            boot_info.framebuffer_base as *mut u32,
            boot_info.framebuffer_width,
            boot_info.framebuffer_height,
            boot_info.framebuffer_stride,
            boot_info.framebuffer_format,
        )
    }

    pub fn new(
        base: *mut u32,
        width: u32,
        height: u32,
        stride: u32,
        format: FramebufferFormat,
    ) -> Option<Self> {
        if base.is_null()
            || width == 0
            || height == 0
            || stride < width
            || matches!(format, FramebufferFormat::Unknown)
        {
            return None;
        }

        let mut console = Self {
            base,
            width: width as usize,
            height: height as usize,
            stride: stride as usize,
            cursor_x: Self::MARGIN_X,
            cursor_y: Self::MARGIN_Y,
            foreground: foreground_color(format),
            background: background_color(format),
        };
        console.clear();
        Some(console)
    }

    fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.write_pixel(x, y, self.background);
            }
        }

        self.cursor_x = Self::MARGIN_X;
        self.cursor_y = Self::MARGIN_Y;
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            b'\r' => self.cursor_x = Self::MARGIN_X,
            b'\t' => {
                for _ in 0..4 {
                    self.write_byte(b' ');
                }
            }
            _ => {
                if self.cursor_x + Self::GLYPH_WIDTH > self.width {
                    self.newline();
                }

                if self.cursor_y + Self::GLYPH_HEIGHT > self.height {
                    self.clear();
                }

                self.draw_glyph(glyph_rows(normalize_ascii(byte)));
                self.cursor_x += Self::GLYPH_ADVANCE_X;
            }
        }
    }

    fn newline(&mut self) {
        self.cursor_x = Self::MARGIN_X;
        self.cursor_y += Self::GLYPH_ADVANCE_Y;

        if self.cursor_y + Self::GLYPH_HEIGHT > self.height {
            self.clear();
        }
    }

    fn draw_glyph(&mut self, rows: [u8; Self::GLYPH_HEIGHT]) {
        for (row_index, row_bits) in rows.iter().enumerate() {
            for col_index in 0..Self::GLYPH_WIDTH {
                let mask = 1 << (Self::GLYPH_WIDTH - 1 - col_index);
                let color = if row_bits & mask != 0 {
                    self.foreground
                } else {
                    self.background
                };
                self.write_pixel(self.cursor_x + col_index, self.cursor_y + row_index, color);
            }
        }
    }

    fn write_pixel(&mut self, x: usize, y: usize, color: u32) {
        let index = y * self.stride + x;
        unsafe {
            core::ptr::write_volatile(self.base.add(index), color);
        }
    }
}

impl ConsoleSink for FramebufferConsole {
    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

pub struct ConsoleLogger<'a> {
    sink: &'a mut dyn ConsoleSink,
}

impl<'a> ConsoleLogger<'a> {
    pub fn new(sink: &'a mut dyn ConsoleSink) -> Self {
        Self { sink }
    }

    pub fn log(&mut self, level: LogLevel, message: &str) {
        self.sink.log(level, message);
    }
}

pub struct TraceConsole;

impl TraceConsole {
    pub const fn new() -> Self {
        Self
    }
}

impl ConsoleSink for TraceConsole {
    fn write_str(&mut self, s: &str) {
        trace_write(s.as_bytes());
    }
}

const fn foreground_color(_format: FramebufferFormat) -> u32 {
    0x00FF_FFFF
}

const fn background_color(_format: FramebufferFormat) -> u32 {
    0x0000_0000
}

const fn normalize_ascii(byte: u8) -> u8 {
    if byte >= b'a' && byte <= b'z' {
        byte - (b'a' - b'A')
    } else {
        byte
    }
}

fn glyph_rows(byte: u8) -> [u8; FramebufferConsole::GLYPH_HEIGHT] {
    match byte {
        b'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        b'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        b'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        b'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        b'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        b'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        b'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        b'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        b'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        b'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        b'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        b'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        b'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        b'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        b'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        b'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        b'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        b'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        b'0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        b'1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        b'3' => [0x1E, 0x01, 0x01, 0x06, 0x01, 0x01, 0x1E],
        b'4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        b'5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
        b'6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        b'7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        b'9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
        b'[' => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E],
        b']' => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E],
        b'-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        b';' => [0x00, 0x04, 0x00, 0x00, 0x04, 0x04, 0x08],
        b'?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        b' ' => [0x00; FramebufferConsole::GLYPH_HEIGHT],
        _ => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
fn trace_write(message: &[u8]) {
    qemu_uart_write(message);
}

#[cfg(not(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
)))]
fn trace_write(_message: &[u8]) {}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
fn qemu_uart_write(message: &[u8]) {
    const PL011_BASE: usize = 0x0900_0000;
    const PL011_DR: *mut u32 = PL011_BASE as *mut u32;
    const PL011_FR: *const u32 = (PL011_BASE + 0x18) as *const u32;
    const PL011_FR_TXFF: u32 = 1 << 5;

    for &byte in message {
        while unsafe { core::ptr::read_volatile(PL011_FR) } & PL011_FR_TXFF != 0 {}
        unsafe {
            core::ptr::write_volatile(PL011_DR, byte as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConsoleSink, FramebufferConsole};
    use crate::bootinfo::FramebufferFormat;

    #[test]
    fn framebuffer_console_rejects_unknown_format() {
        let mut pixels = [0u32; 64];
        assert!(
            FramebufferConsole::new(pixels.as_mut_ptr(), 8, 8, 8, FramebufferFormat::Unknown,)
                .is_none()
        );
    }

    #[test]
    fn framebuffer_console_writes_pixels_on_multiple_lines() {
        let mut pixels = [0u32; 32 * 16];
        let mut console =
            FramebufferConsole::new(pixels.as_mut_ptr(), 32, 16, 32, FramebufferFormat::Rgbx8888)
                .expect("framebuffer console");

        console.write_str("A\nB");

        assert!(nonzero_region(&pixels, 32, 0, 0, 8, 8) > 0);
        assert!(nonzero_region(&pixels, 32, 0, 8, 8, 8) > 0);
    }

    #[test]
    fn framebuffer_console_normalizes_lowercase_to_uppercase() {
        let mut lower_pixels = [0u32; 32 * 8];
        let mut upper_pixels = [0u32; 32 * 8];
        let mut lower = FramebufferConsole::new(
            lower_pixels.as_mut_ptr(),
            32,
            8,
            32,
            FramebufferFormat::Rgbx8888,
        )
        .expect("lower framebuffer console");
        let mut upper = FramebufferConsole::new(
            upper_pixels.as_mut_ptr(),
            32,
            8,
            32,
            FramebufferFormat::Rgbx8888,
        )
        .expect("upper framebuffer console");

        lower.write_str("a");
        upper.write_str("A");

        assert_eq!(lower_pixels, upper_pixels);
    }

    fn nonzero_region(
        pixels: &[u32],
        stride: usize,
        origin_x: usize,
        origin_y: usize,
        width: usize,
        height: usize,
    ) -> usize {
        let mut count = 0;

        for y in origin_y..(origin_y + height) {
            for x in origin_x..(origin_x + width) {
                if pixels[(y * stride) + x] != 0 {
                    count += 1;
                }
            }
        }

        count
    }
}
