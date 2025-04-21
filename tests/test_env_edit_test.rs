use env_edit::env_reader::get_machine_env_var;
use env_edit::env_writer::set_machine_env_var;
use eyre::Result;

/// This test checks that "ENV_EDIT_TEST" increments or
/// sets to 0 if it doesn't exist.
#[test]
fn test_env_edit_test() -> Result<()> {
    // In an actual test, you might ensure you are running as admin or mock the registry:
    // For illustration, we just do a real check. If you're not admin, this might fail.
    // set_machine_env_var() requires admin privileges on Windows machine environment.
    // So you might do a "cargo test -- --test-threads=1" in an elevated console.

    // 1) Get the existing value
    let key_name = "ENV_EDIT_TEST";
    let maybe_value = get_machine_env_var(key_name)?;

    // 2) If it doesn't exist, set it to "0", else increment.
    let current_int = match maybe_value {
        None => 0,
        Some(s) => s.parse::<i64>().unwrap_or(0),
    };
    let next_int = current_int + 1;
    set_machine_env_var(key_name, &next_int.to_string())?;

    println!("Set {key_name} from {current_int} -> {next_int}");
    Ok(())
}
