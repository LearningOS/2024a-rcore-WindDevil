//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

/// 引入系统调用的最大数量
use crate::config::MAX_SYSCALL_NUM;
use crate::loader::{get_app_data, get_num_app};
/// 引入虚拟页号
use crate::mm::{MapPermission, VPNRange, VirtPageNum};
use crate::sync::UPSafeCell;
/// 引入获取时间的函数,注意题目要求是毫秒级别
use crate::timer::get_time_ms;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        // 记录任务的首次调用时间
        next_task.task_first_start_time = get_time_ms();
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            // 记录任务的首次调用时间
            if inner.tasks[next].task_first_start_time == 0 {
                inner.tasks[next].task_first_start_time = get_time_ms();
            }
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    /// 获取当前的任务状态
    fn get_current_task_status(&self) -> TaskStatus {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].task_status
    }

    /// 记录当前任务的系统调用次数
    fn record_current_task_syscall_times(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        // 这里因为是基础类型所以直接拷贝
        let current = inner.current_task;
        // 这边因为修改了所以用的是可变引用
        // 在同一时间,如果有一个可变引用,就不能有其他引用,包括不可变引用
        inner.tasks[current].syscall_times[syscall_id] += 1;
    }

    /// 获取当前任务的系统调用次数
    fn get_current_task_syscall_times(&self) -> [u32; MAX_SYSCALL_NUM] {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].syscall_times
    }

    /// 获取当前的任务起始时间
    fn get_current_task_first_start_time(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].task_first_start_time
    }

    /// 检查是否能够分配出当前虚拟内存的连续内存区域
    fn check_alloc_map_area(&self, start_vpn: VirtPageNum, end_vpn: VirtPageNum) -> bool {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let memory_set = &inner.tasks[current].memory_set;
        let vpn_range = VPNRange::new(start_vpn, end_vpn);
        for vpn in vpn_range {
            if let Some(pte) = memory_set.translate(vpn) {
                if pte.is_valid() {
                    return false;
                }
            }
        }
        true
    }

    /// 创建一个新的连续内存区域
    fn create_new_map_area(&self, start_vpn: VirtPageNum, end_vpn: VirtPageNum, permission:MapPermission) -> isize {
        if !self.check_alloc_map_area(start_vpn, end_vpn) {
            return -1;
        }
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].memory_set.insert_framed_area(start_vpn.into(), end_vpn.into(), permission);
        0
    }

}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// 获取当前的任务状态
pub fn get_current_task_status() -> TaskStatus {
    TASK_MANAGER.get_current_task_status()
}

/// 记录当前任务的系统调用次数
pub fn record_current_task_syscall_times(syscall_id: usize) {
    TASK_MANAGER.record_current_task_syscall_times(syscall_id);
}

/// 获取当前任务的系统调用次数
pub fn get_current_task_syscall_times() -> [u32; MAX_SYSCALL_NUM] {
    TASK_MANAGER.get_current_task_syscall_times()
}

/// 获取当前的任务起始时间
pub fn get_current_task_first_start_time() -> usize {
    TASK_MANAGER.get_current_task_first_start_time()
}

/// 创建一个新的连续内存区域
pub fn create_new_map_area(start_vpn: VirtPageNum, end_vpn: VirtPageNum, permission:MapPermission) -> isize {
    TASK_MANAGER.create_new_map_area(start_vpn, end_vpn, permission)
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}
