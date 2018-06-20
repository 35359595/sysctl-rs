//! A simplified interface to the `sysctl` system call.
//!
//! Currently built for and only tested on FreeBSD.
//!
//! # Example: Get description and value
//! ```
//! extern crate sysctl;
//! #[cfg(not(any(target_os = "macos", target_os = "linux")))]
//! fn main() {
//!
//!     let ctl = "kern.osrevision";
//!
//!     let d: String = sysctl::description(ctl).unwrap();
//!     println!("Description: {:?}", d);
//!
//!     let val_enum = sysctl::value(ctl).unwrap();
//!     if let sysctl::CtlValue::Int(val) = val_enum {
//!         println!("Value: {}", val);
//!     }
//! }
//! #[cfg(target_os = "macos")]
//! fn main() {
//!
//!     let ctl = "kern.osrevision";
//!
//!     let val_enum = sysctl::value(ctl).unwrap();
//!     if let sysctl::CtlValue::Int(val) = val_enum {
//!         println!("Value: {}", val);
//!     }
//! }
//! ```
//! # Example: Get value as struct
//! ```
//! extern crate sysctl;
//! extern crate libc;
//!
//! use libc::c_int;
//!
//! #[derive(Debug)]
//! #[repr(C)]
//! struct ClockInfo {
//!     hz: c_int, /* clock frequency */
//!     tick: c_int, /* micro-seconds per hz tick */
//!     spare: c_int,
//!     stathz: c_int, /* statistics clock frequency */
//!     profhz: c_int, /* profiling clock frequency */
//! }
//!
//! fn main() {
//!     println!("{:?}", sysctl::value_as::<ClockInfo>("kern.clockrate"));
//! }
//! ```
extern crate libc;
extern crate byteorder;
extern crate errno;

use libc::{c_int, c_uint, c_uchar, c_void};
use libc::sysctl;
use libc::BUFSIZ;

use std::convert;
use std::mem;
use std::ptr;
use std::str;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use std::f32;
use errno::{errno, set_errno};
use byteorder::{LittleEndian, ByteOrder, WriteBytesExt};
use std::fmt;
use std::string::String;

// CTL* constants belong to libc crate but have not been added there yet.
// They will be removed from here once in the libc crate.
pub const CTL_MAXNAME: c_uint = 24;

pub const CTLTYPE: c_uint = 0xf; /* mask for the type */

pub const CTLTYPE_NODE: c_uint = 1;
pub const CTLTYPE_INT: c_uint = 2;
pub const CTLTYPE_STRING: c_uint = 3;
pub const CTLTYPE_S64: c_uint = 4;
pub const CTLTYPE_OPAQUE: c_uint = 5;
pub const CTLTYPE_STRUCT: c_uint = 5;
pub const CTLTYPE_UINT: c_uint = 6;
pub const CTLTYPE_LONG: c_uint = 7;
pub const CTLTYPE_ULONG: c_uint = 8;
pub const CTLTYPE_U64: c_uint = 9;
pub const CTLTYPE_U8: c_uint = 10;
pub const CTLTYPE_U16: c_uint = 11;
pub const CTLTYPE_S8: c_uint = 12;
pub const CTLTYPE_S16: c_uint = 13;
pub const CTLTYPE_S32: c_uint = 14;
pub const CTLTYPE_U32: c_uint = 15;

pub const CTLFLAG_RD: c_uint = 0x80000000;
pub const CTLFLAG_WR: c_uint = 0x40000000;
pub const CTLFLAG_RW: c_uint = 0x80000000 | 0x40000000;
pub const CTLFLAG_ANYBODY: c_uint = 268435456;
pub const CTLFLAG_SECURE: c_uint = 134217728;
pub const CTLFLAG_PRISON: c_uint = 67108864;
pub const CTLFLAG_DYN: c_uint = 33554432;
pub const CTLFLAG_SKIP: c_uint = 16777216;
pub const CTLFLAG_TUN: c_uint = 0x00080000;
pub const CTLFLAG_RDTUN: c_uint = 2148007936;
pub const CTLFLAG_RWTUN: c_uint = 3221749760;
pub const CTLFLAG_MPSAFE: c_uint = 262144;
pub const CTLFLAG_VNET: c_uint = 131072;
pub const CTLFLAG_DYING: c_uint = 65536;
pub const CTLFLAG_CAPRD: c_uint = 32768;
pub const CTLFLAG_CAPWR: c_uint = 16384;
pub const CTLFLAG_STATS: c_uint = 8192;
pub const CTLFLAG_NOFETCH: c_uint = 4096;
pub const CTLFLAG_CAPRW: c_uint = 49152;
pub const CTLFLAG_SECURE1: c_uint = 134217728;
pub const CTLFLAG_SECURE2: c_uint = 135266304;
pub const CTLFLAG_SECURE3: c_uint = 136314880;

