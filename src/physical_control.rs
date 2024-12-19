use std::io;
use std::process::Command;
use std::time::Duration;

pub fn keyboard_input(input: &str) -> io::Result<()> {
    for char in input.chars() {
        Command::new("wlrctl")
            .arg("keyboard")
            .arg("type")
            .arg(char.to_string())
            .spawn()?
            .wait()?;

        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

pub fn keyboard_input_with_modifiers(input: &str, modifiers: &str) -> io::Result<()> {
    for char in input.chars() {
        Command::new("wlrctl")
            .arg("keyboard")
            .arg("type")
            .arg(char.to_string())
            .arg("modifiers")
            .arg(modifiers)
            .spawn()?
            .wait()?;

        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

pub fn pointer_click(button: &str) -> io::Result<()> {
    Command::new("wlrctl")
        .arg("pointer")
        .arg("click")
        .arg(button)
        .spawn()?
        .wait()?;

    Ok(())
}

pub fn pointer_move_to(x: u16, y: u16) -> io::Result<()> {
    pointer_move_relative(-9999, -9999)?;

    std::thread::sleep(Duration::from_millis(100));

    pointer_move_relative(x as i32, y as i32)?;

    Ok(())
}

pub fn pointer_move_relative(x: i32, y: i32) -> io::Result<()> {
    Command::new("wlrctl")
        .arg("pointer")
        .arg("move")
        .arg(x.to_string())
        .arg(y.to_string())
        .spawn()?
        .wait()?;

    Ok(())
}
