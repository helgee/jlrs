//! The main goal behind jlrs is to provide a simple and safe interface to the Julia C API that
//! lets you call code written in Julia from Rust and vice versa. Currently this crate is only
//! tested on Linux and Windows in combination with Julia 1.5 and is not compatible with earlier
//! versions of Julia.
//!
//!
//! # Features
//!
//! An incomplete list of features that are currently supported by jlrs:
//!
//!  - Access arbitrary Julia modules and their contents.
//!  - Call arbitrary Julia functions, including functions that take keyword arguments.
//!  - Include and use your own Julia code.
//!  - Load a custom system image.
//!  - Create values that Julia can use, and convert them back to Rust, from Rust.
//!  - Access the type information and fields of values and check their properties.
//!  - Create and use n-dimensional arrays.
//!  - Support for mapping Julia structs to Rust structs which can be generated with `JlrsReflect.jl`.
//!  - Structs that can be mapped to Rust include those with type parameters and bits unions.
//!  - Use these features when calling Rust from Julia through `ccall`.
//!  - Offload long-running functions to another thread and `.await` the result with the (experimental) async runtime.
//!
//!
//! # Generating the bindings
//!
//! This crate depends on `jl-sys` which contains the raw bindings to the Julia C API, these are
//! generated by `bindgen`. You can find the requirements for using `bindgen` in [their User Guide].
//!
//! #### Linux
//!
//! The recommended way to install Julia is to download the binaries from the official website,
//! which is distributed in an archive containing a directory called `julia-x.y.z`. This directory
//! contains several other directories, including a `bin` directory containing the `julia`
//! executable.
//!
//! In order to ensure the `julia.h` header file can be found, either `/usr/include/julia/julia.h`
//! must exist, or you have to set the `JULIA_DIR` environment variable to `/path/to/julia-x.y.z`.
//! This environment variable can be used to override the default. Similarly, in order to load
//! `libjulia.so` you must add `/path/to/julia-x.y.z/lib` to the `LD_LIBRARY_PATH` environment
//! variable.
//!
//! #### Windows
//!
//! The recommended way to install Julia is to download the installer from the official website,
//! which will install Julia in a folder called `Julia-x.y.z`. This folder contains several other
//! folders, including a `bin` folder containing the `julia.exe` executable. You must set the
//! `JULIA_DIR` environment variable to the `Julia-x.y.z` folder and add `Julia-x.y.z\bin` to the
//! `PATH` environment variable. For example, if Julia is installed at `D:\Julia-x.y.z`,
//! `JULIA_DIR` must be set to `D:\Julia-x.y.z` and `D:\Julia-x.y.z\bin` must be added to `PATH`.
//!
//! Additionally, MinGW must be installed through Cygwin. To install this and all potentially
//! required dependencies, follow steps 1-4 of
//! [the instructions for compiling Julia on Windows using Cygwin and MinGW].
//! You must set the `CYGWIN_DIR` environment variable to the installation folder of Cygwin; this
//! folder contains some icons, `Cygwin.bat` and folders with names like `usr` and `bin`. For
//! example, if Cygwin is installed at `D:\cygwin64`, `CYGWIN_DIR` must be set to `D:\cygwin64`.
//!
//! Julia is compatible with the GNU toolchain on Windows. If you use rustup, you can set the
//! toolchain for a project that depends on `jl-sys` by calling the command
//! `rustup override set stable-gnu` in the project root folder.
//!
//!
//! # Using this crate
//!
//! The first thing you should do is `use` the [`prelude`]-module with an asterisk, this will
//! bring all the structs and traits you're likely to need into scope. If you're calling Julia
//! from Rust, you must initialize Julia before you can use it. You can do this by calling
//! [`Julia::init`]. Note that this method can only be called once, if you drop [`Julia`] you won't
//! be able to create a new one and have to restart the entire program. If you want to use a
//! custom system image, you must call [`Julia::init_with_image`] instead of [`Julia::init`].
//! If you're calling Rust from Julia everything has already been initialized, you can use `CCall`
//! instead.
//!
//! ## Calling Julia from Rust
//!
//! You can call [`Julia::include`] to include your own Julia code and either [`Julia::frame`] or
//! [`Julia::dynamic_frame`] to interact with Julia.
//!
//! The other two methods, [`Julia::frame`] and [`Julia::dynamic_frame`], take a closure that
//! provides you with a [`Global`], and either a [`StaticFrame`] or [`DynamicFrame`] respectively.
//! [`Global`] is a token that lets you access Julia modules their contents, and other global
//! values, while the frames are used to deal with local Julia data.
//!
//! Local data must be handled properly: Julia is a programming language with a garbage collector
//! that is unaware of any references to data outside of Julia. In order to make it aware of this
//! usage a stack must be maintained. You choose this stack's size when calling [`Julia::init`].
//! The elements of this stack are called stack frames; they contain a pointer to the previous
//! frame, the number of protected values, and that number of pointers to values. The two frame
//! types offered by jlrs take care of all the technical details, a [`DynamicFrame`] will grow
//! to the required size while a [`StaticFrame`] has a definite number of slots. These frames can
//! be nested (ie stacked) arbitrarily.
//!
//! In order to call a Julia function, you'll need two things: a function to call, and arguments
//! to call it with. You can acquire the function through the module that defines it with
//! [`Module::function`]; [`Module::base`] and [`Module::core`] provide access to Julia's `Base`
//! and `Core` module respectively, while everything you include through [`Julia::include`] is
//! made available relative to the `Main` module which you can access by calling [`Module::main`].
//!
//! Julia data is represented by a [`Value`]. Basic data types like numbers, booleans, and strings
//! can be created through [`Value::new`] and several methods exist to create an n-dimensional
//! array. Each value will be protected by a frame, and the two share a lifetime in order to
//! enforce that a value can only be used as long as its protecting frame hasn't been dropped.
//! Julia functions, their arguments and their results are all `Value`s too. All `Value`s can be
//! called as functions, whether this will succeed depends on the value actually being a function.
//! You can copy data from Julia to Rust by calling [`Value::cast`].
//!
//! As a simple example, let's create two values and add them:
//!
//! ```no_run
//! # use jlrs::prelude::*;
//! # fn main() {
//! let mut julia = unsafe { Julia::init().unwrap() };
//! julia.dynamic_frame(|global, frame| {
//!     // Create the two arguments
//!     let i = Value::new(&mut *frame, 2u64)?;
//!     let j = Value::new(&mut *frame, 1u32)?;
//!
//!     // We can find the addition-function in the base module
//!     let func = Module::base(global).function("+")?;
//!
//!     // Call the function and unbox the result
//!     let output = func.call2(&mut *frame, i, j)?.unwrap();
//!     output.cast::<u64>()
//! }).unwrap();
//! # }
//! ```
//!
//! You can also do this with a static frame:
//!
//! ```no_run
//! # use jlrs::prelude::*;
//! # fn main() {
//! let mut julia = unsafe { Julia::init().unwrap() };
//! // Three slots; two for the inputs and one for the output.
//! julia.frame(3, |global, frame| {
//!     // Create the two arguments, each value requires one slot
//!     let i = Value::new(&mut *frame, 2u64)?;
//!     let j = Value::new(&mut *frame, 1u32)?;
//!
//!     // We can find the addition-function in the base module
//!     let func = Module::base(global).function("+")?;
//!
//!     // Call the function and unbox the result.  
//!     let output = func.call2(&mut *frame, i, j)?.unwrap();
//!     output.cast::<u64>()
//! }).unwrap();
//! # }
//! ```
//!
//! This is only a small example, other things can be done with [`Value`] as well: their fields
//! can be accessed if the [`Value`] is some tuple or struct. They can contain more complex data;
//! if a function returns an array or a module it will still be returned as a [`Value`]. There
//! complex types are compatible with [`Value::cast`]. Additionally, you can create [`Output`]s in
//! a frame in order to protect a value from with a specific frame; this value will share that
//! frame's lifetime.
//!
//! ## Standard library and installed packages
//!
//! Julia has a standard library that includes modules like `LinearAlgebra` and `Dates`, and comes
//! with a package manager that makes it easy to install new packages. In order to use these
//! modules and packages, they must first be loaded. This can be done by calling
//! [`Module::require`].
//!
//! ## Calling Rust from Julia
//!
//! Julia's `ccall` interface can be used to call `extern "C"` functions defined in Rust. There
//! are two major ways to use `ccall`, with a pointer to the function or a
//! `(:function, "library")` pair.
//!
//! A function can be cast to a void pointer and converted to a [`Value`]:
//!
//! ```no_run
//! # use jlrs::prelude::*;
//!
//! unsafe extern "C" fn call_me(arg: bool) -> isize {
//!     if arg {
//!         1
//!     } else {
//!         -1
//!     }
//! }
//!
//! # fn main() {
//! let mut julia = unsafe { Julia::init().unwrap() };
//! julia.frame(2, |global, frame| {
//!     // Cast the function to a void pointer
//!     let call_me_val = Value::new(&mut *frame, call_me as *mut std::ffi::c_void)?;
//!
//!     // `myfunc` will call the function pointer, it's defined in the next block of code
//!     let func = Module::main(global).function("myfunc")?;
//!
//!     // Call the function and unbox the result.  
//!     let output = func.call1(&mut *frame, call_me_val)?
//!         .unwrap()
//!         .cast::<isize>()?;
//!
//!     assert_eq!(output, 1);
//!     
//!     Ok(())
//! }).unwrap();
//! # }
//! ```
//!
//! This pointer can be called from Julia:
//!
//! ```julia
//! function myfunc(callme::Ptr{Cvoid})::Int
//!     ccall(callme, Int, (Bool,), true)
//! end
//! ```
//!
//! You can also use functions defined in `dylib` and `cdylib` libraries. In order to create such
//! a library you need to add
//!
//! ```toml
//! [lib]
//! crate-type = ["dylib"]
//! ```
//!
//! or  
//!
//! ```toml
//! [lib]
//! crate-type = ["cdylib"]
//! ```
//!
//! respectively to your crate's `Cargo.toml`. Use a `dylib` if you want to use the crate in other
//! Rust crates, but if it's only intended to be called through `ccall` a `cdylib` is the better
//! choice. On Linux, compiling such a crate will be compiled to `lib<crate_name>.so`, on Windows
//! `lib<crate_name>.dll`.
//!
//! The functions you want to use with `ccall` must be both `extern "C"` functions to ensure the C
//! ABI is used, and annotated with `#[no_mangle]` to prevent name mangling. Julia can find
//! libraries in directories that are either on the default library search path or included by
//! setting the `LD_LIBRARY_PATH` environment variable on Linux, or `PATH` on Windows. If the
//! compiled library is not directly visible to Julia, you can open it with `Libdl.dlopen` and
//! acquire function pointers with `Libdl.dlsym`. These pointers can be called the same way as
//! the pointer in the previous example.
//!
//! If the library is visible to Julia you can access it with the library name. If `call_me` is
//! defined in a crate called `foo`, the following should work:
//!
//! ```julia
//! ccall((:call_me, "libfoo"), Int, (Bool,), false)
//! ```
//!
//! One important aspect of calling Rust from other languages in general is that panicking across
//! an FFI boundary is undefined behaviour. If you're not sure your code will never panic, wrap it
//! with `std::panic::catch_unwind`.
//!
//! Many features provided by jlrs including accessing modules, calling functions, and borrowing
//! array data require a [`Global`] or a frame. You can access these by creating a [`CCall`]
//! first.
//!
//!
//! ## Async runtime
//!
//! The experimental async runtime runs Julia in a separate thread and allows multiple tasks to
//! run in parallel by offloading functions to a new thread in Julia and waiting for them to
//! complete without blocking the runtime. To use this feature you must enable the `async` feature
//! flag:
//!
//! ```toml
//! [dependencies]
//! jlrs = { version = "0.8", features = ["async"] }
//! ```
//!
//! This features is only supported on Linux.
//!
//! The struct [`AsyncJulia`] is exported by the prelude and lets you initialize the runtime in
//! two ways, either as a task or as a thread. The first type should be used if you want to
//! integrate the async runtime into a larger project that uses `async_std`. In order for the
//! runtime to work correctly the `JULIA_NUM_THREADS` environment variable must be set to a value
//! larger than 1.
//!
//! In order to call Julia with the async runtime you must implement the [`JuliaTask`] trait. The
//! `run`-method of this trait is similar to the closures that are used in the examples
//! above for the sync runtime; it provides you with a [`Global`] and an [`DynamicAsyncFrame`] which
//! implements the [`Frame`] trait. The [`DynamicAsyncFrame`] is required to use [`Value::call_async`]
//! which calls a function on a new thread using `Base.Threads.@spawn` and returns a `Future`.
//! While you await the result the runtime can handle another task. If you don't use
//! [`Value::call_async`] tasks are handled sequentially.
//!
//! It's important to keep in mind that allocating memory in Julia uses a lock, so if you run
//! multiple functions at the same time that allocate new values frequently the performance will
//! drop significantly. The garbage collector can only run when all threads have reached a
//! safepoint, which is the case whenever a function needs to allocate memory. If your function
//! takes a long time to complete but needs to allocate rarely, you should periodically call
//! `GC.safepoint` in Julia to ensure the garbage collector can run.
//!
//! You can find fully commented basic examples in [the examples directory of the repo].
//!
//!
//! # Custom types
//!
//! In order to map a struct in Rust to one in Julia you can derive [`JuliaStruct`]. This will
//! implement [`Cast`], [`JuliaType`], [`ValidLayout`], and [`JuliaTypecheck`] for that type. If
//! the struct in Julia has no type parameters and is a bits type you can also derive
//! [`IntoJulia`], which lets you use the type in combination with [`Value::new`].
//!
//! You should not implement these structs manually. The `JlrsReflect.jl` package can generate
//! the correct Rust struct for types that don't include any unions or tuples with type
//! parameters. The reason for this restriction is that the layout of tuple and union fields can
//! be very different depending on these parameters in a way that can't be nicely expressed in
//! Rust.
//!
//! These custom types can also be used when you call Rust from Julia through `ccall`.
//!
//!
//! # Lifetimes
//!
//! While reading the documentation for this crate, you will see that a lot of lifetimes are used.
//! Most of these lifetimes have a specific meaning:
//!
//! - `'base` is the lifetime of a frame created through [`Julia::frame`] or
//! [`Julia::dynamic_frame`]. This lifetime prevents you from using global Julia data outside of a
//! frame.
//!
//! - `'frame` is the lifetime of an arbitrary frame; in the base frame it will be the same as
//! `'base`. This lifetime prevents you from using Julia data after the frame that protects it
//! from garbage collection goes out of scope.
//!
//! - `'data` or `'borrow` is the lifetime of data that is borrowed. This lifetime prevents you
//! from mutably aliasing data and trying to use it after the borrowed data is dropped.
//!
//! - `'output` is the lifetime of the frame that created the output. This lifetime ensures that
//! when Julia data is protected by an older frame this data can be used until that frame goes out
//! of scope.
//!
//! [their User Guide]: https://rust-lang.github.io/rust-bindgen/requirements.html
//! [`prelude`]: prelude/index.html
//! [`Julia`]: struct.Julia.html
//! [`CCall`]: struct.CCall.html
//! [`Julia::init`]: struct.Julia.html#method.init
//! [`Julia::init_with_image`]: struct.Julia.html#method.init_with_image
//! [`Julia::include`]: struct.Julia.html#method.include
//! [`Julia::frame`]: struct.Julia.html#method.frame
//! [`Julia::dynamic_frame`]: struct.Julia.html#method.dynamic_frame
//! [`Global`]: global/struct.Global.html
//! [`Output`]: frame/struct.Output.html
//! [`DynamicAsyncFrame`]: frame/struct.DynamicAsyncFrame.html
//! [`StaticFrame`]: frame/struct.StaticFrame.html
//! [`DynamicFrame`]: frame/struct.DynamicFrame.html
//! [`Frame`]: traits/trait.Frame.html
//! [`JuliaStruct`]: traits/trait.JuliaStruct.html
//! [`Cast`]: traits/trait.Cast.html
//! [`JuliaType`]: traits/trait.JuliaType.html
//! [`JuliaTypecheck`]: traits/trait.JuliaTypecheck.html
//! [`ValidLayout`]: traits/trait.ValidLayout.html
//! [`IntoJulia`]: traits/trait.IntoJulia.html
//! [`Module::function`]: value/module/struct.Module.html#method.function
//! [`Module::base`]: value/module/struct.Module.html#method.base
//! [`Module::core`]: value/module/struct.Module.html#method.core
//! [`Module::main`]: value/module/struct.Module.html#method.main
//! [`JuliaTask`]: traits/multitask/trait.JuliaTask.html
//! [`Value`]: value/struct.Value.html
//! [`Value::new`]: value/struct.Value.html#method.new
//! [`Value::call_async`]: value/struct.Value.html#method.call_async
//! [`Value::cast`]: value/struct.Value.html#method.cast
//! [`AsyncJulia`]: multitask/struct.AsyncJulia.html
//! [the instructions for compiling Julia on Windows using Cygwin and MinGW]: https://github.com/JuliaLang/julia/blob/v1.5.2/doc/build/windows.md#cygwin-to-mingw-cross-compiling
//! [the examples directory of the repo]: https://github.com/Taaitaaiger/jlrs/tree/v0.8/examples

