## 实验作业

通过改变`TaskControlBlock`加入一些`task_info`所需的内容.

在每次进行任务切换之前判断这个任务是否被调用过.这样就可以得到它初次被调用的时间.

同样,每次进入`syscall`的时候对它的`id`进行记录,先获取当前的任务,然后对桶进行自增,获取其系统调用次数.

至于获取任务状态,这个因为我们已经访问到了当前任务,添加一个`fn`把这个字段取出来即可.


## 简答作业[](https://learningos.cn/rCore-Camp-Guide-2024A/chapter3/5exercise.html#id4 "永久链接至标题")

1.  正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 [三个 bad 测例 (ch2b_bad_*.rs)](https://github.com/LearningOS/rCore-Tutorial-Test-2024A/tree/master/src/bin) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

`sbi`的版本在log里.

出错的原因是:
- 使用了U模式下不允许运行的指令(一般是对CSR进行操作的指令)
- 访问了U模式下不能访问的CSR
- 访问了U模式下不允许访问的地址(0x00这种地址不在应用地址内)

```rust
[rustsbi] RustSBI version 0.3.1, adapting to RISC-V SBI v1.0.0
.______       __    __      _______.___________.  _______..______   __
|   _  \     |  |  |  |    /       |           | /       ||   _  \ |  |
|  |_)  |    |  |  |  |   |   (----`---|  |----`|   (----`|  |_)  ||  |
|      /     |  |  |  |    \   \       |  |      \   \    |   _  < |  |
|  |\  \----.|  `--'  |.----)   |      |  |  .----)   |   |  |_)  ||  |
| _| `._____| \______/ |_______/       |__|  |_______/    |______/ |__|
[rustsbi] Implementation     : RustSBI-QEMU Version 0.2.0-alpha.2
[rustsbi] Platform Name      : riscv-virtio,qemu
[rustsbi] Platform SMP       : 1
[rustsbi] Platform Memory    : 0x80000000..0x88000000
[rustsbi] Boot HART          : 0
[rustsbi] Device Tree Region : 0x87000000..0x87000f02
[rustsbi] Firmware Address   : 0x80000000
[rustsbi] Supervisor Address : 0x80200000
[rustsbi] pmp01: 0x00000000..0x80000000 (-wr)
[rustsbi] pmp02: 0x80000000..0x80200000 (---)
[rustsbi] pmp03: 0x80200000..0x88000000 (xwr)
[rustsbi] pmp04: 0x88000000..0x00000000 (-wr)
[kernel] Hello, world!
[kernel] num_app = 5
[kernel] app_0 [0x8020a038, 0x8020b360)
[kernel] app_1 [0x8020b360, 0x8020c730)
[kernel] app_2 [0x8020c730, 0x8020dcd8)
[kernel] app_3 [0x8020dcd8, 0x8020f090)
[kernel] app_4 [0x8020f090, 0x80210440)
[kernel] Loading app_0
Hello, world!
[kernel] Application exited with code 0
[kernel] Loading app_1
Into Test store_fault, we will insert an invalid store operation...
Kernel should kill this application!
[kernel] PageFault in application, kernel killed it.
[kernel] Loading app_2
3^10000=5079(MOD 10007)
3^20000=8202(MOD 10007)
3^30000=8824(MOD 10007)
3^40000=5750(MOD 10007)
3^50000=3824(MOD 10007)
3^60000=8516(MOD 10007)
3^70000=2510(MOD 10007)
3^80000=9379(MOD 10007)
3^90000=2621(MOD 10007)
3^100000=2749(MOD 10007)
Test power OK!
[kernel] Application exited with code 0
[kernel] Loading app_3
Try to execute privileged instruction in U Mode
Kernel should kill this application!
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] Loading app_4
Try to access privileged CSR in U Mode
Kernel should kill this application!
[kernel] IllegalInstruction in application, kernel killed it.
All applications completed!
```

2.  深入理解 [trap.S](https://github.com/LearningOS/rCore-Camp-Code-2024A/blob/ch3/os/src/trap/trap.S) 中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:
    
    1.  L40：刚进入 `__restore` 时，`a0` 代表了什么值。请指出 `__restore` 的两种使用情景。
        刚刚进入`__restore`的时候是`a0`是我们传入`__restore`的参数,我们在`run_next_app`中调用了这个函数(函数代码省略)
		可以看到传入的是我们主动制造的`TrapContext`的指针,`TrapContext`内容是APP加载位置`APP_BASE_ADDRESS`和用户栈的指针.
		它被调用的情景分为两种:
		1.  一个APP运行结束或者出错之后的APP切换
		2.  在内核工作之后开始APP的加载和运行
    2.  L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
        
        ld t0, 32*8(sp)
        ld t1, 33*8(sp)
        ld t2, 2*8(sp)
        csrw sstatus, t0
        csrw sepc, t1
        csrw sscratch, t2
        
        这一部分是先把存在栈中的内容转移到`t0~t2`,然后再把它们还原回`sstatus`,`sepc`和`sscratch`.
		这里不能直接移动的原因是`ld`可以操作寄存器的指针偏移,而`csrw`只能操作寄存器和寄存器.
		
    3.  L50-L56：为何跳过了 `x2` 和 `x4`？
        
        ld x1, 1*8(sp)
        ld x3, 3*8(sp)
        .set n, 5
        .rept 27
           LOAD_GP %n
           .set n, n+1
        .endr
        [官方文档](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter2/4trap-handling.html#id7):
		1.  但这里也有一些例外，如 `x0` 被硬编码为 0 ，它自然不会有变化；还有 `tp(x4)` 寄存器，除非我们手动出于一些特殊用途使用它，否则一般也不会被用到。
		2.  我们在这里也不保存 sp(x2)，因为我们要基于它来找到每个寄存器应该被保存到的正确的位置。
    4.  L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？
        
        csrrw sp, sscratch, sp
        此指令后,sp->user stack sscratch->kernel stack,因此`sp`重新指向用户栈.
    5.  `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
        发生状态切换是在`sret`.
		硬件上是这个原因,调用它之后寄存器会发生变化.
    6.  L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？
        
        csrrw sp, sscratch, sp
        这个指令之后就把`sp`和`sscratch`切换了,就会导致sp->kernel stack, sscratch->user stack.
    7.  从 U 态进入 S 态是哪一条指令发生的？
		应该是再用户态的`ecall`指令发生的.
		
