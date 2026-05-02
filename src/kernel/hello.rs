use core::fmt::{self, Write};

use crate::HELLO_WORLD;

pub fn render(writer: &mut impl Write) -> fmt::Result {
    writeln!(writer, "{HELLO_WORLD}")
}