pub mod error;
pub mod frame;
pub mod global;
#[doc(hidden)]
pub mod jl_sys_export;
#[cfg(all(feature = "async", target_os = "linux"))]
pub mod julia_future;
pub mod mode;
#[cfg(all(feature = "async", target_os = "linux"))]
pub mod multitask;
pub mod prelude;
pub mod traits;
#[doc(hidden)]
pub mod util;
pub mod value;

use error::{JlrsError, JlrsResult};
use frame::{DynamicFrame, NullFrame, StaticFrame, PAGE_SIZE};
use global::Global;
use jl_sys::{jl_atexit_hook, jl_init, jl_init_with_image__threading, jl_is_initialized};
use mode::Sync;
use std::ffi::{c_void, CString};
use std::io::{Error as IOError, ErrorKind};
use std::mem::MaybeUninit;
use std::path::Path;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use traits::Call;
use value::array::Array;
use value::module::Module;
use value::Value;

pub(crate) static INIT: AtomicBool = AtomicBool::new(false);

pub(crate) static JLRS_JL: &'static str = include_str!("jlrs.jl");

struct Stack {
    raw: Box<[*mut c_void]>,
}

impl Stack {
    pub(crate) fn new() -> Self {
        let raw = vec![null_mut(); PAGE_SIZE];
        Stack {
            raw: raw.into_boxed_slice(),
        }
    }
}

