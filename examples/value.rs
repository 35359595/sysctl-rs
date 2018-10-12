extern crate sysctl;

use sysctl::{Ctl, CtlValue};

#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
fn main() {
    let ctl = Ctl::new("kern.osrevision").expect("could not get sysctl");

    let name = ctl.name().expect("could not get name");

    println!("\nRead sysctl {}", name);

    let description = ctl.description().expect("could not get description");

    println!("Description: {:?}", description);

    let val_enum = ctl.value().expect("could not get sysctl value");

    if let CtlValue::Int(val) = val_enum {
        println!("Value: {}", val);
    }
    
    //Same output with fmt::Display
    println!("Value: {}", val_enum);

    //Get value to String
    let val_str: String = val_enum.into();

    //Same output as with fmt::Display
    println!("String val: {}", val_str);
}

//MacOS value extraction
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn main() {
    // on macos the `name` and `newp` parameters of the sysctl(3)
    // syscall API are not marked `const`. This means the sysctl
    // structure has to be mutable.
    let ctl = Ctl::new("kernel.hostname").expect("could not get sysctl");

    let name = ctl.name().expect("could not get name");

    println!("\nRead sysctl {}", name);

    // sysctl descriptions are not available on macos.

    let val_enum = ctl.value().expect("could not get sysctl value");

    if let CtlValue::Int(val) = val_enum {
        println!("Value: {}", val);
    }
}
