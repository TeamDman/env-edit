use env_edit::env_reader::get_machine_env;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    println!("Hello, world!");
    let environment_variables = get_machine_env()?;
    for env_var in environment_variables {
        println!("{:?}", env_var);
    }
    Ok(())
}