impl AsMut<[*mut c_void]> for Stack {
    fn as_mut(&mut self) -> &mut [*mut c_void] {
        self.raw.as_mut()
    }
}

/// This struct can be created only once during the lifetime of your program. You must create it
/// with [`Julia::init`] or [`Julia::init_with_image`] before you can do anything related to
/// Julia. While this struct exists, Julia is active; dropping it causes the shutdown code to be
/// called.
///
/// [`Julia::init`]: struct.Julia.html#method.init
/// [`Julia::init_with_image`]: struct.Julia.html#method.init_with_image
pub struct Julia {
    stack: Stack,
}

impl Julia {
    /// Initializes Julia, this function can only be called once. If you call it a second time it
    /// will return an error. If this struct is dropped, you will need to restart your program to
    /// be able to call Julia code again.
    ///
    /// You have to choose a stack size when calling this function. This will be the total number
    /// of slots that will be available for the GC stack. One of these slots will always be in
    /// use. Each frame needs two slots of overhead, plus one for every value created with that
    /// frame. A [`StaticFrame`] preallocates its slots, while a [`DynamicFrame`] grows to the
    /// required size. If calling a method requires one or more slots, this amount is explicitly
    /// documented.
    ///
    /// This function is unsafe because this crate provides you with a way to execute arbitrary
    /// Julia code which can't be checked for correctness.
    ///
    /// [`StaticFrame`]: frame/struct.StaticFrame.html
    /// [`DynamicFrame`]: frame/struct.DynamicFrame.html
    pub unsafe fn init() -> JlrsResult<Self> {
        if jl_is_initialized() != 0 || INIT.swap(true, Ordering::SeqCst) {
            return Err(JlrsError::AlreadyInitialized.into());
        }

        jl_init();
        let mut jl = Julia {
            stack: Stack::new(),
        };

        jl.frame(2, |global, frame| {
            Value::eval_string(frame, JLRS_JL)?.expect("Could not load Jlrs module");

            let droparray_fn = Value::new(frame, droparray as *mut c_void)?;
            Module::main(global)
                .submodule("Jlrs")?
                .global("droparray")?
                .set_nth_field(0, droparray_fn)?;

            Ok(())
        })
        .expect("Could not load Jlrs module");

        Ok(jl)
    }

