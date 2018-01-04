extern crate sysctl;
#[cfg(not(target_os = "macos"))]
fn main() {

    let ctl = "kern.osrevision";

    println!("\nRead sysctl {}", ctl);

    let d: String = sysctl::description(ctl).unwrap();
    println!("Description: {:?}", d);

    let val_enum = sysctl::value(ctl).unwrap();

    if let sysctl::CtlValue::Int(val) = val_enum {
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
#[cfg(target_os = "macos")]
fn main() {
    let ctl = "kern.osrevision";

    println!("\nRead sysctl {}", ctl);

    let val_enum = sysctl::value(ctl).unwrap();

    if let sysctl::CtlValue::Int(val) = val_enum {
        println!("Value: {}", val);
    }
    
    //Same output with fmt::Display
    println!("Value: {}", val_enum);

    //Get value to String
    let val_str: String = val_enum.into();

    //Same output as with fmt::Display
    println!("String val: {}", val_str);
}