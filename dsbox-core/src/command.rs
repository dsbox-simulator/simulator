use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RunnerCommand {
    pub runner: String,
    pub args: Vec<String>,
}

impl std::fmt::Display for RunnerCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.runner, self.args.join(" "))
    }
}

impl RunnerCommand {
    pub fn new(runner: impl Into<String>, args: impl Into<Vec<String>>) -> Self {
        Self {
            runner: runner.into(),
            args: args.into(),
        }
    }

    pub fn from_commandline(
        runner: impl Into<String>,
        commandline: impl AsRef<str>,
    ) -> Option<Self> {
        Some(Self {
            runner: runner.into(),
            args: Self::split_command(commandline.as_ref())?,
        })
    }

    pub fn runner(&self) -> &str {
        &self.runner
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }



    #[cfg(not(windows))]
    pub fn split_command(input: impl AsRef<str>) -> Option<Vec<String>> {
        let input = input.as_ref();
        shlex::split(input)
    }

    #[cfg(windows)]
    pub fn split_command(input: impl AsRef<str>) -> Option<Vec<String>> {
        let input = input.as_ref();
        unsafe {
            let widelen = windows::Win32::Globalization::MultiByteToWideChar(
                windows::Win32::Globalization::CP_UTF8,
                Default::default(),
                input.as_bytes(),
                None,
            );
            let mut buf = vec![0u16; widelen as usize];
            let widelen = windows::Win32::Globalization::MultiByteToWideChar(
                windows::Win32::Globalization::CP_UTF8,
                Default::default(),
                input.as_bytes(),
                Some(buf.as_mut_slice()),
            );
            buf.truncate(widelen as usize);
            let mut num_args = 0;
            let argv = windows::Win32::UI::Shell::CommandLineToArgvW(
                windows::core::PCWSTR(buf.as_ptr()),
                &mut num_args,
            );
            if num_args == 0 {
                return None;
            }
            let mut args = Vec::with_capacity(num_args as usize);
            for i in 0..num_args {
                let arg = argv.add(i as usize);
                let multilen = windows::Win32::Globalization::WideCharToMultiByte(
                    windows::Win32::Globalization::CP_UTF8,
                    Default::default(),
                    (*arg).as_wide(),
                    None,
                    windows::core::PCSTR::null(),
                    None,
                );
                let mut buf = vec![0; multilen as usize];
                let multilen = windows::Win32::Globalization::WideCharToMultiByte(
                    windows::Win32::Globalization::CP_UTF8,
                    Default::default(),
                    (*arg).as_wide(),
                    Some(buf.as_mut_slice()),
                    windows::core::PCSTR::null(),
                    None,
                );
                buf.truncate(multilen as usize);
                args.push(String::from_utf8_lossy(buf.as_slice()).to_string());
            }
            Some(args)
        }
    }
}