pub const CTLMASK_SECURE: c_uint = 15728640;
pub const CTLSHIFT_SECURE: c_uint = 20;


#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u32)]
enum CtlType {
    Node = 1,
    Int = 2,
    String = 3,
    S64 = 4,
    Struct = 5,
    Uint = 6,
    Long = 7,
    Ulong = 8,
    U64 = 9,
    U8 = 10,
    U16 = 11,
    S8 = 12,
    S16 = 13,
    S32 = 14,
    U32 = 15,
    // Added custom types below
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    Temperature = 16,
}
impl convert::From<u32> for CtlType {
    fn from(t: u32) -> Self {
        assert!(t >= 1 && t <= 16);
        unsafe { mem::transmute(t) }
    }
}
impl<'a> convert::From<&'a CtlValue> for CtlType {
    fn from(t: &'a CtlValue) -> Self {
        match t {
            &CtlValue::Node(_) => CtlType::Node,
            &CtlValue::Int(_) => CtlType::Int,
            &CtlValue::String(_) => CtlType::String,
            &CtlValue::S64(_) => CtlType::S64,
            &CtlValue::Struct(_) => CtlType::Struct,
            &CtlValue::Uint(_) => CtlType::Uint,
            &CtlValue::Long(_) => CtlType::Long,
            &CtlValue::Ulong(_) => CtlType::Ulong,
            &CtlValue::U64(_) => CtlType::U64,
            &CtlValue::U8(_) => CtlType::U8,
            &CtlValue::U16(_) => CtlType::U16,
            &CtlValue::S8(_) => CtlType::S8,
            &CtlValue::S16(_) => CtlType::S16,
            &CtlValue::S32(_) => CtlType::S32,
            &CtlValue::U32(_) => CtlType::U32,
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            &CtlValue::Temperature(_) => CtlType::Temperature,
        }
    }
}

/// An Enum that holds all values returned by sysctl calls.
/// Extract inner value with `if let` or `match`.
///
/// # Example
///
/// ```ignore
/// let val_enum = sysctl::value("kern.osrevision");
///
/// if let sysctl::CtlValue::Int(val) = val_enum {
///     println!("Value: {}", val);
/// }
/// ```
#[derive(Debug, PartialEq, PartialOrd)]
pub enum CtlValue {
    Node(Vec<u8>),
    Int(i32),
    String(String),
    S64(u64),
    Struct(Vec<u8>),
    Uint(u32),
    Long(i64),
    Ulong(u64),
    U64(u64),
    U8(u8),
    U16(u16),
    S8(i8),
    S16(i16),
    S32(i32),
    U32(u32),
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    Temperature(Temperature),
}

impl fmt::Display for CtlValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let &CtlValue::Int(val) = self {
            write!(f, "{}", val)
        }
        else if let &CtlValue::String(ref val_s) = self {
           write!(f, "{}", val_s) 
        } 
        else if let &CtlValue::S64(val_s64) = self {
            write!(f, "{}", val_s64)
        }
        else if let &CtlValue::Uint(val_uint) = self {
            write!(f, "{}", val_uint)
        }
        else if let &CtlValue::Long(val_long) = self {
            write!(f, "{}", val_long)
        }
        else if let &CtlValue::Ulong(val_ulong) = self {
            write!(f, "{}", val_ulong)
        }
        else if let &CtlValue::U64(val_u64) = self {
            write!(f, "{}", val_u64)
        }
        else if let &CtlValue::U8(val_u8) = self {
            write!(f, "{}", val_u8)
        }
        else if let &CtlValue::U16(val_u16) = self {
            write!(f, "{}", val_u16)
        }
        else if let &CtlValue::S8(val_s8) = self {
            write!(f, "{}", val_s8)
        }
        else if let &CtlValue::S16(val_s16) = self {
            write!(f, "{}", val_s16)
        }
        else if let &CtlValue::S32(val_s32) = self {
            write!(f, "{}", val_s32)
        }
        else if let &CtlValue::U32(val_u32) = self {
            write!(f, "{}", val_u32)
        }
        else {
            write!(f, "{}", "unknown value format for Display. use {:?}")
        }
    }
}

