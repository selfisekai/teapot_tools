use crate::types::machine::{GclientCPU, GclientOS};

pub fn gclient_host_os() -> GclientOS {
    #[cfg(target_os = "linux")]
    return GclientOS::Unix;

    #[cfg(target_os = "macos")]
    return GclientOS::Mac;

    #[cfg(windows)]
    return GclientOS::Win;
}

pub fn gclient_host_cpu() -> GclientCPU {
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

pub fn cipd_host_os() -> String {
    #[cfg(target_os = "linux")]
    return "linux".to_string();

    #[cfg(target_os = "macos")]
    return "mac".to_string();

    #[cfg(windows)]
    return "windows".to_string();
}

pub fn cipd_host_cpu() -> String {
    // architecture naming scheme of "fuck around and find out"

    #[cfg(target_arch = "x86_64")]
    return "amd64".to_string();

    #[cfg(target_arch = "x86")]
    return "386".to_string();

    #[cfg(target_arch = "aarch64")]
    return "arm64".to_string();

    #[cfg(target_arch = "arm")]
    return "arm".to_string();

    #[cfg(target_arch = "riscv64")]
    return "riscv64".to_string();

    // starting here, the repository itself either doesn't have
    // any packages for these architectures, or doesn't even agree
    // on their names. it doesn't make any sense.
    //
    // and most probably they don't have your packages anyway.

    #[cfg(target_arch = "mips")]
    return "mips32".to_string();

    #[cfg(target_arch = "mips64")]
    return "mips64".to_string();

    #[cfg(target_arch = "powerpc")]
    return "ppc".to_string();

    #[cfg(target_arch = "powerpc64")]
    return "ppc64".to_string();

    #[cfg(target_arch = "s390")]
    return "s390".to_string();

    #[cfg(target_arch = "s390x")]
    return "s390x".to_string();
}
