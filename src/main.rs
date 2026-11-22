//! SigmaOS component binary: shell-config

use sigmaos_shell-config::Component;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Component::new().operate()?;
    println!("sigmaos-shell-config: ok");
    Ok(())
}
