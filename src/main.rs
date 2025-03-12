use env_edit::env_reader::get_machine_env_var;
use env_edit::env_reader::list_machine_env_var;
use env_edit::env_writer::set_machine_env_var;
use env_edit::init::init;
use env_edit::win_elevation::ensure_elevated;
use tracing::info;

fn main() -> eyre::Result<()> {
    init()?;
    info!("Hello, world!");

    do_stuff()?;

    ensure_elevated()?;

    do_admin_stuff()?;

    info!("We have reached the end of the program.");
    wait_for_enter();
    Ok(())
}

fn do_stuff() -> eyre::Result<()> {
    // 1) Print out all machine environment variables
    let environment_variables = list_machine_env_var()?;
    let dump = serde_json::to_string_pretty(&environment_variables)?;
    println!("{}", dump);
    Ok(())
}

fn do_admin_stuff() -> eyre::Result<()> {
    // 2) Our test: "ENV_EDIT_TEST"
    //    If it doesn't exist, set to "0".
    //    If it does exist, parse as integer, increment by 1, and update it.
    test_env_edit_test()?;
    Ok(())
}

fn test_env_edit_test() -> eyre::Result<()> {
    let key_name = "ENV_EDIT_TEST";

    // Try to get the current value of ENV_EDIT_TEST
    let maybe_value = get_machine_env_var(key_name)?;
    let current_int = match maybe_value {
        None => 0, // Key did not exist
        Some(ref s) => s.parse::<i64>().unwrap_or(0),
    };
    let next_int = current_int + 1;

    set_machine_env_var(key_name, &next_int.to_string())?;
    info!("Set {key_name} to {next_int}");
    Ok(())
}

/// Waits for the user to press Enter.
pub fn wait_for_enter() {
    print!("Press Enter to exit...");
    std::io::Write::flush(&mut std::io::stdout()).unwrap(); // Ensure the prompt is displayed immediately
    let _ = std::io::stdin().read_line(&mut String::new()); // Wait for user input
}
