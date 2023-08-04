use crate::Runtime;
use multicall::multicall;

pub const STD: &str = include_str!("../std.spl");
pub const NET: &str = include_str!("../net.spl");
pub const ITER: &str = include_str!("../iter.spl");
pub const HTTP: &str = include_str!("../http.spl");
pub const STREAM: &str = include_str!("../stream.spl");
pub const MESSAGING: &str = include_str!("../messaging.spl");

pub fn register(runtime: &mut Runtime) {
    multicall! {
        &mut runtime.embedded_files:
        insert("std.spl", STD);
        insert("net.spl", NET);
        insert("iter.spl", ITER);
        insert("http.spl", HTTP);
        insert("stream.spl", STREAM);
        insert("messaging.spl", MESSAGING);
    }
}