    /// This function is similar to [`Julia::init`] except that it loads a custom system image. A
    /// custom image can be generated with the [`PackageCompiler`] package for Julia. The main
    /// advantage of using a custom image over the default one is that it allows you to avoid much
    /// of the compilation overhead often associated with Julia.
    ///
    /// Two additional arguments are required to call this function compared to [`Julia::init`];
    /// `julia_bindir` and `image_relative_path`. The first must be the absolute path to a
    /// directory that contains a compatible Julia binary (eg `${JULIA_DIR}/bin`), the second must
    /// be either an absolute or a relative path to a system image.
    ///
    /// This function will return an error if either of the two paths does not exist or if Julia
    /// has already been initialized.
    ///
    /// [`Julia::init`]: struct.Julia.html#init
    /// [`PackageCompiler`]: https://julialang.github.io/PackageCompiler.jl/dev/
    pub unsafe fn init_with_image<P: AsRef<Path>>(
        julia_bindir: P,
        image_path: P,
    ) -> JlrsResult<Self> {
        if INIT.swap(true, Ordering::SeqCst) {
            Err(JlrsError::AlreadyInitialized)?;
        }

        let julia_bindir_str = julia_bindir.as_ref().to_string_lossy().to_string();
        let image_path_str = image_path.as_ref().to_string_lossy().to_string();

        if !julia_bindir.as_ref().exists() {
            let io_err = IOError::new(ErrorKind::NotFound, julia_bindir_str);
            return Err(JlrsError::other(io_err))?;
        }

        if !image_path.as_ref().exists() {
            let io_err = IOError::new(ErrorKind::NotFound, image_path_str);
            return Err(JlrsError::other(io_err))?;
        }

        let bindir = CString::new(julia_bindir_str).unwrap();
        let im_rel_path = CString::new(image_path_str).unwrap();

        jl_init_with_image__threading(bindir.as_ptr(), im_rel_path.as_ptr());

        let mut jl = Julia {
            stack: Stack::new(),
        };

        jl.frame(2, |global, frame| {
            Value::eval_string(frame, JLRS_JL)?.expect("Could not load Jlrs module");

            let droparray_fn = Value::new(frame, droparray as *mut c_void)?;
            Module::main(global)
                .submodule("Jlrs")?
                .global("droparray")?
                .set_nth_field(0, droparray_fn)?;

            Ok(())
        })
        .expect("Could not load Jlrs module");

        Ok(jl)
    }

