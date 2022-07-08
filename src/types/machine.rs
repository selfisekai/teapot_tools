use std::fmt;

use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GclientOS {
    /// Unix or Linux, except macOS, iOS, Android
    Unix,
    /// Windows
    Win,
    /// macOS
    Mac,
    /// iOS
    IOS,
    Android,
    ChromeOS,
    Fuchsia,
    /// used to specify that all OS stuff should be checkout
    All,
}

impl fmt::Display for GclientOS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GclientOS::Unix => "unix",
                GclientOS::Win => "win",
                GclientOS::Mac => "mac",
                GclientOS::IOS => "ios",
                GclientOS::Android => "android",
                GclientOS::ChromeOS => "chromeos",
                GclientOS::Fuchsia => "fuchsia",
                GclientOS::All => "all",
            }
        )
    }
}

// keep in sync with GclientOS
pub const OS_LIST: [GclientOS; 8] = [
    GclientOS::Unix,
    GclientOS::Win,
    GclientOS::Mac,
    GclientOS::IOS,
    GclientOS::Android,
    GclientOS::ChromeOS,
    GclientOS::Fuchsia,
    GclientOS::All,
];

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GclientCPU {
    /// x86_64, amd64
    X64,
    /// i686, x86
    X86,
    /// aarch64, arm64
    Arm64,
    /// arm 32-bit
    Arm,
    /// mips 32-bit
    Mips,
    /// mips64
    Mips64,
    /// powerpc 32-bit
    Ppc,
    /// powerpc64
    Ppc64,
    /// riscv64
    Riscv64,
    /// s390
    S390,
    /// s390x
    S390x,
}

impl fmt::Display for GclientCPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GclientCPU::X64 => "x64",
                GclientCPU::X86 => "x86",
                GclientCPU::Arm64 => "arm64",
                GclientCPU::Arm => "arm",
                GclientCPU::Mips => "mips",
                GclientCPU::Mips64 => "mips64",
                GclientCPU::Ppc => "ppc",
                GclientCPU::Ppc64 => "ppc64",
                GclientCPU::Riscv64 => "riscv64",
                GclientCPU::S390 => "s390",
                GclientCPU::S390x => "s390x",
            }
        )
    }
}
