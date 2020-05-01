#![windows_subsystem = "windows"]   // This make rust to not open a console for the app
#[cfg(windows)] extern crate winapi;


// to convert our Rust UTF-8 strings to Windows UTF-16 strings
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::iter::once;
//------------------------------------------------------------
use std::ptr::null_mut;
use std::io::Error;

// use is a "using namespace" equivalent

use self::winapi::shared::minwindef::{
    LRESULT, UINT,
    WPARAM, LPARAM,
};

use self::winapi::shared::windef::{
    HWND, LPRECT, RECT
};
use self::winapi::shared::basetsd::LONG_PTR;
use self::winapi::um::libloaderapi::GetModuleHandleW;
use self::winapi::um::winnt::VOID;

use self::winapi::um::wingdi::{
    StretchDIBits,
    BITMAPINFO, BITMAPINFOHEADER, RGBQUAD,
    DIB_RGB_COLORS, SRCCOPY, BI_RGB
};

use self::winapi::um::winuser::{
    MSG, PAINTSTRUCT, WNDCLASSW, CREATESTRUCTW,
    CS_OWNDC, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, GWLP_USERDATA,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    WM_PAINT, WM_SIZE, WM_CREATE,
    DefWindowProcW, RegisterClassW, CreateWindowExW,
    TranslateMessage, DispatchMessageW, GetMessageW,
    SetWindowLongPtrW, GetWindowLongPtrW, GetClientRect,
    BeginPaint, EndPaint,
};

//--------------------------------------------------------------
unsafe extern "system" fn custom_win_proc(
    h_wnd: HWND, 
    msg: UINT, 
    w_param: WPARAM, 
    l_param: LPARAM
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let p_create = l_param as *mut CREATESTRUCTW;
            let window_data = (*p_create).lpCreateParams as *mut Window;
            SetWindowLongPtrW(h_wnd, GWLP_USERDATA, window_data as LONG_PTR)
        }
        WM_SIZE => {
            let window_data = GetWindowLongPtrW(h_wnd, GWLP_USERDATA) as *mut Window;
            let mut rect = std::mem::MaybeUninit::<RECT>::uninit().assume_init();
            GetClientRect(h_wnd, &mut rect as LPRECT);
            (*window_data).width = rect.right - rect.left;
            (*window_data).height = rect.top - rect.bottom;
            true as isize
        }
        WM_PAINT => {
            let mut paint_struct = std::mem::MaybeUninit::<PAINTSTRUCT>::zeroed().assume_init();
            let paint_struct_ptr = &mut paint_struct as *mut PAINTSTRUCT;

            let window_data = GetWindowLongPtrW(h_wnd, GWLP_USERDATA) as *mut Window;

            let hdc = BeginPaint(h_wnd, paint_struct_ptr);

            
            let canvas = &(*window_data).canvas;

            let bit_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: canvas.width as i32,
                    biHeight: -(canvas.height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0
                },
                bmiColors: [RGBQUAD{rgbBlue: 1, rgbGreen: 1, rgbRed: 1, rgbReserved: 1}]
            };

            let window_width: i32 = 800;
            let window_height: i32 = 600;
            StretchDIBits(hdc,
                        0, 0, canvas.width as i32, canvas.height as i32,
                        0, 0, window_width, window_height,
                        canvas.pixels.as_ptr() as *const VOID, &bit_info as *const BITMAPINFO,
                        DIB_RGB_COLORS, SRCCOPY);

            EndPaint(h_wnd, paint_struct_ptr);
            false as isize
        }
        _ => DefWindowProcW(h_wnd, msg, w_param, l_param)
    }
}

fn win32_string( value : &str ) -> Vec<u16> {
    OsStr::new( value ).encode_wide().chain( once( 0 ) ).collect()
}

// 'wl = window lifetime
pub struct Window {
    handle : HWND,
    pub width: i32,
    pub height: i32,
    canvas: ::painting::Canvas
}

pub fn create_window<'a>( name : &str, title : &str, canvas: ::painting::Canvas) -> Result<&'a mut Window, Error> {
    // convert the strings to win32 strings
    let name = win32_string( name );
    let title = win32_string( title );

    // tell rust compiler to overlook safety features
    unsafe {
        // Get the module handle
        let hinstance = GetModuleHandleW( null_mut() );

        // create the window class
        let wnd_class = WNDCLASSW {
            style : CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc : Some( custom_win_proc ),
            hInstance : hinstance,
            lpszClassName : name.as_ptr(),
            cbClsExtra : 0,
            cbWndExtra : 0,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null_mut(),
        };

        RegisterClassW( &wnd_class );

        let mut raw_window = std::boxed::Box::into_raw(
            std::boxed::Box::new(
                Window {
                    handle: 0 as HWND,
                    width: canvas.width as i32,
                    height: canvas.height as i32,
                    canvas: canvas
                }
        ));

        // Create the window
        let handle = CreateWindowExW(
            0,
            name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            //WS_VISIBLE,
            CW_USEDEFAULT,  // X default position
            CW_USEDEFAULT,  // Y default position
            (*raw_window).canvas.width as i32,
            (*raw_window).canvas.height as i32,
            null_mut(), // No parent
            null_mut(), // No menu
            hinstance,
            raw_window as *mut VOID // App data
        );

        if handle.is_null() {
            Err( Error::last_os_error() )
        } else {
            (*raw_window).handle = handle;
            Ok( &mut (*raw_window) )
        }
    }
}

impl Window {
    pub fn handle_message(&self) -> bool {
        unsafe {
            let mut message : MSG = std::mem::MaybeUninit::<MSG>::uninit().assume_init();
            if GetMessageW( &mut message as *mut MSG, self.handle, 0, 0 ) > 0 {
                TranslateMessage( &message as *const MSG );
                DispatchMessageW( &message as *const MSG );

                true
            } else {
                false
            }
        }
    }

    pub fn swap_buffer(&mut self, canvas: ::painting::Canvas) {
        self.canvas = canvas;
    }
}
