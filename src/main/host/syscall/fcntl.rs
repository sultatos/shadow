use crate::cshadow;
use crate::host::context::{ThreadContext, ThreadContextObjs};
use crate::host::descriptor::{CompatDescriptor, FileFlags};
use crate::host::syscall;
use crate::host::syscall_types::SyscallResult;
use crate::host::syscall_types::{SysCallArgs, SysCallReg};
use log::*;
use nix::errno::Errno;
use std::os::unix::prelude::RawFd;

fn fcntl(ctx: &mut ThreadContext, args: &SysCallArgs) -> SyscallResult {
    let fd: RawFd = args.args[0].into();
    let cmd: i32 = args.args[1].into();

    // get the descriptor, or return early if it doesn't exist
    let desc = unsafe { &*syscall::get_descriptor(fd, ctx.process.raw_mut())? };

    // if it's a legacy descriptor, use the C syscall handler instead
    let desc = match desc {
        CompatDescriptor::New(d) => d,
        CompatDescriptor::Legacy(_) => {
            return unsafe {
                cshadow::syscallhandler_fcntl(
                    ctx.thread.csyscallhandler(),
                    args as *const cshadow::SysCallArgs,
                )
            }
            .into()
        }
    };

    Ok(match cmd {
        libc::F_GETFL => {
            let flags = desc.get_file().borrow().get_flags();
            SysCallReg::from(flags.bits())
        }
        libc::F_SETFL => {
            let flags = FileFlags::from_bits(i32::from(args.args[2])).ok_or(Errno::EINVAL)?;
            desc.get_file().borrow_mut().set_flags(flags);
            SysCallReg::from(0)
        }
        _ => Err(Errno::EINVAL)?,
    })
}

mod export {
    use super::*;
    use crate::utility::notnull::notnull_mut_debug;

    #[no_mangle]
    pub extern "C" fn rustsyscallhandler_fcntl(
        sys: *mut cshadow::SysCallHandler,
        args: *const cshadow::SysCallArgs,
    ) -> cshadow::SysCallReturn {
        let mut objs = unsafe { ThreadContextObjs::from_syscallhandler(notnull_mut_debug(sys)) };
        fcntl(&mut objs.borrow(), unsafe { args.as_ref().unwrap() }).into()
    }

    #[no_mangle]
    pub extern "C" fn rustsyscallhandler_fcntl64(
        sys: *mut cshadow::SysCallHandler,
        args: *const cshadow::SysCallArgs,
    ) -> cshadow::SysCallReturn {
        // Our fcntl supports the flock64 struct when any of the F_GETLK64, F_SETLK64, and F_SETLKW64
        // commands are specified, so we can just use our fcntl handler directly.
        trace!("fcntl64 called, forwarding to fcntl handler");
        rustsyscallhandler_fcntl(sys, args)
    }
}