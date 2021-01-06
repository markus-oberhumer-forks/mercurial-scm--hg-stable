use cpython::ObjectProtocol;
use cpython::PyBytes;
use cpython::PyDict;
use cpython::PyList;
use cpython::PyModule;
use cpython::PyObject;
use cpython::PyResult;
use cpython::PyTuple;
use cpython::Python;

use hg::copy_tracing::ChangedFiles;
use hg::copy_tracing::CombineChangesetCopies;
use hg::Revision;

/// Combines copies information contained into revision `revs` to build a copy
/// map.
///
/// See mercurial/copies.py for details
pub fn combine_changeset_copies_wrapper(
    py: Python,
    revs: PyList,
    children_count: PyDict,
    target_rev: Revision,
    rev_info: PyObject,
    multi_thread: bool,
) -> PyResult<PyDict> {
    let children_count = children_count
        .items(py)
        .iter()
        .map(|(k, v)| Ok((k.extract(py)?, v.extract(py)?)))
        .collect::<PyResult<_>>()?;

    /// (Revision number, parent 1, parent 2, copy data for this revision)
    type RevInfo = (Revision, Revision, Revision, Option<PyBytes>);

    let revs_info = revs.iter(py).map(|rev_py| -> PyResult<RevInfo> {
        let rev = rev_py.extract(py)?;
        let tuple: PyTuple =
            rev_info.call(py, (rev_py,), None)?.cast_into(py)?;
        let p1 = tuple.get_item(py, 0).extract(py)?;
        let p2 = tuple.get_item(py, 1).extract(py)?;
        let opt_bytes = tuple.get_item(py, 2).extract(py)?;
        Ok((rev, p1, p2, opt_bytes))
    });

    let path_copies = if !multi_thread {
        let mut combine_changeset_copies =
            CombineChangesetCopies::new(children_count);

        for rev_info in revs_info {
            let (rev, p1, p2, opt_bytes) = rev_info?;
            let files = match &opt_bytes {
                Some(bytes) => ChangedFiles::new(bytes.data(py)),
                // Python None was extracted to Option::None,
                // meaning there was no copy data.
                None => ChangedFiles::new_empty(),
            };

            combine_changeset_copies.add_revision(rev, p1, p2, files)
        }
        combine_changeset_copies.finish(target_rev)
    } else {
        // Use a bounded channel to provide back-pressure:
        // if the child thread is slower to process revisions than this thread
        // is to gather data for them, an unbounded channel would keep
        // growing and eat memory.
        //
        // TODO: tweak the bound?
        let (rev_info_sender, rev_info_receiver) =
            crossbeam_channel::bounded::<RevInfo>(1000);

        // Start a thread that does CPU-heavy processing in parallel with the
        // loop below.
        //
        // If the parent thread panics, `rev_info_sender` will be dropped and
        // “disconnected”. `rev_info_receiver` will be notified of this and
        // exit its own loop.
        let thread = std::thread::spawn(move || {
            let mut combine_changeset_copies =
                CombineChangesetCopies::new(children_count);
            for (rev, p1, p2, opt_bytes) in rev_info_receiver {
                let gil = Python::acquire_gil();
                let py = gil.python();
                let files = match &opt_bytes {
                    Some(raw) => ChangedFiles::new(raw.data(py)),
                    // Python None was extracted to Option::None,
                    // meaning there was no copy data.
                    None => ChangedFiles::new_empty(),
                };
                combine_changeset_copies.add_revision(rev, p1, p2, files)
            }

            combine_changeset_copies.finish(target_rev)
        });

        for rev_info in revs_info {
            let (rev, p1, p2, opt_bytes) = rev_info?;

            // We’d prefer to avoid the child thread calling into Python code,
            // but this avoids a potential deadlock on the GIL if it does:
            py.allow_threads(|| {
                rev_info_sender.send((rev, p1, p2, opt_bytes)).expect(
                    "combine_changeset_copies: channel is disconnected",
                );
            });
        }
        // We’d prefer to avoid the child thread calling into Python code,
        // but this avoids a potential deadlock on the GIL if it does:
        py.allow_threads(|| {
            // Disconnect the channel to signal the child thread to stop:
            // the `for … in rev_info_receiver` loop will end.
            drop(rev_info_sender);

            // Wait for the child thread to stop, and propagate any panic.
            thread.join().unwrap_or_else(|panic_payload| {
                std::panic::resume_unwind(panic_payload)
            })
        })
    };

    let out = PyDict::new(py);
    for (dest, source) in path_copies.into_iter() {
        out.set_item(
            py,
            PyBytes::new(py, &dest.into_vec()),
            PyBytes::new(py, &source.into_vec()),
        )?;
    }
    Ok(out)
}

/// Create the module, with `__package__` given from parent
pub fn init_module(py: Python, package: &str) -> PyResult<PyModule> {
    let dotted_name = &format!("{}.copy_tracing", package);
    let m = PyModule::new(py, dotted_name)?;

    m.add(py, "__package__", package)?;
    m.add(py, "__doc__", "Copy tracing - Rust implementation")?;

    m.add(
        py,
        "combine_changeset_copies",
        py_fn!(
            py,
            combine_changeset_copies_wrapper(
                revs: PyList,
                children: PyDict,
                target_rev: Revision,
                rev_info: PyObject,
                multi_thread: bool
            )
        ),
    )?;

    let sys = PyModule::import(py, "sys")?;
    let sys_modules: PyDict = sys.get(py, "modules")?.extract(py)?;
    sys_modules.set_item(py, dotted_name, &m)?;

    Ok(m)
}
