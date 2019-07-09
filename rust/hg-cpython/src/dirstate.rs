// dirstate.rs
//
// Copyright 2019 Raphaël Gomès <rgomes@octobus.net>
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2 or any later version.

//! Bindings for the `hg::dirstate` module provided by the
//! `hg-core` package.
//!
//! From Python, this will be seen as `mercurial.rustext.dirstate`
mod dirs_multiset;
use crate::dirstate::dirs_multiset::Dirs;
use cpython::{
    PyBytes, PyDict, PyErr, PyModule, PyObject, PyResult, PySequence, Python,
};
use hg::{DirstateEntry, StateMap};
use libc::{c_char, c_int};
#[cfg(feature = "python27")]
use python27_sys::PyCapsule_Import;
#[cfg(feature = "python3")]
use python3_sys::PyCapsule_Import;
use std::ffi::CStr;
use std::mem::transmute;

/// C code uses a custom `dirstate_tuple` type, checks in multiple instances
/// for this type, and raises a Python `Exception` if the check does not pass.
/// Because this type differs only in name from the regular Python tuple, it
/// would be a good idea in the near future to remove it entirely to allow
/// for a pure Python tuple of the same effective structure to be used,
/// rendering this type and the capsule below useless.
type MakeDirstateTupleFn = extern "C" fn(
    state: c_char,
    mode: c_int,
    size: c_int,
    mtime: c_int,
) -> PyObject;

/// This is largely a copy/paste from cindex.rs, pending the merge of a
/// `py_capsule_fn!` macro in the rust-cpython project:
/// https://github.com/dgrunwald/rust-cpython/pull/169
pub fn decapsule_make_dirstate_tuple(
    py: Python,
) -> PyResult<MakeDirstateTupleFn> {
    unsafe {
        let caps_name = CStr::from_bytes_with_nul_unchecked(
            b"mercurial.cext.parsers.make_dirstate_tuple_CAPI\0",
        );
        let from_caps = PyCapsule_Import(caps_name.as_ptr(), 0);
        if from_caps.is_null() {
            return Err(PyErr::fetch(py));
        }
        Ok(transmute(from_caps))
    }
}

pub fn extract_dirstate(py: Python, dmap: &PyDict) -> Result<StateMap, PyErr> {
    dmap.items(py)
        .iter()
        .map(|(filename, stats)| {
            let stats = stats.extract::<PySequence>(py)?;
            let state = stats.get_item(py, 0)?.extract::<PyBytes>(py)?;
            let state = state.data(py)[0] as i8;
            let mode = stats.get_item(py, 1)?.extract(py)?;
            let size = stats.get_item(py, 2)?.extract(py)?;
            let mtime = stats.get_item(py, 3)?.extract(py)?;
            let filename = filename.extract::<PyBytes>(py)?;
            let filename = filename.data(py);
            Ok((
                filename.to_owned(),
                DirstateEntry {
                    state,
                    mode,
                    size,
                    mtime,
                },
            ))
        })
        .collect()
}

/// Create the module, with `__package__` given from parent
pub fn init_module(py: Python, package: &str) -> PyResult<PyModule> {
    let dotted_name = &format!("{}.dirstate", package);
    let m = PyModule::new(py, dotted_name)?;

    m.add(py, "__package__", package)?;
    m.add(py, "__doc__", "Dirstate - Rust implementation")?;

    m.add_class::<Dirs>(py)?;

    let sys = PyModule::import(py, "sys")?;
    let sys_modules: PyDict = sys.get(py, "modules")?.extract(py)?;
    sys_modules.set_item(py, dotted_name, &m)?;

    Ok(m)
}
