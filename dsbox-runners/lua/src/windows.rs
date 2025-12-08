fn convert_to_acp(input: &str) -> bstr::BString {
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
        let multilen = windows::Win32::Globalization::WideCharToMultiByte(
            windows::Win32::Globalization::CP_ACP,
            Default::default(),
            buf.as_slice(),
            None,
            windows::core::PCSTR::null(),
            None,
        );
        let mut out = bstr::BString::new(vec![0u8; multilen as usize]);
        let multilen = windows::Win32::Globalization::WideCharToMultiByte(
            windows::Win32::Globalization::CP_ACP,
            Default::default(),
            buf.as_slice(),
            Some(out.as_mut_slice()),
            windows::core::PCSTR::null(),
            None,
        );
        out.truncate(multilen as usize);
        out
    }
}