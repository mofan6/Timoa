#![doc = include_str!("../readme.md")]
#![no_std]

/// Defines an external function to import.
#[cfg(all(windows, target_arch = "x86"))]
#[macro_export]
macro_rules! link_raw {
    ($library:literal $abi:literal $($link_name:literal)? fn $($function:tt)*) => (
        #[link(name = $library, kind = "raw-dylib", modifiers = "+verbatim", import_name_type = "undecorated")]
        extern $abi {
            $(#[link_name=$link_name])?
            pub fn $($function)*;
        }
    )
}

/// Defines an external function to import.
#[cfg(all(windows, not(target_arch = "x86")))]
#[macro_export]
macro_rules! link_raw {
    ($library:literal $abi:literal $($link_name:literal)? fn $($function:tt)*) => (
        #[link(name = $library, kind = "raw-dylib", modifiers = "+verbatim")]
        extern $abi {
            $(#[link_name=$link_name])?
            pub fn $($function)*;
        }
    )
}

/// Defines an external function to import.
#[cfg(not(windows))]
#[macro_export]
macro_rules! link_raw {
    ($library:literal $abi:literal $($link_name:literal)? fn $($function:tt)*) => (
        extern $abi {
            pub fn $($function)*;
        }
    )
}

// Windows 7 exports these COM functions from ole32.dll. Newer Windows also
// forwards them through combase.dll, which does not exist on Windows 7.
#[macro_export]
macro_rules! link {
    ("combase.dll" $abi:literal fn CoTaskMemAlloc $($rest:tt)*) => {
        #[cfg(target_vendor = "win7")]
        $crate::link_raw!("ole32.dll" $abi fn CoTaskMemAlloc $($rest)*);
        #[cfg(not(target_vendor = "win7"))]
        $crate::link_raw!("combase.dll" $abi fn CoTaskMemAlloc $($rest)*);
    };
    ("combase.dll" $abi:literal fn CoTaskMemFree $($rest:tt)*) => {
        #[cfg(target_vendor = "win7")]
        $crate::link_raw!("ole32.dll" $abi fn CoTaskMemFree $($rest)*);
        #[cfg(not(target_vendor = "win7"))]
        $crate::link_raw!("combase.dll" $abi fn CoTaskMemFree $($rest)*);
    };
    ("combase.dll" $abi:literal fn CoCreateFreeThreadedMarshaler $($rest:tt)*) => {
        #[cfg(target_vendor = "win7")]
        $crate::link_raw!("ole32.dll" $abi fn CoCreateFreeThreadedMarshaler $($rest)*);
        #[cfg(not(target_vendor = "win7"))]
        $crate::link_raw!("combase.dll" $abi fn CoCreateFreeThreadedMarshaler $($rest)*);
    };
    ($($rest:tt)*) => { $crate::link_raw!($($rest)*); };
}