# **荣誉准则**

1.  在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
    >
    > 本章暂无
    >
2.  此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
	>[Rust语言圣经(Rust Course)](https://course.rs/about-book.html)
	>
	[简介 - The Little Book of Rust Macros （Rust 宏小册） (zjp-cn.github.io)](https://zjp-cn.github.io/tlborm/)
	>
	[rustsbi - Rust (docs.rs)](https://docs.rs/rustsbi/latest/rustsbi/)
	>
	[Introduction - The Cargo Book (rust-lang.org)](https://doc.rust-lang.org/cargo/index.html)
	>
	[RISC-V手册 (ustc.edu.cn)](http://staff.ustc.edu.cn/~llxx/cod/reference_books/RISC-V-Reader-Chinese-v2p12017.pdf) 
	>
	[GDB Documentation (sourceware.org)](https://sourceware.org/gdb/documentation/)
	>
	[Summit_bootflow (riscv.org)](https://riscv.org/wp-content/uploads/2019/12/Summit_bootflow.pdf)
	>
	《[RISC-V开放架构设计之道 The RISC-V Reader.pdf](https://crva.ict.ac.cn/wjxz/202311/P020231213600105558154.pdf)》
	>
	[RISC-V Specification for generic_rv64 :: RISC-V Specification for generic_rv64 (riscv-software-src.github.io)](https://riscv-software-src.github.io/riscv-unified-db/html/generic_rv64/landing.html)
	>
	[The RISC-V Instruction Set Manual Volume II: Privileged Architecture](https://www2.eecs.berkeley.edu/Pubs/TechRpts/2016/EECS-2016-161.pdf)
	>
	[Makefile教程和示例指南 (foofun.cn)](http://makefiletutorial.foofun.cn/)
	>
	[RISCV 特权级拾遗_riscv mret-CSDN博客](https://blog.csdn.net/qq_42556934/article/details/124452742)
    >

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
