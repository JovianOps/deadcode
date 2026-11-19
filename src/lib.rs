use pyo3::prelude::*;

#[pymodule]
fn deadcode(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", "1.5.0")?;
    Ok(())
}
