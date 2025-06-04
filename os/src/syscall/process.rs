//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
use crate::task::get_sys_call_times;
use crate::config::MAX_SYSCALL_NUM;

pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            // 读取用户空间一个字节
            let ptr = id as *const u8;
            let value = unsafe { ptr.read_volatile() };
            value as isize
        }
        1 => {
            // 写入用户空间一个字节
            let ptr = id as *mut u8;
            let value = (data & 0xFF) as u8;
            unsafe { ptr.write_volatile(value); }
            0
        }
        2 => {
            // 查询当前任务某系统调用次数
            if id < MAX_SYSCALL_NUM {
                //increase_sys_call(id); // 本次调用也计入统计
                let times = get_sys_call_times();
                times[id] as isize
            } else {
                -1
            }
        }
        _ => -1,
    }
}