    /// Calls `include` in the `Main` module in Julia, which executes the file's contents in that
    /// module. This has the same effect as calling `include` in the Julia REPL.
    ///
    /// Example:
    ///
    /// ```no_run
    /// # use jlrs::prelude::*;
    /// # fn main() {
    /// # let mut julia = unsafe { Julia::init().unwrap() };
    /// julia.include("MyJuliaCode.jl").unwrap();
    /// # }
    /// ```
    pub fn include<P: AsRef<Path>>(&mut self, path: P) -> JlrsResult<()> {
        if path.as_ref().exists() {
            return self.frame(3, |global, frame| {
                let path_jl_str = Value::new(&mut *frame, path.as_ref().to_string_lossy())?;
                let include_func = Module::main(global).function("include")?;
                let res = include_func.call1(frame, path_jl_str)?;

                return match res {
                    Ok(_) => Ok(()),
                    Err(e) => Err(JlrsError::IncludeError(
                        path.as_ref().to_string_lossy().into(),
                        e.type_name().into(),
                    )
                    .into()),
                };
            });
        }

        Err(JlrsError::IncludeNotFound(path.as_ref().to_string_lossy().into()).into())
    }

    /// Create a [`StaticFrame`] that can hold `capacity` values, and call the given closure.
    /// Returns the result of this closure, or an error if the new frame can't be created because
    /// there's not enough space on the GC stack. The number of required slots on the stack is
    /// `capacity + 2`.
    ///
    /// Every output and value you create inside the closure using the [`StaticFrame`], either
    /// directly or through calling a [`Value`], will reduce the available capacity of the
    /// [`StaticFrame`] by 1.
    ///
    /// Example:
    ///
    /// ```
    /// # use jlrs::prelude::*;
    /// # use jlrs::util::JULIA;
    /// # fn main() {
    /// # JULIA.with(|j| {
    /// # let mut julia = j.borrow_mut();
    ///   julia.frame(1, |_global, frame| {
    ///       let i = Value::new(frame, 1u64)?;
    ///       Ok(())
    ///   }).unwrap();
    /// # });
    /// # }
    /// ```
    ///
    /// [`StaticFrame`]: ../frame/struct.StaticFrame.html
    /// [`Value`]: ../value/struct.Value.html
    pub fn frame<'base, 'julia: 'base, T, F>(
        &'julia mut self,
        capacity: usize,
        func: F,
    ) -> JlrsResult<T>
    where
        F: FnOnce(Global<'base>, &mut StaticFrame<'base, Sync>) -> JlrsResult<T>,
    {
        unsafe {
            let global = Global::new();
            let mut frame = StaticFrame::new(self.stack.as_mut(), capacity, Sync);
            func(global, &mut frame)
        }
    }

    /// Create a [`DynamicFrame`] and call the given closure. Returns the result of this closure,
    /// or an error if the new frame can't be created because the stack is too small. The number
    /// of required slots on the stack is 2.
    ///
    /// Every output and value you create inside the closure using the [`DynamicFrame`], either
    /// directly or through calling a [`Value`], will occupy a single slot on the GC stack.
    ///
    /// Example:
    ///
    /// ```
    /// # use jlrs::prelude::*;
    /// # use jlrs::util::JULIA;
    /// # fn main() {
    /// # JULIA.with(|j| {
    /// # let mut julia = j.borrow_mut();
    /// julia.dynamic_frame(|_global, frame| {
    ///     let j = Value::new(frame, 1u64)?;
    ///     Ok(())
    /// }).unwrap();
    /// # });
    /// # }
    /// ```
    ///
    /// [`DynamicFrame`]: ../frame/struct.DynamicFrame.html
    /// [`Value`]: ../value/struct.Value.html
    pub fn dynamic_frame<'base, 'julia: 'base, T, F>(&'julia mut self, func: F) -> JlrsResult<T>
    where
        F: FnOnce(Global<'base>, &mut DynamicFrame<'base, Sync>) -> JlrsResult<T>,
    {
        unsafe {
            let global = Global::new();
            let mut frame = DynamicFrame::new(self.stack.as_mut(), Sync);
            func(global, &mut frame)
        }
    }
}

