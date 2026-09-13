#![doc(html_root_url = "https://docs.rs/guess_host_triple/0.1.5")]
#![warn(missing_docs)]
//! Guess which Rust-supported platform is running the current code.
//!
//! Introduction
//! ============
//!
//! In the world of compilers,
//! for both Rust and other languages,
//! the "host" is the computer where the compiler runs
//! and produces an executable,
//! and the "target" is the computer where the resulting executable runs.
//! Usually they are the same,
//! but they don't have to be;
//! for example,
//! you can use a powerful desktop computer
//! to compile code for a tiny microcontroller
//! that wouldn't have the resources to compile a program on its own.
//!
//! For compilers that can target many different kinds of computers,
//! each target is given a unique name
//! according to [the "target triple" convention][ttc].
//! If we describe the computer that the compiler runs on with a "triple",
//! that's a "host triple".
//!
//! Before you can install a copy of the Rust compiler on a given computer,
//! you need to know the host triple Rust uses to describe computer,
//! so you can choose a compatible one.
//! This crate provides a function that guesses the host triple
//! for the computer that it runs on,
//! which you can look up in the [Rust release channel metadata][rcm]
//! to find the URLs of the packages you'll need to download.
//!
//! The code in this crate was originally extracted
//! from the official rustup tool.
//!
//! [ttc]: https://wiki.osdev.org/Target_Triplet
//! [rcm]: https://docs.rs/rust_release_channel/
//!
//! Example
//! =======
//!
//!     extern crate guess_host_triple;
//!
//!     guess_host_triple::guess_host_triple()
//!         .map(|triple| println!("This computer's triple: {}", triple))
//!         .unwrap_or_else(|| println!("Can't guess this computer's triple"));
//!
//!     // Prints something like:
//!     //
//!     // This computer's triple: x86_64-unknown-linux-gnu
//!
#[cfg(unix)]
extern crate libc;
#[cfg(windows)]
extern crate winapi;
#[macro_use]
extern crate log;
extern crate errno;

use errno::errno;
use std::{ffi::CStr, os::raw::c_char};

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
enum Architecture {
    Unknown,
    I686,
    X86_64,
    Arm,
    ArmV7,
    ArmV8,
    Aarch64,
    Mips,
    Mips64,
    PowerPc,
    PowerPc64,
}

