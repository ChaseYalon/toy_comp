use crate::ToyPtr;
use crate::builtins;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::Duration;
use tiny_http::{Request, Server};
use ureq;
#[unsafe(no_mangle)]

pub extern "C" fn toy_net_get_url(url: ToyPtr) -> ToyPtr {
    let url_str = unsafe { CStr::from_ptr(url as *const c_char) }
        .to_str()
        .unwrap();

    let body: String = ureq::get(url_str)
        .call()
        .unwrap()
        .body_mut()
        .read_to_string()
        .unwrap();

    let c_string = CString::new(body).unwrap();
    let len = c_string.as_bytes_with_nul().len();
    let bytes = c_string.as_bytes_with_nul();

    //THIS IS WRONG
    //IT DOES NOT USE THE TOY_MALLOC DEBUG ALLOCATOR
    //IT WILL CAUSE MEMORY LEAKS
    unsafe {
        let ptr: *mut i8 = libc::malloc(len) as *mut c_char;
        std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, ptr, len);
        return ptr as i64;
    };
}

static GLOBAL_SERVER_ARR: OnceLock<RwLock<Vec<Server>>> = OnceLock::new();

fn get_servers() -> &'static RwLock<Vec<Server>> {
    GLOBAL_SERVER_ARR.get_or_init(|| RwLock::new(vec![]))
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_net_configure_http_server(port: ToyPtr) -> ToyPtr {
    let server = Server::http(&format!("0.0.0.0:{}", port)).unwrap();
    let mut servers = get_servers().write().unwrap();
    servers.push(server);
    return (servers.len() - 1) as i64;
}

static GLOBAL_REQUEST: OnceLock<Mutex<Option<Request>>> = OnceLock::new();

fn get_request() -> &'static Mutex<Option<Request>> {
    GLOBAL_REQUEST.get_or_init(|| Mutex::new(None))
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_net_connection_requested(server_handle: ToyPtr) -> ToyPtr {
    let servers = get_servers().read().unwrap();
    let server = &servers[server_handle as usize];

    return match server.recv_timeout(Duration::ZERO) {
        Ok(Some(request)) => {
            *get_request().lock().unwrap() = Some(request);
            1
        }
        _ => 0,
    };
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_net_read_request() -> *mut ToyPtr {
    let mut slot = get_request().lock().unwrap();

    match slot.take() {
        None => {
            panic!("[ERROR] GLOBAL_REQUEST Undefined");
        }
        Some(mut request) => {
            let method = request.method().to_string();
            let path = request.url().to_string();
            let ip = request
                .remote_addr()
                .map(|addr| addr.ip().to_string())
                .unwrap_or_default();

            let mut body = String::new();
            request.as_reader().read_to_string(&mut body).unwrap_or(0);

            *slot = Some(request);
            let arr_ptr =  builtins::toy_malloc_arr(4, 4, 1);

            let ip_s = CString::new(ip).unwrap();
            let method_s = CString::new(method).unwrap();
            let path_s = CString::new(path).unwrap();
            let body_s = CString::new(body).unwrap();
            let ip_ptr = builtins::toy_malloc(ip_s.as_ptr() as i64);
            let method_ptr = builtins::toy_malloc(method_s.as_ptr() as i64);
            let path_ptr = builtins::toy_malloc(path_s.as_ptr() as i64);
            let body_ptr = builtins::toy_malloc(body_s.as_ptr() as i64);

            builtins::toy_write_to_arr(arr_ptr, ip_ptr, 0, 0);
            builtins::toy_write_to_arr(arr_ptr, method_ptr, 1, 0);
            builtins::toy_write_to_arr(arr_ptr, path_ptr, 2, 0);
            builtins::toy_write_to_arr(arr_ptr, body_ptr, 3, 0);

            return arr_ptr as *mut i64;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_net_close_client() {
    let mut slot = get_request().lock().unwrap();
    drop(slot.take());
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_net_write_bytes(
    status_code: i64,
    content_type: ToyPtr,
    data: ToyPtr,
    size: i64,
) {
    let mut slot = get_request().lock().unwrap();
    let request = match slot.take() {
        Some(r) => r,
        None => return,
    };

    let ct = if content_type != 0 {
        unsafe { CStr::from_ptr(content_type as *const c_char) }
            .to_str()
            .unwrap_or("application/octet-stream")
            .to_string()
    } else {
        "application/octet-stream".to_string()
    };

    let body_bytes: Vec<u8> = if data != 0 && size > 0 {
        unsafe { std::slice::from_raw_parts(data as *const u8, size as usize) }.to_vec()
    } else {
        vec![]
    };

    let response = tiny_http::Response::new(
        tiny_http::StatusCode(status_code as u16),
        vec![
            tiny_http::Header::from_bytes("Content-Type", ct.as_bytes()).unwrap(),
            tiny_http::Header::from_bytes("Access-Control-Allow-Origin", b"*").unwrap(),
            tiny_http::Header::from_bytes("Connection", b"close").unwrap(),
        ],
        std::io::Cursor::new(body_bytes),
        Some(size as usize),
        None,
    );

    request.respond(response).unwrap_or(());
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_net_write_response(status_code: i64, content_type: ToyPtr, body: ToyPtr) {
    let ct = if content_type != 0 {
        content_type
    } else {
        c"text/plain; charset=utf-8".as_ptr() as i64
    };

    let b = if body != 0 { body } else { c"".as_ptr() as i64 };

    let size = unsafe { libc::strlen(b as *const c_char) as i64 };

    toy_net_write_bytes(status_code, ct, b, size);
}
