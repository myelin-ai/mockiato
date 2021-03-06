use mockiato::mockable;
use std::io::{self, Write};

#[mockable(remote)]
trait Write {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;

    fn flush(&mut self) -> io::Result<()>;
}

#[test]
fn remote_trait() {
    let write_mock = WriteMock::new();
    let _assert_implements_io_write: &dyn io::Write = &write_mock;
}