impl Architecture {
    #[cfg(unix)]
    fn from_uname_machine(machine: &CStr) -> Architecture {
        use Architecture::*;

        match machine.to_bytes() {
            b"amd64" => X86_64,
            b"arm" => Arm,
            b"armv7l" => ArmV7,
            b"armv8l" => ArmV8,
            b"aarch64" => Aarch64,
            b"arm64" => Aarch64,
            b"i686" => I686,
            b"x86_64" => X86_64,
            b"mips" => Mips,
            b"mips64" => Mips64,
            b"powerpc" => PowerPc,
            b"powerpc64" => PowerPc64,
            _ => {
                warn!("uname returned unrecognised machine {machine:?}");
                Unknown
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
enum Endianness {
    Unknown,
    Big,
    Little,
}

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
enum Os {
    Unknown,
    Windows,
    Android,
    Linux,
    Darwin,
    FreeBsd,
    OpenBsd,
    NetBsd,
    DragonFly,
}

impl Os {
    #[cfg(unix)]
    fn from_uname_sysname(sysname: &CStr) -> Os {
        use Os::*;

        match sysname.to_bytes() {
            b"Linux" => Linux,
            b"Darwin" => Darwin,
            b"FreeBSD" => FreeBsd,
            b"OpenBSD" => OpenBsd,
            b"NetBSD" => NetBsd,
            b"DragonFly" => DragonFly,
            _ => {
                warn!("uname returned unrecognised sysname {sysname:?}");
                Unknown
            }
        }
    }
}

trait Environment {
    fn architecture(&self) -> Architecture;
    fn endianness(&self) -> Endianness;
    fn os(&self) -> Os;
}

fn guess_triple_from_environment<E: Environment>(
    env: &E,
) -> Option<&'static str> {
    use Architecture::*;
    use Endianness::*;
    use Os::*;

    match (env.architecture(), env.endianness(), env.os()) {
        (I686, _, Windows) => Some("i686-pc-windows-msvc"),
        (X86_64, _, Windows) => Some("x86_64-pc-windows-msvc"),
        (Aarch64, _, Windows) => Some("aarch64-pc-windows-msvc"),
        (Arm, _, Android) => Some("arm-linux-androideabi"),
        (ArmV7, _, Android) => Some("armv7-linux-androideabi"),
        (ArmV8, _, Android) => Some("armv7-linux-androideabi"),
        (Aarch64, _, Android) => Some("aarch64-linux-android"),
        (I686, _, Android) => Some("i686-linux-android"),
        (X86_64, _, Android) => Some("x86_64-linux-android"),
        (I686, _, Linux) => Some("i686-unknown-linux-gnu"),
        (X86_64, _, Linux) => Some("x86_64-unknown-linux-gnu"),
        (Mips, Big, Linux) => Some("mips-unknown-linux-gnu"),
        (Mips, Little, Linux) => Some("mipsel-unknown-linux-gnu"),
        (Mips64, Big, Linux) => Some("mips64-unknown-linux-gnuabi64"),
        (Mips64, Little, Linux) => Some("mips64el-unknown-linux-gnuabi64"),
        (Arm, _, Linux) => Some("arm-unknown-linux-gnueabi"),
        (ArmV7, _, Linux) => Some("armv7-unknown-linux-gnueabihf"),
        (ArmV8, _, Linux) => Some("armv7-unknown-linux-gnueabihf"),
        (Aarch64, _, Linux) => Some("aarch64-unknown-linux-gnu"),
        (I686, _, Darwin) => Some("i686-apple-darwin"),
        (X86_64, _, Darwin) => Some("x86_64-apple-darwin"),
        (Aarch64, _, Darwin) => Some("aarch64-apple-darwin"),
        (I686, _, FreeBsd) => Some("i686-unknown-freebsd"),
        (X86_64, _, FreeBsd) => Some("x86_64-unknown-freebsd"),
        (I686, _, OpenBsd) => Some("i686-unknown-openbsd"),
        (X86_64, _, OpenBsd) => Some("x86_64-unknown-openbsd"),
        (I686, _, NetBsd) => Some("i686-unknown-netbsd"),
        (X86_64, _, NetBsd) => Some("x86_64-unknown-netbsd"),
        (X86_64, _, DragonFly) => Some("x86_64-unknown-dragonfly"),
        other => {
            warn!("Could not guess triple for {other:?}");
            None
        }
    }
}

#[derive(Copy, Clone)]
struct HostEnvironment {
    #[cfg(unix)]
    maybe_uname_info: Option<libc::utsname>,
}

impl HostEnvironment {
    #[cfg(unix)]
    fn new() -> HostEnvironment {
        use std::mem::MaybeUninit;

        // Make space to store maybe_uname_info.
        let mut uname_info = MaybeUninit::uninit();

        // Try and load uname_info.
        // Safety: libc::uname return a non-negative value upon successful
        // completion, -1 otherwise.
        if unsafe { libc::uname(uname_info.as_mut_ptr()) } >= 0 {
            // Safety: libc::uname returned successfully
            let uname_info = unsafe { uname_info.assume_init() };
            HostEnvironment {
                maybe_uname_info: Some(uname_info),
            }
        } else {
            warn!("Could not get uname data: {}", errno());

            HostEnvironment {
                maybe_uname_info: None,
            }
        }
    }

    #[cfg(not(unix))]
    fn new() -> HostEnvironment {
        HostEnvironment {}
    }
}

impl Environment for HostEnvironment {
    #[cfg(windows)]
    fn architecture(&self) -> Architecture {
        use std::mem::MaybeUninit;
        use winapi::um::sysinfoapi::GetNativeSystemInfo;
        use winapi::um::winnt;
        use Architecture::*;

        let mut sys_info = MaybeUninit::uninit();

        // Safety: GetSystemInfo takes a pointer to a SYSTEM_INFO structure,
        // returns void.
        unsafe {
            GetNativeSystemInfo(sys_info.as_mut_ptr());
        }

        // Safety: GetNativeSystemInfo does not seem like it can fail, therefore
        // sys_info must be valid at this point.
        let sys_info = unsafe { sys_info.assume_init() };

        // Safety: the structure containing wProcessorArchitecture is the only
        // member of the union that can be used, therefore it is the
        // only possible active member.
        match unsafe { sys_info.u.s() }.wProcessorArchitecture {
            winnt::PROCESSOR_ARCHITECTURE_AMD64 => X86_64,
            winnt::PROCESSOR_ARCHITECTURE_INTEL => I686,
            winnt::PROCESSOR_ARCHITECTURE_ARM64 => Aarch64,
            _ => return Unknown,
        }
    }

    #[cfg(unix)]
    fn architecture(&self) -> Architecture {
        self.maybe_uname_info
            .and_then(|uname_info| {
                let machine = match c_str_from_c_char_slice(&uname_info.machine)
                {
                    machine @ Some(_) => machine,
                    None => {
                        warn!(
                            "uname() returned non-terminated string ignoring"
                        );
                        None
                    }
                };

                Some(Architecture::from_uname_machine(machine?))
            })
            .unwrap_or(Architecture::Unknown)
    }

    #[cfg(target_endian = "little")]
    fn endianness(&self) -> Endianness {
        Endianness::Little
    }

    #[cfg(target_endian = "big")]
    fn endianness(&self) -> Endianness {
        Endianness::Big
    }

    #[cfg(not(any(target_endian = "little", target_endian = "big")))]
    fn endianness(&self) -> Endianness {
        Endianness::Unknown
    }

    #[cfg(windows)]
    fn os(&self) -> Os {
        Os::Windows
    }

    #[cfg(target_os = "android")]
    fn os(&self) -> Os {
        Os::Android
    }

    #[cfg(all(unix, not(target_os = "android")))]
    fn os(&self) -> Os {
        self.maybe_uname_info
            .and_then(|uname_info| {
                let sysname = match c_str_from_c_char_slice(&uname_info.sysname)
                {
                    sysname @ Some(_) => sysname,
                    None => {
                        warn!(
                            "uname() returned non-terminated string ignoring"
                        );
                        None
                    }
                };

                Some(Os::from_uname_sysname(sysname?))
            })
            .unwrap_or(Os::Unknown)
    }
}

#[cfg(unix)]
fn c_str_from_c_char_slice(chars: &[c_char]) -> Option<&CStr> {
    // Safety: same conversion performed in `CStr::to_bytes_with_nul`
    let slice = unsafe { &*{ chars as *const [c_char] as *const [u8] } };

    // CStr::from_bytes_with_nul works only if there is only one '\0'
    // terminator, trailing terminators are not allowed.
    slice.into_iter().position(|&c| c == b'\0').map(|pos| {
        // Safety: we already know that there is a null terminator at pos
        unsafe { CStr::from_bytes_with_nul_unchecked(&slice[..pos + 1]) }
    })
}

/// Guess which Rust-supported platform is running the current code.
///
/// This function may return `None`
/// if it cannot obtain details about the current platform,
/// or if it does not recognise them.
///
/// Problems that occur during platform detection
/// are logged via the `log` crate.
///
/// Example
/// =======
///
/// See [the top-level example](index.html#example).
pub fn guess_host_triple() -> Option<&'static str> {
    guess_triple_from_environment(&HostEnvironment::new())
}

#[cfg(test)]
mod test {
    extern crate libc;
    use std::ffi::CString;

    #[cfg(unix)]
    #[test]
    fn os_from_sysname() {
        fn assert_sysname_lookup(text: &str, expected: super::Os) {
            assert_eq!(
                super::Os::from_uname_sysname(&CString::new(text).unwrap()),
                expected,
            );
        }

        assert_sysname_lookup("Linux", super::Os::Linux);
        assert_sysname_lookup("Darwin", super::Os::Darwin);
        assert_sysname_lookup("FreeBSD", super::Os::FreeBsd);
        assert_sysname_lookup("OpenBSD", super::Os::OpenBsd);
        assert_sysname_lookup("NetBSD", super::Os::NetBsd);
        assert_sysname_lookup("DragonFly", super::Os::DragonFly);
        assert_sysname_lookup("blorp", super::Os::Unknown);
    }

    #[cfg(unix)]
    #[test]
    fn architecture_from_machine() {
        fn assert_machine_lookup(text: &str, expected: super::Architecture) {
            assert_eq!(
                super::Architecture::from_uname_machine(
                    &CString::new(text).unwrap()
                ),
                expected,
            );
        }

        assert_machine_lookup("arm", super::Architecture::Arm);
        assert_machine_lookup("armv7l", super::Architecture::ArmV7);
        assert_machine_lookup("armv8l", super::Architecture::ArmV8);
        assert_machine_lookup("aarch64", super::Architecture::Aarch64);
        assert_machine_lookup("i686", super::Architecture::I686);
        assert_machine_lookup("x86_64", super::Architecture::X86_64);
        assert_machine_lookup("mips", super::Architecture::Mips);
        assert_machine_lookup("mips64", super::Architecture::Mips64);
        assert_machine_lookup("powerpc", super::Architecture::PowerPc);
        assert_machine_lookup("powerpc64", super::Architecture::PowerPc64);
        assert_machine_lookup("blorp", super::Architecture::Unknown);
    }

    #[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
    struct StubEnvironment {
        architecture: super::Architecture,
        endianness: super::Endianness,
        os: super::Os,
    }

    impl super::Environment for StubEnvironment {
        fn architecture(&self) -> super::Architecture {
            self.architecture
        }
        fn endianness(&self) -> super::Endianness {
            self.endianness
        }
        fn os(&self) -> super::Os {
            self.os
        }
    }

    #[test]
    fn windows_x86() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::I686,
                endianness: super::Endianness::Little,
                os: super::Os::Windows,
            }),
            // Default to MSVC.
            Some("i686-pc-windows-msvc"),
        );
    }

    #[test]
    fn windows_x86_64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::X86_64,
                endianness: super::Endianness::Little,
                os: super::Os::Windows,
            }),
            // Default to MSVC.
            Some("x86_64-pc-windows-msvc"),
        );
    }

    #[test]
    fn android_arm() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Arm,
                endianness: super::Endianness::Little,
                os: super::Os::Android,
            }),
            Some("arm-linux-androideabi"),
        );
    }

    #[test]
    fn android_armv7() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::ArmV7,
                endianness: super::Endianness::Little,
                os: super::Os::Android,
            }),
            Some("armv7-linux-androideabi"),
        );
    }

    #[test]
    fn android_armv8() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::ArmV8,
                endianness: super::Endianness::Little,
                os: super::Os::Android,
            }),
            // ARMv8 is backwards compatible
            Some("armv7-linux-androideabi"),
        );
    }

    #[test]
    fn android_aarch64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Aarch64,
                endianness: super::Endianness::Little,
                os: super::Os::Android,
            }),
            Some("aarch64-linux-android"),
        );
    }

    #[test]
    fn android_i686() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::I686,
                endianness: super::Endianness::Little,
                os: super::Os::Android,
            }),
            Some("i686-linux-android"),
        );
    }

    #[test]
    fn android_x86_64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::X86_64,
                endianness: super::Endianness::Little,
                os: super::Os::Android,
            }),
            Some("x86_64-linux-android"),
        );
    }

    #[test]
    fn linux_x86_64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::X86_64,
                endianness: super::Endianness::Little,
                os: super::Os::Linux,
            }),
            Some("x86_64-unknown-linux-gnu"),
        );
    }

    #[test]
    fn linux_i686() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::I686,
                endianness: super::Endianness::Little,
                os: super::Os::Linux,
            }),
            Some("i686-unknown-linux-gnu"),
        );
    }

    #[test]
    fn linux_mipsel() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Mips,
                endianness: super::Endianness::Little,
                os: super::Os::Linux,
            }),
            Some("mipsel-unknown-linux-gnu"),
        );
    }

    #[test]
    fn linux_mips() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Mips,
                endianness: super::Endianness::Big,
                os: super::Os::Linux,
            }),
            Some("mips-unknown-linux-gnu"),
        );
    }

    #[test]
    fn linux_mips64el() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Mips64,
                endianness: super::Endianness::Little,
                os: super::Os::Linux,
            }),
            Some("mips64el-unknown-linux-gnuabi64"),
        );
    }

    #[test]
    fn linux_mips64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Mips64,
                endianness: super::Endianness::Big,
                os: super::Os::Linux,
            }),
            Some("mips64-unknown-linux-gnuabi64"),
        );
    }

    #[test]
    fn linux_arm() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Arm,
                endianness: super::Endianness::Little,
                os: super::Os::Linux,
            }),
            Some("arm-unknown-linux-gnueabi"),
        );
    }

    #[test]
    fn linux_armv7l() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::ArmV7,
                endianness: super::Endianness::Little,
                os: super::Os::Linux,
            }),
            Some("armv7-unknown-linux-gnueabihf"),
        );
    }

    #[test]
    fn linux_armv8l() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::ArmV8,
                endianness: super::Endianness::Little,
                os: super::Os::Linux,
            }),
            // ARMv8 is backwards compatible
            Some("armv7-unknown-linux-gnueabihf"),
        );
    }

    #[test]
    fn linux_aarch64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Aarch64,
                endianness: super::Endianness::Little,
                os: super::Os::Linux,
            }),
            Some("aarch64-unknown-linux-gnu"),
        );
    }

    #[test]
    fn darwin_x86_64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::X86_64,
                endianness: super::Endianness::Little,
                os: super::Os::Darwin,
            }),
            Some("x86_64-apple-darwin"),
        );
    }

    #[test]
    fn darwin_aarch64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::Aarch64,
                endianness: super::Endianness::Little,
                os: super::Os::Darwin,
            }),
            Some("aarch64-apple-darwin"),
        );
    }

    #[test]
    fn darwin_i686() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::I686,
                endianness: super::Endianness::Little,
                os: super::Os::Darwin,
            }),
            Some("i686-apple-darwin"),
        );
    }

    #[test]
    fn freebsd_x86_64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::X86_64,
                endianness: super::Endianness::Little,
                os: super::Os::FreeBsd,
            }),
            Some("x86_64-unknown-freebsd"),
        );
    }

    #[test]
    fn freebsd_i686() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::I686,
                endianness: super::Endianness::Little,
                os: super::Os::FreeBsd,
            }),
            Some("i686-unknown-freebsd"),
        );
    }

    #[test]
    fn openbsd_x86_64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::X86_64,
                endianness: super::Endianness::Little,
                os: super::Os::OpenBsd,
            }),
            Some("x86_64-unknown-openbsd"),
        );
    }

    #[test]
    fn openbsd_i686() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::I686,
                endianness: super::Endianness::Little,
                os: super::Os::OpenBsd,
            }),
            Some("i686-unknown-openbsd"),
        );
    }

    #[test]
    fn netbsd_x86_64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::X86_64,
                endianness: super::Endianness::Little,
                os: super::Os::NetBsd,
            }),
            Some("x86_64-unknown-netbsd"),
        );
    }

    #[test]
    fn netbsd_i686() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::I686,
                endianness: super::Endianness::Little,
                os: super::Os::NetBsd,
            }),
            Some("i686-unknown-netbsd"),
        );
    }

    #[test]
    fn dragonfly_x86_64() {
        assert_eq!(
            super::guess_triple_from_environment(&StubEnvironment {
                architecture: super::Architecture::X86_64,
                endianness: super::Endianness::Little,
                os: super::Os::DragonFly,
            }),
            Some("x86_64-unknown-dragonfly"),
        );
    }

    #[test]
    fn native_platform_is_supported() {
        assert!(super::guess_host_triple().is_some())
    }
}