impl convert::Into<String> for CtlValue {
    fn into(self) -> String {
        if let CtlValue::Int(val) = self {
            val.to_string()
        }
        else if let CtlValue::String(val_s) = self {
           val_s.to_string() 
        } 
        else if let CtlValue::S64(val_s64) = self {
            val_s64.to_string()
        }
        else if let CtlValue::Uint(val_uint) = self {
            val_uint.to_string()
        }
        else if let CtlValue::Long(val_long) = self {
            val_long.to_string()
        }
        else if let CtlValue::Ulong(val_ulong) = self {
            val_ulong.to_string()
        }
        else if let CtlValue::U64(val_u64) = self {
            val_u64.to_string()
        }
        else if let CtlValue::U8(val_u8) = self {
            val_u8.to_string()
        }
        else if let CtlValue::U16(val_u16) = self {
            val_u16.to_string()
        }
        else if let CtlValue::S8(val_s8) = self {
            val_s8.to_string()
        }
        else if let CtlValue::S16(val_s16) = self {
            val_s16.to_string()
        }
        else if let CtlValue::S32(val_s32) = self {
            val_s32.to_string()
        }
        else if let CtlValue::U32(val_u32) = self {
            val_u32.to_string()
        }
        else {
            String::from("unknown value format for Display. use {:?}")
        }
    }
}

