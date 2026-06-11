#[derive(Debug)]
pub struct SongSelector {
    connected: bool,
}

impl SongSelector {
    pub fn new() -> Self {
        Self { connected: false }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn reconnect(&mut self) -> bool {
        #[cfg(windows)]
        {
            self.connected = windows_impl::process_exists("DivaMegaMix.exe");
        }
        #[cfg(not(windows))]
        {
            self.connected = false;
        }
        self.connected
    }

    pub fn change_song(&mut self, song_id: u32) -> Result<String, String> {
        if !self.is_connected() && !self.reconnect() {
            return Err("游戏进程未连接".to_string());
        }
        #[cfg(windows)]
        {
            windows_impl::change_song(song_id)?;
            return Ok("success!".to_string());
        }
        #[cfg(not(windows))]
        {
            let _ = song_id;
            Err("切歌功能仅支持 Windows".to_string())
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use std::mem::size_of;

    use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
        MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
    };

    const PROCESS_NAME: &str = "DivaMegaMix.exe";
    const LAST_SELECT_PVID: usize = 0x12B6350;
    const LAST_SELECT_SORT: usize = 0x12B6354;
    const LAST_SELECT_DIFF: usize = 0x12B635C;
    const EDEN_OFFSET: usize = 0x105F460;
    const CHANGE_SONG_SELECT: usize = 0xCC61098;
    const START_CHANGE: usize = 0xCC610A0;

    pub fn process_exists(name: &str) -> bool {
        find_process(name).is_some()
    }

    pub fn change_song(song_id: u32) -> Result<(), String> {
        let pid = find_process(PROCESS_NAME).ok_or_else(|| "游戏进程未连接".to_string())?;
        let base =
            module_base(pid, PROCESS_NAME).ok_or_else(|| "无法获取游戏模块基地址".to_string())?;
        let process = unsafe {
            OpenProcess(
                PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
                false,
                pid,
            )
        }
        .map_err(|error| format!("打开游戏进程失败: {error}"))?;

        let result = write_song(process, base, song_id);
        unsafe {
            let _ = CloseHandle(process);
        }
        result
    }

    fn write_song(process: HANDLE, base: usize, song_id: u32) -> Result<(), String> {
        let mut pvid = base + LAST_SELECT_PVID;
        let mut sort = base + LAST_SELECT_SORT;
        let mut diff = base + LAST_SELECT_DIFF;
        let change_song_select = base + CHANGE_SONG_SELECT;
        let start_change = base + START_CHANGE;

        if read_i32(process, pvid)? == 0 {
            pvid += EDEN_OFFSET;
            sort += EDEN_OFFSET;
            diff += EDEN_OFFSET;
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        if read_i32(process, change_song_select)? == 6 {
            write_i32(process, change_song_select, 6)?;
            write_i32(process, start_change, 2)?;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        write_selection(process, pvid, sort, diff, song_id)?;
        trigger_selection(process, change_song_select, start_change)?;
        Ok(())
    }

    fn write_selection(
        process: HANDLE,
        pvid: usize,
        sort: usize,
        diff: usize,
        song_id: u32,
    ) -> Result<(), String> {
        write_i32(process, pvid, song_id as i32)?;
        write_i32(process, sort, 1)?;
        write_i32(process, diff, 19)
    }

    fn trigger_selection(
        process: HANDLE,
        change_song_select: usize,
        start_change: usize,
    ) -> Result<(), String> {
        write_i32(process, change_song_select, 5)?;
        write_i32(process, start_change, 2)
    }

    fn find_process(name: &str) -> Option<u32> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()? };
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = None;
        unsafe {
            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    if wide_to_string(&entry.szExeFile) == name {
                        found = Some(entry.th32ProcessID);
                        break;
                    }
                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }
        found
    }

    fn module_base(pid: u32, name: &str) -> Option<usize> {
        let snapshot =
            unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid).ok()? };
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry = MODULEENTRY32W {
            dwSize: size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };
        let mut base = None;
        unsafe {
            if Module32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    if wide_to_string(&entry.szModule) == name {
                        base = Some(entry.modBaseAddr as usize);
                        break;
                    }
                    if Module32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }
        base
    }

    fn read_i32(process: HANDLE, address: usize) -> Result<i32, String> {
        let mut value = 0i32;
        let mut read = 0;
        unsafe {
            ReadProcessMemory(
                process,
                address as *const _,
                &mut value as *mut _ as *mut _,
                size_of::<i32>(),
                Some(&mut read),
            )
            .map_err(|error| format!("读取游戏内存失败: {error}"))?;
        }
        if read != size_of::<i32>() {
            return Err(format!(
                "读取游戏内存不完整: {read}/{} 字节",
                size_of::<i32>()
            ));
        }
        Ok(value)
    }

    fn write_i32(process: HANDLE, address: usize, value: i32) -> Result<(), String> {
        let mut written = 0;
        unsafe {
            WriteProcessMemory(
                process,
                address as *const _,
                &value as *const _ as *const _,
                size_of::<i32>(),
                Some(&mut written),
            )
            .map_err(|error| format!("写入游戏内存失败: {error}"))?;
        }
        if written != size_of::<i32>() {
            return Err(format!(
                "写入游戏内存不完整: {written}/{} 字节",
                size_of::<i32>()
            ));
        }
        Ok(())
    }

    fn wide_to_string(buffer: &[u16]) -> String {
        let length = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..length])
    }
}
