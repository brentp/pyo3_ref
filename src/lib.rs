use pyo3::prelude::*;
use std::sync::Arc;

struct Data {
    x: [f64; 1000],
}

pub struct Outer {
    _data: Data,
}

impl Outer {
    pub fn data_mut(&mut self) -> &mut Data {
        &mut self._data
    }
}

#[pyclass]
struct PyData {
    // Q: what to put here for mutable access to &Data since we can't have lifetime.
    data: Py<Data>,
}

#[pymethods]
impl PyData {
    fn set_data(&mut self, i: usize, x: f64, py: Python<'_>) {
        // ???
        self.data.x[i] = x;
    }
}

#[pyclass]
struct PyOuter {
    rust: Outer,
}

#[pymethods]
impl PyOuter {
    #[new]
    fn new() -> Self {
        Self {
            rust: Outer {
                _data: Data { x: [0.0; 1000] },
            },
        }
    }

    #[getter]
    fn data(&mut self, py: Python<'_>) -> PyResult<Py<PyData>> {
        // Q: what to put here? PyCell?

        Py::new(
            py,
            PyData {
                data: Py::new(py, self.rust.data_mut())?,
            },
        )
    }
}

#[pymodule]
fn rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyOuter>()?;
    m.add_class::<PyData>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_data() {
        Python::with_gil(|py| {});
    }
}
