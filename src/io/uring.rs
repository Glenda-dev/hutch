use glenda::io::uring::{
    IOURING_OP_READ, IOURING_OP_SYNC, IOURING_OP_WRITE, IoUringCqe, IoUringLayout, IoUringSqe,
};
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::sync::atomic::Ordering;

pub struct UringEmulator {
    base_ptr: *mut u8,
    file: Option<File>,
}

impl UringEmulator {
    pub fn new(base_ptr: *mut u8) -> Self {
        Self { base_ptr, file: None }
    }

    pub fn set_file(&mut self, file: File) {
        self.file = Some(file);
    }

    /// 模拟处理 SQE 并产生 CQE
    pub fn process_requests(&mut self) -> io::Result<usize> {
        if self.base_ptr.is_null() {
            return Ok(0);
        }
        let layout = unsafe { &*(self.base_ptr as *const IoUringLayout) };
        let mut sq_head = layout.sq_head.load(Ordering::Acquire);
        let sq_tail = layout.sq_tail.load(Ordering::Acquire);

        let mut processed = 0;

        while sq_head != sq_tail {
            let idx = (sq_head & layout.sq_mask) as usize;
            let sqe_ptr = unsafe {
                self.base_ptr.add(std::mem::size_of::<IoUringLayout>()) as *const IoUringSqe
            }
            .wrapping_add(idx);

            let sqe = unsafe { &*sqe_ptr };
            let res = self.handle_sqe(sqe);

            self.push_cqe(sqe.user_data, res);

            sq_head += 1;
            processed += 1;
        }

        layout.sq_head.store(sq_head, Ordering::Release);
        Ok(processed)
    }

    fn handle_sqe(&self, sqe: &IoUringSqe) -> i32 {
        let file = match &self.file {
            Some(f) => f,
            None => return -libc::EBADF,
        };

        match sqe.opcode {
            IOURING_OP_READ => {
                // 在模拟环境中，addr 通常是进程内的虚拟地址
                // Hutch 运行在 host，我们需要确保 addr 在 host 是可访问的
                // 简化起见，这里假设 addr 是合法的 host 指针 (由 hosted 模式下的 libglenda 分配)
                let buf = unsafe {
                    std::slice::from_raw_parts_mut(sqe.addr as *mut u8, sqe.len as usize)
                };
                match file.read_at(buf, sqe.off as u64) {
                    Ok(n) => n as i32,
                    Err(e) => -(e.raw_os_error().unwrap_or(libc::EIO)),
                }
            }
            IOURING_OP_WRITE => {
                let buf =
                    unsafe { std::slice::from_raw_parts(sqe.addr as *const u8, sqe.len as usize) };
                match file.write_at(buf, sqe.off as u64) {
                    Ok(n) => n as i32,
                    Err(e) => -(e.raw_os_error().unwrap_or(libc::EIO)),
                }
            }
            IOURING_OP_SYNC => match file.sync_all() {
                Ok(_) => 0,
                Err(e) => -(e.raw_os_error().unwrap_or(libc::EIO)),
            },
            _ => -libc::EINVAL,
        }
    }

    fn push_cqe(&self, user_data: usize, res: i32) {
        let layout = unsafe { &*(self.base_ptr as *const IoUringLayout) };
        let cq_head = layout.cq_head.load(Ordering::Acquire);
        let cq_tail = layout.cq_tail.load(Ordering::Acquire);

        if cq_tail - cq_head >= layout.cq_entries {
            // CQ 溢出，简单忽略或在真实场景中记录
            return;
        }

        let idx = (cq_tail & layout.cq_mask) as usize;
        let cqe_offset = std::mem::size_of::<IoUringLayout>()
            + (layout.sq_entries as usize * std::mem::size_of::<IoUringSqe>());
        let cqe_ptr = unsafe { self.base_ptr.add(cqe_offset) as *mut IoUringCqe }.wrapping_add(idx);

        unsafe {
            (*cqe_ptr).user_data = user_data;
            (*cqe_ptr).res = res;
            (*cqe_ptr).flags = 0;
        }

        layout.cq_tail.store(cq_tail + 1, Ordering::Release);
    }
}

unsafe impl Send for UringEmulator {}
unsafe impl Sync for UringEmulator {}