#[derive(Debug, PartialEq)]
struct CtlInfo {
    ctl_type: CtlType,
    fmt: String,
    flags: u32,
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl CtlInfo {
    fn is_temperature(&self) -> bool {
        match &self.fmt[0..2] {
            "IK" => true,
            _ => false,
        }
    }
}

/// A custom type for temperature sysctls.
///
/// # Example
/// ```
/// extern crate sysctl;
/// #[cfg(not(any(target_os = "macos", target_os = "linux")))]
/// fn main() {
///     let val_enum = sysctl::value("dev.cpu.0.temperature").unwrap();
///     if let sysctl::CtlValue::Temperature(val) = val_enum {
///         println!("Temperature: {:.2}K, {:.2}F, {:.2}C",
///                  val.kelvin(),
///                  val.fahrenheit(),
///                  val.celsius());
///     } else {
///         panic!("Error, not a temperature ctl!")
///     }
/// }
/// ```
/// Not available on MacOS
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Temperature {
    value: f32, // Kelvin
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl Temperature {
    pub fn kelvin(&self) -> f32 {
        self.value
    }
    pub fn celsius(&self) -> f32 {
        self.value - 273.15
    }
    pub fn fahrenheit(&self) -> f32 {
        1.8 * self.celsius() + 32.0
    }
}

fn errno_string() -> String {
    let e = errno();
    set_errno(e);
    let code = e.0;
    format!("errno {}: {}", code, e)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn name2oid(name: &str) -> Result<Vec<c_int>, String> {

    // Request command for OID
    let oid: [c_int; 2] = [0, 3];

    let mut len: usize = CTL_MAXNAME as usize * mem::size_of::<c_int>();

    // We get results in this vector
    let mut res: Vec<c_int> = vec![0; CTL_MAXNAME as usize];

    let ret = unsafe {
        sysctl(oid.as_ptr(),
               2,
               res.as_mut_ptr() as *mut c_void,
               &mut len,
               name.as_ptr() as *const c_void,
               name.len())
    };
    if ret < 0 {
        return Err(errno_string());
    }

    // len is in bytes, convert to number of c_ints
    len /= mem::size_of::<c_int>();

    // Trim result vector
    res.truncate(len);

    Ok(res)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn name2oid(name: &str) -> Result<Vec<c_int>, String> {

    // Request command for OID
    let mut oid: [c_int; 2] = [0, 3];

    let mut len: usize = CTL_MAXNAME as usize * mem::size_of::<c_int>();

    // We get results in this vector
    let mut res: Vec<c_int> = vec![0; CTL_MAXNAME as usize];

    let ret = unsafe {
        sysctl(oid.as_mut_ptr(),
               2,
               res.as_mut_ptr() as *mut c_void,
               &mut len,
               name.as_ptr() as *mut c_void,
               name.len())
    };
    if ret < 0 {
        return Err(errno_string());
    }

    // len is in bytes, convert to number of c_ints
    len /= mem::size_of::<c_int>();

    // Trim result vector
    res.truncate(len);

    Ok(res)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn oidfmt(oid: &[c_int]) -> Result<CtlInfo, String> {

    // Request command for type info
    let mut qoid: Vec<c_int> = vec![0, 4];
    qoid.extend(oid);

    // Store results here
    let mut buf: [c_uchar; BUFSIZ as usize] = [0; BUFSIZ as usize];
    let mut buf_len = mem::size_of_val(&buf);
    let ret = unsafe {
        sysctl(qoid.as_ptr(),
               qoid.len() as u32,
               buf.as_mut_ptr() as *mut c_void,
               &mut buf_len,
               ptr::null(),
               0)
    };
    if ret != 0 {
        return Err(errno_string());
    }

    // 'Kind' is the first 32 bits of result buffer
    let kind = LittleEndian::read_u32(&buf);

    // 'Type' is the first 4 bits of 'Kind'
    let ctltype_val = kind & CTLTYPE as u32;

    // 'fmt' is after 'Kind' in result buffer
    let fmt: String = match str::from_utf8(&buf[mem::size_of::<u32>()..buf_len]) {
        Ok(x) => x.to_owned(),
        Err(e) => return Err(format!("{}", e)),
    };

    let s = CtlInfo {
        ctl_type: CtlType::from(ctltype_val),
        fmt: fmt,
        flags: kind,
    };
    Ok(s)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn temperature(info: &CtlInfo, val: &Vec<u8>) -> Result<CtlValue, String> {
    let prec: u32 = {
        match info.fmt.len() {
            l if l > 2 => {
                match info.fmt[2..3].parse::<u32>() {
                    Ok(x) if x <= 9 => x,
                    _ => 1,
                }
            }
            _ => 1,
        }
    };

    let base = 10u32.pow(prec) as f32;

    let make_temp = move |f: f32| -> Result<CtlValue, String> {
        Ok(CtlValue::Temperature(Temperature { value: f / base }))
    };

    match info.ctl_type {
        CtlType::Int => make_temp(LittleEndian::read_i32(&val) as f32),
        CtlType::S64 => make_temp(LittleEndian::read_u64(&val) as f32),
        CtlType::Uint => make_temp(LittleEndian::read_u32(&val) as f32),
        CtlType::Long => make_temp(LittleEndian::read_i64(&val) as f32),
        CtlType::Ulong => make_temp(LittleEndian::read_u64(&val) as f32),
        CtlType::U64 => make_temp(LittleEndian::read_u64(&val) as f32),
        CtlType::U8 => make_temp(val[0] as u8 as f32),
        CtlType::U16 => make_temp(LittleEndian::read_u16(&val) as f32),
        CtlType::S8 => make_temp(val[0] as i8 as f32),
        CtlType::S16 => make_temp(LittleEndian::read_i16(&val) as f32),
        CtlType::S32 => make_temp(LittleEndian::read_i32(&val) as f32),
        CtlType::U32 => make_temp(LittleEndian::read_u32(&val) as f32),
        _ => Err("No matching type for value".into()),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn oidfmt(oid: &[c_int]) -> Result<CtlInfo, String> {

    // Request command for type info
    let mut qoid: Vec<c_int> = vec![0, 4];
    qoid.extend(oid);

    // Store results here
    let mut buf: [c_uchar; BUFSIZ as usize] = [0; BUFSIZ as usize];
    let mut buf_len = mem::size_of_val(&buf);
    #[cfg(target_os = "linux")]
    let ret = unsafe {
        sysctl(qoid.as_mut_ptr(),
               qoid.len() as i32,
               buf.as_mut_ptr() as *mut c_void,
               &mut buf_len,
               ptr::null_mut(),
               0)
    };
    #[cfg(target_os = "macos")]
    let ret = unsafe {
        sysctl(qoid.as_mut_ptr(),
               qoid.len() as u32,
               buf.as_mut_ptr() as *mut c_void,
               &mut buf_len,
               ptr::null_mut(),
               0)
    };
    if ret != 0 {
        return Err(errno_string());
    }

    // 'Kind' is the first 32 bits of result buffer
    let kind = LittleEndian::read_u32(&buf);

    // 'Type' is the first 4 bits of 'Kind'
    let ctltype_val = kind & CTLTYPE as u32;

    // 'fmt' is after 'Kind' in result buffer
    let fmt: String = match str::from_utf8(&buf[mem::size_of::<u32>()..buf_len]) {
        Ok(x) => x.to_owned(),
        Err(e) => return Err(format!("{}", e)),
    };

    let s = CtlInfo {
        ctl_type: CtlType::from(ctltype_val),
        fmt: fmt,
        flags: kind,
    };
    Ok(s)
}

/// Takes the name of the OID as argument and returns
/// a result containing the sysctl value if success,
/// the errno caused by sysctl() as string if failure.
///
/// # Example
/// ```
/// extern crate sysctl;
///
/// fn main() {
///     println!("Value: {:?}", sysctl::value("kern.osrevision"));
/// }
/// ```
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn value(name: &str) -> Result<CtlValue, String> {
    match name2oid(name) {
        Ok(v) => value_oid(&v),
        Err(e) => Err(e),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn value(name: &str) -> Result<CtlValue, String> {
    match name2oid(name) {
        Ok(mut v) => value_oid(&mut v),
        Err(e) => Err(e),
    }
}

/// Takes an OID as argument and returns a result
/// containing the sysctl value if success, the errno
/// caused by sysctl() as string if failure.
///
/// # Example
/// ```
/// extern crate sysctl;
/// extern crate libc;
///
/// fn main() {
///     let oid = vec![libc::CTL_KERN, libc::KERN_OSREV];
///     println!("Value: {:?}", sysctl::value_oid(&oid));
/// }
/// ```
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn value_oid(oid: &mut Vec<i32>) -> Result<CtlValue, String> {

    let info: CtlInfo = try!(oidfmt(&oid));

    // First get size of value in bytes
    let mut val_len = 0;
    let ret = unsafe {
        sysctl(oid.as_ptr(),
               oid.len() as u32,
               ptr::null_mut(),
               &mut val_len,
               ptr::null(),
               0)
    };
    if ret < 0 {
        return Err(errno_string());
    }

    // Then get value
    let mut val: Vec<c_uchar> = vec![0; val_len];
    let mut new_val_len = val_len;
    let ret = unsafe {
        sysctl(oid.as_ptr(),
               oid.len() as u32,
               val.as_mut_ptr() as *mut c_void,
               &mut new_val_len,
               ptr::null(),
               0)
    };
    if ret < 0 {
        return Err(errno_string());
    }

    // Confirm that we got the bytes we requested
    assert_eq!(val_len, new_val_len);

    // Special treatment for temperature ctls.
    if info.is_temperature() {
        return temperature(&info, &val);
    }

    // Wrap in Enum and return
    match info.ctl_type {
        CtlType::Node => Ok(CtlValue::Node(val)),
        CtlType::Int => Ok(CtlValue::Int(LittleEndian::read_i32(&val))),
        CtlType::String => {
            if let Ok(s) = str::from_utf8(&val[..val.len() - 1]) {
                Ok(CtlValue::String(s.into()))
            } else {
                Err("Error parsing string".into())
            }
        }
        CtlType::S64 => Ok(CtlValue::S64(LittleEndian::read_u64(&val))),
        CtlType::Struct => Ok(CtlValue::Struct(val)),
        CtlType::Uint => Ok(CtlValue::Uint(LittleEndian::read_u32(&val))),
        CtlType::Long => Ok(CtlValue::Long(LittleEndian::read_i64(&val))),
        CtlType::Ulong => Ok(CtlValue::Ulong(LittleEndian::read_u64(&val))),
        CtlType::U64 => Ok(CtlValue::U64(LittleEndian::read_u64(&val))),
        CtlType::U8 => Ok(CtlValue::U8(val[0])),
        CtlType::U16 => Ok(CtlValue::U16(LittleEndian::read_u16(&val))),
        CtlType::S8 => Ok(CtlValue::S8(val[0] as i8)),
        CtlType::S16 => Ok(CtlValue::S16(LittleEndian::read_i16(&val))),
        CtlType::S32 => Ok(CtlValue::S32(LittleEndian::read_i32(&val))),
        CtlType::U32 => Ok(CtlValue::U32(LittleEndian::read_u32(&val))),
        _ => Err("No matching type for value".into()),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn value_oid(oid: &mut Vec<i32>) -> Result<CtlValue, String> {

    let info: CtlInfo = try!(oidfmt(&oid));

    // First get size of value in bytes
    let mut val_len = 0;
    #[cfg(target_os = "linux")]
    let ret = unsafe {
        sysctl(oid.as_mut_ptr(),
               oid.len() as i32,
               ptr::null_mut(),
               &mut val_len,
               ptr::null_mut(),
               0)
    };
    #[cfg(target_os = "macos")]
    let ret = unsafe {
        sysctl(oid.as_mut_ptr(),
               oid.len() as u32,
               ptr::null_mut(),
               &mut val_len,
               ptr::null_mut(),
               0)
    };
    if ret < 0 {
        return Err(errno_string());
    }

    // Then get value
    let mut val: Vec<c_uchar> = vec![0; val_len];
    let mut new_val_len = val_len;
    #[cfg(target_os = "linux")]
    let ret = unsafe {
        sysctl(oid.as_mut_ptr(),
               oid.len() as i32,
               val.as_mut_ptr() as *mut c_void,
               &mut new_val_len,
               ptr::null_mut(),
               0)
    };
    #[cfg(target_os = "macos")]
    let ret = unsafe {
        sysctl(oid.as_mut_ptr(),
               oid.len() as u32,
               val.as_mut_ptr() as *mut c_void,
               &mut new_val_len,
               ptr::null_mut(),
               0)
    };
    if ret < 0 {
        return Err(errno_string());
    }

    // Confirm that we got the bytes we requested
    assert_eq!(val_len, new_val_len);

    // Wrap in Enum and return
    match info.ctl_type {
        CtlType::Node => Ok(CtlValue::Node(val)),
        CtlType::Int => Ok(CtlValue::Int(LittleEndian::read_i32(&val))),
        CtlType::String => {
            if let Ok(s) = str::from_utf8(&val[..val.len() - 1]) {
                Ok(CtlValue::String(s.into()))
            } else {
                Err("Error parsing string".into())
            }
        }
        CtlType::S64 => Ok(CtlValue::S64(LittleEndian::read_u64(&val))),
        CtlType::Struct => Ok(CtlValue::Struct(val)),
        CtlType::Uint => Ok(CtlValue::Uint(LittleEndian::read_u32(&val))),
        CtlType::Long => Ok(CtlValue::Long(LittleEndian::read_i64(&val))),
        CtlType::Ulong => Ok(CtlValue::Ulong(LittleEndian::read_u64(&val))),
        CtlType::U64 => Ok(CtlValue::U64(LittleEndian::read_u64(&val))),
        CtlType::U8 => Ok(CtlValue::U8(val[0])),
        CtlType::U16 => Ok(CtlValue::U16(LittleEndian::read_u16(&val))),
        CtlType::S8 => Ok(CtlValue::S8(val[0] as i8)),
        CtlType::S16 => Ok(CtlValue::S16(LittleEndian::read_i16(&val))),
        CtlType::S32 => Ok(CtlValue::S32(LittleEndian::read_i32(&val))),
        CtlType::U32 => Ok(CtlValue::U32(LittleEndian::read_u32(&val))),
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        _ => Err("No matching type for value".into()),
    }
}

/// A generic function that takes a string as argument and
/// returns a result containing the sysctl value if success,
/// the errno caused by sysctl() as string if failure.
///
/// Can only be called for sysctls of type Opaque or Struct.
///
/// # Example
/// ```
/// extern crate sysctl;
/// extern crate libc;
///
/// use libc::c_int;
///
/// #[derive(Debug)]
/// #[repr(C)]
/// struct ClockInfo {
///     hz: c_int, /* clock frequency */
///     tick: c_int, /* micro-seconds per hz tick */
///     spare: c_int,
///     stathz: c_int, /* statistics clock frequency */
///     profhz: c_int, /* profiling clock frequency */
/// }
///
/// fn main() {
///     println!("{:?}", sysctl::value_as::<ClockInfo>("kern.clockrate"));
/// }
/// ```
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn value_as<T>(name: &str) -> Result<Box<T>, String> {
    match name2oid(name) {
        Ok(v) => value_oid_as::<T>(&v),
        Err(e) => Err(e),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn value_as<T>(name: &str) -> Result<Box<T>, String> {
    match name2oid(name) {
        Ok(mut v) => value_oid_as::<T>(&mut v),
        Err(e) => Err(e),
    }
}

/// A generic function that takes an OID as argument and
/// returns a result containing the sysctl value if success,
/// the errno caused by sysctl() as string if failure.
///
/// Can only be called for sysctls of type Opaque or Struct.
///
/// # Example
/// ```
/// extern crate sysctl;
/// extern crate libc;
///
/// use libc::c_int;
///
/// #[derive(Debug)]
/// #[repr(C)]
/// struct ClockInfo {
///     hz: c_int, /* clock frequency */
///     tick: c_int, /* micro-seconds per hz tick */
///     spare: c_int,
///     stathz: c_int, /* statistics clock frequency */
///     profhz: c_int, /* profiling clock frequency */
/// }
///
/// #[cfg(not(any(target_os = "macos", target_os = "linux")))]
/// fn main() {
///     let oid = vec![libc::CTL_KERN, libc::KERN_CLOCKRATE];
///     println!("{:?}", sysctl::value_oid_as::<ClockInfo>(&oid));
/// }
/// #[cfg(target_os = "macos")]
/// fn main() {
///     let mut oid = vec![libc::CTL_KERN, libc::KERN_CLOCKRATE];
///     println!("{:?}", sysctl::value_oid_as::<ClockInfo>(&mut oid));
/// }
/// ```
pub fn value_oid_as<T>(oid: &mut Vec<i32>) -> Result<Box<T>, String> {

    let val_enum = try!(value_oid(oid));

    // Some structs are apparently reported as Node so this check is invalid..
    // let ctl_type = CtlType::from(&val_enum);
    // assert_eq!(CtlType::Struct, ctl_type, "Error type is not struct/opaque");

    // TODO: refactor this when we have better clue to what's going on
    if let CtlValue::Struct(val) = val_enum {
        // Make sure we got correct data size
        assert_eq!(mem::size_of::<T>(),
                   val.len(),
                   "Error memory size mismatch. Size of struct {}, size of data retrieved {}.",
                   mem::size_of::<T>(),
                   val.len());

        // val is Vec<u8>
        let val_array: Box<[u8]> = val.into_boxed_slice();
        let val_raw: *mut T = Box::into_raw(val_array) as *mut T;
        let val_box: Box<T> = unsafe { Box::from_raw(val_raw) };
        Ok(val_box)
    } else if let CtlValue::Node(val) = val_enum {
        // Make sure we got correct data size
        assert_eq!(mem::size_of::<T>(),
                   val.len(),
                   "Error memory size mismatch. Size of struct {}, size of data retrieved {}.",
                   mem::size_of::<T>(),
                   val.len());

        // val is Vec<u8>
        let val_array: Box<[u8]> = val.into_boxed_slice();
        let val_raw: *mut T = Box::into_raw(val_array) as *mut T;
        let val_box: Box<T> = unsafe { Box::from_raw(val_raw) };
        Ok(val_box)
    } else {
        Err("Error extracting value".into())
    }
}

/// Sets the value of a sysctl.
/// Fetches and returns the new value if successful, errno string if failure.
///
/// # Example
/// ```
/// extern crate sysctl;
///
/// fn main() {
///     println!("{:?}", sysctl::set_value("hw.usb.debug", sysctl::CtlValue::Int(1)));
/// }
/// ```
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn set_value(name: &str, value: CtlValue) -> Result<CtlValue, String> {

    let oid = try!(name2oid(name));
    let info: CtlInfo = try!(oidfmt(&oid));

    let ctl_type = CtlType::from(&value);
    assert_eq!(info.ctl_type,
               ctl_type,
               "Error type mismatch. Type given {:?}, sysctl type: {:?}",
               ctl_type,
               info.ctl_type);


    // TODO rest of the types

    if let CtlValue::Int(v) = value {
        let mut bytes = vec![];
        bytes
            .write_i32::<LittleEndian>(v)
            .expect("Error parsing value to byte array");

        // Set value
        let ret = unsafe {
            sysctl(oid.as_ptr(),
                   oid.len() as u32,
                   ptr::null_mut(),
                   ptr::null_mut(),
                   bytes.as_ptr() as *const c_void,
                   bytes.len())
        };
        if ret < 0 {
            return Err(errno_string());
        }
    }

    // Get the new value and return for confirmation
    self::value(name)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn set_value(name: &str, value: CtlValue) -> Result<CtlValue, String> {

    let mut oid = try!(name2oid(name));
    let info: CtlInfo = try!(oidfmt(&oid));

    let ctl_type = CtlType::from(&value);
    assert_eq!(info.ctl_type,
               ctl_type,
               "Error type mismatch. Type given {:?}, sysctl type: {:?}",
               ctl_type,
               info.ctl_type);


    // TODO rest of the types

    if let CtlValue::Int(v) = value {
        let mut bytes = vec![];
        bytes
            .write_i32::<LittleEndian>(v)
            .expect("Error parsing value to byte array");

        // Set value
        #[cfg(target_os = "linux")]
        let ret = unsafe {
            sysctl(oid.as_mut_ptr(),
                   oid.len() as i32,
                   ptr::null_mut(),
                   ptr::null_mut(),
                   bytes.as_ptr() as *mut c_void,
                   bytes.len())
        };
        #[cfg(target_os = "macos")]
        let ret = unsafe {
            sysctl(oid.as_mut_ptr(),
                   oid.len() as u32,
                   ptr::null_mut(),
                   ptr::null_mut(),
                   bytes.as_ptr() as *mut c_void,
                   bytes.len())
        };
        if ret < 0 {
            return Err(errno_string());
        }
    }

    // Get the new value and return for confirmation
    self::value(name)
}

/// Returns a result containing the sysctl description if success,
/// the errno caused by sysctl() as string if failure.
///
/// # Example
/// ```
/// extern crate sysctl;
///
/// fn main() {
///     println!("Description: {:?}", sysctl::description("kern.osrevision"));
/// }
/// ```
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn description(name: &str) -> Result<String, String> {

    let oid: Vec<c_int> = try!(name2oid(name));

    // Request command for description
    let mut qoid: Vec<c_int> = vec![0, 5];
    qoid.extend(oid);

    // Store results in u8 array
    let mut buf: [c_uchar; BUFSIZ as usize] = [0; BUFSIZ as usize];
    let mut buf_len = mem::size_of_val(&buf);
    let ret = unsafe {
        sysctl(qoid.as_ptr(),
               qoid.len() as u32,
               buf.as_mut_ptr() as *mut c_void,
               &mut buf_len,
               ptr::null(),
               0)
    };
    if ret != 0 {
        return Err(errno_string());
    }

    // Use buf_len - 1 so that we remove the trailing NULL
    match str::from_utf8(&buf[..buf_len - 1]) {
        Ok(s) => Ok(s.to_owned()),
        Err(e) => Err(format!("{}", e)),
    }
}
//NOT WORKING ON MacOS
// #[cfg(target_os = "macos")]
// pub fn description(name: &str) -> Result<String, String> {

//     let oid: Vec<c_int> = try!(name2oid(name));

//     // Request command for description
//     let mut qoid: Vec<c_int> = vec![0, 5];
//     qoid.extend(oid);

//     // Store results in u8 array
//     let mut buf: [c_uchar; BUFSIZ as usize] = [0; BUFSIZ as usize];
//     let mut buf_len = mem::size_of_val(&buf);
//     let ret = unsafe {
//         sysctl(qoid.as_mut_ptr(),
//                qoid.len() as u32,
//                buf.as_mut_ptr() as *mut c_void,
//                &mut buf_len,
//                ptr::null_mut(),
//                0)
//     };
//     if ret != 0 {
//         return Err(errno_string());
//     }

//     // Use buf_len - 1 so that we remove the trailing NULL
//     match str::from_utf8(&buf[..buf_len - 1]) {
//         Ok(s) => Ok(s.to_owned()),
//         Err(e) => Err(format!("{}", e)),
//     }
// }

#[cfg(test)]
mod tests {

    // use ::*;
    // use libc::*;
    use super::*;
    use std::process::Command;

    #[test]
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn ctl_mib() {
        let oid = name2oid("kern.proc.pid").unwrap();
        assert_eq!(oid.len(), 3);
        assert_eq!(oid[0], CTL_KERN);
        assert_eq!(oid[1], KERN_PROC);
        assert_eq!(oid[2], KERN_PROC_PID);
    }

    #[test]
    fn ctl_type() {
        let oid = name2oid("kern").unwrap();
        let fmt = oidfmt(&oid).unwrap();
        assert_eq!(fmt.ctl_type, CtlType::Node);

        let oid = name2oid("kern.osrelease").unwrap();
        let fmt = oidfmt(&oid).unwrap();
        assert_eq!(fmt.ctl_type, CtlType::String);

        let oid = name2oid("kern.osrevision").unwrap();
        let fmt = oidfmt(&oid).unwrap();
        assert_eq!(fmt.ctl_type, CtlType::Int);
    }

    #[test]
    fn ctl_flags() {
        let oid = name2oid("kern.osrelease").unwrap();
        let fmt = oidfmt(&oid).unwrap();

        assert_eq!(fmt.flags & CTLFLAG_RD, CTLFLAG_RD);
        assert_eq!(fmt.flags & CTLFLAG_WR, 0);
    }

    #[test]
    fn ctl_value_int() {
        let output = Command::new("sysctl")
            .arg("-n")
            .arg("kern.osrevision")
            .output()
            .expect("failed to execute process");
        let rev_str = String::from_utf8_lossy(&output.stdout);
        let rev = rev_str.trim().parse::<i32>().unwrap();
        let n = match value("kern.osrevision") {
            Ok(CtlValue::Int(n)) => n,
            Ok(_) => 0,
            Err(_) => 0,
        };
        assert_eq!(n, rev);
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn ctl_value_oid_int() {
        let output = Command::new("sysctl")
            .arg("-n")
            .arg("kern.osrevision")
            .output()
            .expect("failed to execute process");
        let rev_str = String::from_utf8_lossy(&output.stdout);
        let rev = rev_str.trim().parse::<i32>().unwrap();
        let n = match value_oid(&mut vec![libc::CTL_KERN, libc::KERN_OSREV]) {
            Ok(CtlValue::Int(n)) => n,
            Ok(_) => 0,
            Err(_) => 0,
        };
        assert_eq!(n, rev);
    }

    #[test]
    fn ctl_value_string() {
        let output = Command::new("sysctl")
            .arg("-n")
            .arg("kern.version")
            .output()
            .expect("failed to execute process");
        let ver = String::from_utf8_lossy(&output.stdout);
        let s = match value("kern.version") {
            Ok(CtlValue::String(s)) => s,
            _ => "...".into(),
        };
        assert_eq!(s.trim(), ver.trim());
    }

    #[test]
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn ctl_description() {
        let s: String = match description("hw.ncpu") {
            Ok(s) => s,
            _ => "...".into(),
        };
        assert_eq!(s, "8");
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    #[test]
    fn ctl_temperature_ik() {
        let info = CtlInfo {
            ctl_type: CtlType::Int,
            fmt: "IK".into(),
            flags: 0,
        };
        let mut val = vec![];
        // Default value (IK) in deciKelvin integer
        val.write_i32::<LittleEndian>(3330)
            .expect("Error parsing value to byte array");

        let t = temperature(&info, &val).unwrap();
        if let CtlValue::Temperature(tt) = t {
            assert!(tt.kelvin() - 333.0 < 0.1);
            assert!(tt.celsius() - 59.85 < 0.1);
            assert!(tt.fahrenheit() - 139.73 < 0.1);
        } else {
            assert!(false);
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    #[test]
    fn ctl_temperature_ik3() {
        let info = CtlInfo {
            ctl_type: CtlType::Int,
            fmt: "IK3".into(),
            flags: 0,
        };
        let mut val = vec![];
        // Set value in milliKelvin
        val.write_i32::<LittleEndian>(333000)
            .expect("Error parsing value to byte array");

        let t = temperature(&info, &val).unwrap();
        if let CtlValue::Temperature(tt) = t {
            assert!(tt.kelvin() - 333.0 < 0.1);
        } else {
            assert!(false);
        }
    }
}
