use itertools::Itertools;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

/// Some common stuff I frequently use across projects.
pub fn init() -> eyre::Result<()> {
    color_eyre::install()?;
    init_logging();

    // fix colours in the default exe terminal
    #[cfg(windows)]
    let _ = windows_ansi::enable_ansi_support();
    // show no errors when colours unavailable
    // (happens when program used in a shell pipe situations)

    Ok(())
}

#[cfg(windows)]
mod windows_ansi {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    use windows::Win32::System::Console::GetConsoleMode;
    use windows::Win32::System::Console::GetStdHandle;
    use windows::Win32::System::Console::STD_OUTPUT_HANDLE;
    use windows::Win32::System::Console::SetConsoleMode;
    use windows::core::Result;

    pub fn enable_ansi_support() -> Result<()> {
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE)?;
            if handle == HANDLE::default() {
                return Err(windows::core::Error::from_win32());
            }

            let mut mode = std::mem::zeroed();
            GetConsoleMode(handle, &mut mode)?;
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING)?;
            Ok(())
        }
    }
}

fn init_logging() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .add_directive(
            format!(
                "
                {}=debug
                ",
                env!("CARGO_PKG_NAME")
            )
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.starts_with("//"))
            .filter(|line| !line.is_empty())
            .join(",")
            .trim()
            .parse()
            .unwrap(),
        );
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
}
