//! Process management syscalls


use crate::{
    // 引入页大小和最大系统调用数
    config::{MAX_SYSCALL_NUM, PAGE_SIZE}, 
    mm::{translated_byte_buffer, MapPermission, VirtAddr},
    // 这里为了方便，直接引入了task模块的所有内容
    task::*,
    // 这里也需要引入get_time_ms
    timer::{get_time_ms, get_time_us},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    //* 奇妙的跳过不允许直接转换的操作 */
    //? 从ref只能转换为自己类型的const裸指针 
    let src = &time_val as *const TimeVal;
    //? const裸指针可以转换为任何类型
    let mut src = src as usize;
    //* 奇妙的跳过不允许直接转换的操作 */
    let dst_vec = translated_byte_buffer(current_user_token(), _ts as *const u8, core::mem::size_of::<TimeVal>());
    for dst in dst_vec {
        unsafe {
            core::ptr::copy_nonoverlapping(src as *mut u8, dst.as_mut_ptr(), dst.len());
            src += dst.len();
        }
    }
    0
}

/// 记录当前任务的系统调用次数
pub fn sys_record_syscall(syscall_id: usize) -> isize {
    trace!("kernel: sys_record_syscall, syscall_id={}", syscall_id);
    record_current_task_syscall_times(syscall_id);
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    let task_info = TaskInfo {
        status: get_current_task_status(),
        syscall_times: get_current_task_syscall_times(),
        time: get_time_ms() - get_current_task_first_start_time(),
    };
    //* 奇妙的跳过不允许直接转换的操作 */
    //? 从ref只能转换为自己类型的const裸指针 
    let src = &task_info as *const TaskInfo;
    //? const裸指针可以转换为任何类型
    let mut src = src as usize;
    //* 奇妙的跳过不允许直接转换的操作 */
    let dst_vec = translated_byte_buffer(current_user_token(), _ti as *const u8, core::mem::size_of::<TaskInfo>());
    for dst in dst_vec {
        unsafe {
            core::ptr::copy_nonoverlapping(src as *mut u8, dst.as_mut_ptr(), dst.len());
            src += dst.len();
        }
    }
    0
}

//* 申请长度为 len 字节的物理内存（不要求实际物理内存位置，可以随便找一块），将其映射到 start 开始的虚存，内存页属性为 port
//* @_start 需要映射的虚存起始地址，要求按页对齐
//* @_len 映射字节长度，可以为 0
//* @_port 第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0
//* @return 成功返回 0，失败返回 -1
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    //* 可能的错误 */
    //* start 没有按页大小对齐 */
    //* port & !0x7 != 0 (port 其余位必须为0) */
    //* port & 0x7 = 0 (这样的内存无意义) */
    if _start % PAGE_SIZE != 0 ||
        _port & !0x7 != 0 ||
        _port & 0x7 == 0 {
        return -1;
    }
    // 这里使用from_bits_truncate是因为我们的flag中有未知的bit,所以不能使用from_bits
    let permission = MapPermission::from_bits_truncate((_port<<1) as u8)|MapPermission::U;
    // 创建一个新的内存区域
    // 向上取整和向下取整
    let start_vpn = VirtAddr::from(_start).floor();
    let end_vpn = VirtAddr::from(_start + _len).ceil();
    // 调用task模块的函数
    //* [start, start + len) 中存在已经被映射的页 */
    create_new_map_area(start_vpn, end_vpn, permission)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
