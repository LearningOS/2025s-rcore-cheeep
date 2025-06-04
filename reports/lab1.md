## chapter3 

### 编程

#### 要求

```
获取任务信息
在 ch3 中，我们的系统已经能够支持多个任务分时轮流运行，我们希望引入一个新的系统调用 ``sys_trace``（ID 为 410）用来追踪当前任务系统调用的历史信息，并做对应的修改。定义如下。

fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize
调用规范：
这个系统调用有三种功能，根据 trace_request 的值不同，执行不同的操作：

如果 trace_request 为 0，则 id 应被视作 *const u8 ，表示读取当前任务 id 地址处一个字节的无符号整数值。此时应忽略 data 参数。返回值为 id 地址处的值。
如果 trace_request 为 1，则 id 应被视作 *mut u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。
如果 trace_request 为 2，表示查询当前任务调用编号为 id 的系统调用的次数，返回值为这个调用次数。本次调用也计入统计 。

否则，忽略其他参数，返回值为 -1。
```

#### 提示

```
可以扩展 TaskManagerInner 中的结构来维护新的信息。src/task
系统调用次数可以考虑在内核态的 syscall 函数中统计。src/syscall
不要害怕使用 unsafe 做类型转换，这在内核处理用户调用时是不可避免的。
```

#### 实现

新增全局遍历 src/trap/config.rs

```
/// Maximum number of system calls supported
pub const MAX_SYSCALL_NUM: usize = 500; 
```

扩展TaskManagerInner src/task/task.rs

```rust
use crate::config::MAX_SYSCALL_NUM;
/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// system call times  新增
    pub sys_call_times: [u32; MAX_SYSCALL_NUM]
}
```

在mod.rs中修改初始化

```
use crate::config::MAX_SYSCALL_NUM;
let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            ///新增
            sys_call_times: [0; MAX_SYSCALL_NUM], 
        }; MAX_APP_NUM];
```

在syscall时增加计数

首先新增一个计数功能 src/task/mod.rs

```
/// 增加当前任务的某个系统调用计数
pub fn increase_sys_call(syscall_id: usize) {
    use crate::config::MAX_SYSCALL_NUM;
    if syscall_id < MAX_SYSCALL_NUM {
        let mut inner = TASK_MANAGER.inner.exclusive_access();
        let current = inner.current_task; // 存到局部变量
        inner.tasks[current].sys_call_times[syscall_id] += 1;
    }
}
```

然后修改 src/syscall/mod.rs 完成自增

```
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    increase_sys_call(syscall_id); //新增
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TRACE => sys_trace(args[0], args[1], args[2]),
        
        //SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut TaskInfo),

        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
```

为TaskManager实现get_sys_call_times方法, 获取当前任务current_task对应的TaskControlBlock结构体的系统调用数组的拷贝 src/tsak/mod.rs

```
fn get_sys_call_times(&self) -> [u32; MAX_SYSCALL_NUM] {
     let inner: core::cell::RefMut<'_, TaskManagerInner> = self.inner.exclusive_access();
     inner.tasks[inner.current_task].sys_call_times.clone()


pub fn get_sys_call_times() -> [u32; MAX_SYSCALL_NUM] {
    TASK_MANAGER.get_sys_call_times()
}
```

完善process.rs的sys_task_info  src/process.rs

```
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
            /// 查询当前任务某系统调用次数
            if id < MAX_SYSCALL_NUM {
                //increase_sys_call(id); 
                let times = get_sys_call_times();
                times[id] as isize
            } else {
                -1
            }
        }
        _ => -1,
    }
}
```

上述修改后本地编译可以通过，而CI因为依赖版本问题无法通过。无奈换一种方式重新实现。

```
use crate::config::MAX_SYSCALL_NUM;

syscall_count: UPSafeCell<[[usize; MAX_SYSCALL_NUM]; MAX_APP_NUM]>,

/// inc_syscall_count()
pub fn add_syscall_count(syscall_id: usize) {
    TASK_MANAGER.increase_sys_call(syscall_id);
}

/// get_syscall_count()
pub fn get_syscall_count(syscall_id: usize) -> usize {
    TASK_MANAGER.get_sys_call_times(syscall_id)
}

// TODO: implement the syscall
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => unsafe { (id as *const u8).read() as isize },
        1 => {
            unsafe { (id as *mut u8).write(data as u8); }
            0
        }
        2 => TASK_MANAGER.get_sys_call_times(id) as isize,
        _ => -1,
    }
}
```



### 简答作业

1.正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

```
[rustsbi] Implementation     : RustSBI-QEMU Version 0.2.0-alpha.2

[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.

ch2b_bad_address：用户态访问非法地址，触发页异常（PageFault），被内核捕获并终止。
ch2b_bad_instructions：用户态执行 S 态特权指令，触发非法指令异常（IllegalInstruction），被内核捕获并终止。
ch2b_bad_register：用户态访问 S 态寄存器，触发非法指令异常（IllegalInstruction），被内核捕获并终止。
```

2.深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:

​	(1) L40：刚进入 __restore 时，sp 代表了什么值。请指出 __restore 的两种使用情景。	

```
sp 的含义：
刚进入 __restore 时，sp 指向保存了所有通用寄存器和部分特权寄存器的 trap 上下文（TrapContext）结构体的栈顶地址。

__restore 的两种使用情景：
从 S 态返回 U 态（trap 返回用户态，恢复用户上下文）。
切换任务时，恢复另一个任务的上下文（任务切换）。
```

​	(2) L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

```
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```

```
ld t0, 32*8(sp)      # 加载 sstatus
ld t1, 33*8(sp)      # 加载 sepc
ld t2, 2*8(sp)       # 加载 sscratch
csrw sstatus, t0     # 恢复 sstatus
csrw sepc, t1        # 恢复 sepc
csrw sscratch, t2    # 恢复 sscratch

sstatus：保存了 S/U 态切换时的状态（如中断使能、当前特权级等），恢复后决定返回后 CPU 的状态。
sepc：保存 trap 发生时的 PC，恢复后决定返回用户态时从哪条指令继续执行。
sscratch：通常用于保存 S 态下的 sp，方便 trap 处理时切换栈。
```

​	(3) L50-L56：为何跳过了 x2 和 x4？

```
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```

```
x2 (sp) 和 x4 (tp) 没有直接恢复：
x2 (sp)：当前 sp 正在使用，最后通过 csrrw sp, sscratch, sp 恢复。
x4 (tp)：通常用于线程指针，部分实现会单独处理或不需要恢复
```

​	

​	(4)L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```
csrrw sp, sscratch, sp
```

```
sp：切换为用户态的栈指针（TrapContext 里保存的 sp）。
sscratch：保存 S 态的栈指针，方便下次 trap 时切换回来,指向内核栈
```

​	(5)__restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

```
sret 指令会根据 sstatus 和 sepc 的值，将 CPU 从 S 态切换回 U 态，并跳转到 sepc 指定的指令继续执行。
```

​	(6)L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```
csrrw sp, sscratch, sp
```

```
sp：变为 S 态的栈指针（原来保存在 sscratch）。
sscratch：变为 trap 时 U 态的 sp，方便异常返回时恢复。
```

​	(7)从 U 态进入 S 态是哪一条指令发生的？	

```
（用户程序执行 ecall，触发 trap，进入 S 态）。
```



## **荣誉准则**



1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > copilot

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > copilot

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
