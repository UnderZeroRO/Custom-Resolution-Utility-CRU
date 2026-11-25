//! Probe: verify setupapi symbol resolution and enumerate display adapters.
//! Read-only: does NOT change any device state.

use std::ffi::c_void;

#[link(name = "setupapi", kind = "raw-dylib")]
extern "system" {
    fn SetupDiGetClassDevsW(
        class_guid: *const Guid,
        enumerator: *const u16,
        hwnd_parent: isize,
        flags: u32,
    ) -> isize;
    fn SetupDiEnumDeviceInfo(
        device_info_set: isize,
        member_index: u32,
        device_info_data: *mut SpDevInfoData,
    ) -> i32;
    fn SetupDiDestroyDeviceInfoList(device_info_set: isize);
}

const DIGCF_PRESENT: u32 = 0x0000_0002;

/// {4d36e968-e325-11ce-bfc1-08002be10318}
const GUID_DEVCLASS_DISPLAY: Guid = Guid {
    data1: 0x4d36e968,
    data2: 0xe325,
    data3: 0x11ce,
    data4: [0xbf, 0xc1, 0x08, 0x00, 0x2b, 0xe1, 0x03, 0x18],
};

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct SpDevInfoData {
    cb_size: u32,
    class_guid: Guid,
    dev_inst: u32,
    reserved: *mut c_void,
}

fn main() {
    unsafe {
        let devices =
            SetupDiGetClassDevsW(&GUID_DEVCLASS_DISPLAY, std::ptr::null(), 0, DIGCF_PRESENT);
        println!("handle = {devices}");
        if devices == -1 {
            println!("INVALID_HANDLE_VALUE");
            return;
        }

        let mut index = 0u32;
        let mut count = 0u32;
        loop {
            let mut device: SpDevInfoData = std::mem::zeroed();
            device.cb_size = std::mem::size_of::<SpDevInfoData>() as u32;
            if SetupDiEnumDeviceInfo(devices, index, &mut device) == 0 {
                break;
            }
            count += 1;
            println!(
                "device {index}: dev_inst={:#x} guid={:08X}-{:04X}-{:04X}",
                device.dev_inst,
                device.class_guid.data1,
                device.class_guid.data2,
                device.class_guid.data3
            );
            index += 1;
        }

        SetupDiDestroyDeviceInfoList(devices);
        println!("display adapters found: {count}");
    }
}