impl Drop for Julia {
    fn drop(&mut self) {
        unsafe {
            jl_atexit_hook(0);
        }
    }
}

/// When you call Rust from Julia through `ccall`, Julia has already been initialized and trying to
/// initialize it again would cause a crash. In order to still be able to call Julia from Rust
/// and to borrow arrays (if you pass them as `Array` rather than `Ptr{Array}`), you'll need to
/// create a frame first. You can use this struct to do so. It must never be used outside
/// functions called through `ccall`.
///
/// If you only need to use a frame to borrow array data, you can use [`CCall::null`] and
/// [`CCall::null_frame`]. Unlike [`Julia`], `CCall` postpones the allocation of the stack that is
/// used for managing the GC until a static or dynamic frame is created. In the case of a null
/// frame, this stack isn't allocated at all. Unlike the other frame types null frames can't be
/// nested.
///
/// [`Julia`]: struct.Julia.html
/// [`CCall::null_frame`]: struct.CCall.html#method.null_frame
/// [`CCall::null`]: struct.CCall.html#method.null
pub struct CCall {
    stack: Option<Stack>,
}

impl CCall {
    /// Create a new `CCall` that provides a stack with `stack_size` slots. This functions the
    /// same way as [`Julia::init`] does. This function must never be called outside a function
    /// called through `ccall` from Julia and must only be called once during that call. The stack
    /// is not allocated untl a static or dynamic frame is created.
    ///
    /// [`Julia::init`]: struct.Julia.html#method.init
    pub unsafe fn new() -> Self {
        CCall { stack: None }
    }

