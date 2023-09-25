use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

pub struct Data {
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
    outer: Arc<Mutex<Outer>>,
}

#[pymethods]
impl PyData {
    fn set_data(&mut self, i: usize, x: f64, py: Python<'_>) -> PyResult<()> {
        // ???
        match self.outer.lock() {
            Ok(mut outer) => {
                outer.data_mut().x[i] = x;
            }
            Err(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Failed to lock",
                ))
            }
        }
        Ok(())
    }
}

#[pyclass]
struct PyOuter {
    rust: Arc<Mutex<Outer>>,
}

#[pymethods]
impl PyOuter {
    #[new]
    fn new() -> Self {
        Self {
            rust: Arc::new(Mutex::new(Outer {
                _data: Data { x: [0.0; 1000] },
            })),
        }
    }

    #[getter]
    fn data(&mut self, py: Python<'_>) -> PyResult<PyData> {
        Ok(PyData {
            outer: self.rust.clone(),
        })
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
        // create a PyOuter, access .data and set a value
        Python::with_gil(|py| {
            let mut outer = PyOuter::new();
            let mut data = outer.data(py).unwrap();
            data.set_data(0, 1.0, py).unwrap();

            // check that the value was set
            assert_eq!(outer.rust.lock().unwrap().data_mut().x[0], 1.0);
        });
    }
}
