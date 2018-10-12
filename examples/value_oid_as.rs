extern crate libc;
extern crate sysctl;

use libc::c_int;

// Copied from /usr/include/sys/time.h
#[derive(Debug)]
#[repr(C)]
struct ClockInfo {
    hz: c_int,   /* clock frequency */
    tick: c_int, /* micro-seconds per hz tick */
    spare: c_int,
    stathz: c_int, /* statistics clock frequency */
    profhz: c_int, /* profiling clock frequency */
}
#[cfg(not(target_os = "linux"))] //no CTL_KERN or KERN_CLOCKRATE on x86_64 linux
fn main() {
    let oid: Vec<i32> = vec![libc::CTL_KERN, libc::KERN_CLOCKRATE];
    let val: Box<ClockInfo> = sysctl::Ctl { oid }.value_as().expect("could not get value");
    println!("{:?}", val);
}
#[cfg(target_os = "linux")]
fn main() {
    //TODO implement linux test here?
}