    /// Create a new `CCall` that provides a stack with no slots. This means only creating a null
    /// frame is supported. This function must never be called outside a function
    /// called through `ccall` from Julia and must only be called once during that call. The stack
    /// is not allocated untl a static or dynamic frame is created.
    pub unsafe fn null() -> Self {
        CCall::new()
    }

    /// Create a [`StaticFrame`] that can hold `capacity` values, and call the given closure.
    /// Returns the result of this closure, or an error if the new frame can't be created because
    /// there's not enough space on the GC stack. The number of required slots on the stack is
    /// `capacity + 2`.
    ///
    /// Every output and value you create inside the closure using the [`StaticFrame`], either
    /// directly or through calling a [`Value`], will reduce the available capacity of the
    /// [`StaticFrame`] by 1.
    ///
    /// [`StaticFrame`]: ../frame/struct.StaticFrame.html
    /// [`Value`]: ../value/struct.Value.html
    pub fn frame<'base, 'julia: 'base, T, F>(
        &'julia mut self,
        capacity: usize,
        func: F,
    ) -> JlrsResult<T>
    where
        F: FnOnce(Global<'base>, &mut StaticFrame<'base, Sync>) -> JlrsResult<T>,
    {
        unsafe {
            self.ensure_init_stack()
                .map(|s| {
                    let global = Global::new();
                    let mut frame = StaticFrame::new(s.as_mut(), capacity, Sync);
                    func(global, &mut frame)
                })
                .unwrap_or_else(|| std::hint::unreachable_unchecked()) // The stack is guaranteed to be initialized
        }
    }

    /// Create a [`DynamicFrame`] and call the given closure. Returns the result of this closure,
    /// or an error if the new frame can't be created because the stack is too small. The number
    /// of required slots on the stack is 2.
    ///
    /// Every output and value you create inside the closure using the [`DynamicFrame`], either
    /// directly or through calling a [`Value`], will occupy a single slot on the GC stack.
    ///
    /// [`DynamicFrame`]: ../frame/struct.DynamicFrame.html
    /// [`Value`]: ../value/struct.Value.html
    pub fn dynamic_frame<'base, 'julia: 'base, T, F>(&'julia mut self, func: F) -> JlrsResult<T>
    where
        F: FnOnce(Global<'base>, &mut DynamicFrame<'base, Sync>) -> JlrsResult<T>,
    {
        unsafe {
            self.ensure_init_stack()
                .map(|s| {
                    let global = Global::new();
                    let mut frame = DynamicFrame::new(s.as_mut(), Sync);
                    func(global, &mut frame)
                })
                .unwrap_or_else(|| std::hint::unreachable_unchecked()) // The stack is guaranteed to be initialized
        }
    }

    /// Create a [`NullFrame`] and call the given closure. A [`NullFrame`] cannot be nested and
    /// can only be used to (mutably) borrow array data. Unlike the other frame-creating methods,
    /// no `Global` is provided to the closure.
    ///
    /// [`NullFrame`]: ../frame/struct.NullFrame.html
    /// [`Global`]: ../global/struct.Global.html
    pub fn null_frame<'base, 'julia: 'base, T, F>(&'julia mut self, func: F) -> JlrsResult<T>
    where
        F: FnOnce(&mut NullFrame<'base>) -> JlrsResult<T>,
    {
        unsafe {
            let mut frame = NullFrame::new(self);
            func(&mut frame)
        }
    }

    #[inline(always)]
    fn ensure_init_stack(&mut self) -> Option<&mut Stack> {
        if self.stack.is_none() {
            self.stack = Some(Stack::new());
        }

        self.stack.as_mut()
    }
}

unsafe extern "C" fn droparray(a: Array) {
    // The data of a moved array is allocated by Rust, this function is called by
    // a finalizer in order to ensure it's also freed by Rust.
    let arr_ref = &mut *a.ptr();

    if arr_ref.flags.how() != 2 {
        return;
    }

    let data_ptr = arr_ref.data.cast::<MaybeUninit<u8>>();
    arr_ref.data = null_mut();
    let n_els = arr_ref.elsize as usize * arr_ref.length;
    Vec::from_raw_parts(data_ptr, n_els, n_els);
}
