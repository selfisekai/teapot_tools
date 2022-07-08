use crate::types::machine::{GclientCPU, GclientOS};

pub fn host_os() -> GclientOS {
    #[cfg(target_os = "linux")]
    return GclientOS::Unix;

    #[cfg(target_os = "macos")]
    return GclientOS::Mac;

    #[cfg(windows)]
    return GclientOS::Win;
}

pub fn host_cpu() -> GclientCPU {
    // depot_tools arch reference: https://chromium.googlesource.com/chromium/tools/depot_tools.git/+/refs/heads/main/detect_host_arch.py

    #[cfg(target_arch = "x86_64")]
    return GclientCPU::X64;

    #[cfg(target_arch = "x86")]
    return GclientCPU::X86;

    #[cfg(target_arch = "aarch64")]
    return GclientCPU::Arm64;

    #[cfg(target_arch = "arm")]
    return GclientCPU::Arm;

    #[cfg(target_arch = "mips")]
    return GclientCPU::Mips;

    #[cfg(target_arch = "mips64")]
    return GclientCPU::Mips64;

    #[cfg(target_arch = "powerpc")]
    return GclientCPU::Ppc;

    #[cfg(target_arch = "powerpc64")]
    return GclientCPU::Ppc64;

    #[cfg(target_arch = "riscv64")]
    return GclientCPU::Riscv64;

    #[cfg(target_arch = "s390")]
    return GclientCPU::S390;

    #[cfg(target_arch = "s390x")]
    return GclientCPU::S390x;
}
