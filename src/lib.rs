use pyo3::prelude::*;

#[pyclass]
struct Inner {
    rust: Vec<i32>,
}

#[pyclass]
struct Variant {
    inner: Py<Inner>,
}

#[pyclass]
struct Info {
    variant: Py<Variant>,
}

#[pymethods]
impl Variant {
    #[new]
    fn new(rust: Vec<i32>, py: Python<'_>) -> Self {
        Self {
            inner: Py::new(py, Inner { rust }).unwrap(),
        }
    }

    fn info(&mut self, py: Python<'_>) -> PyResult<Py<Info>> {
        let variant = Py::new(
            py,
            Variant {
                inner: self.inner.clone_ref(py),
            },
        )?;
        let info = Py::new(py, Info { variant }).unwrap();
        Ok(info)
    }
}

#[pymethods]
impl Info {
    fn set_data(&mut self, index: usize, value: i32) -> PyResult<()> {
        Python::with_gil(|py| {
            let mut guard = self.variant.as_ref(py).borrow_mut();

            let mutable = &mut *guard;
            let mut iguard = mutable.inner.as_ref(py).borrow_mut();
            let mutinner = &mut *iguard;
            mutinner.rust[index] = value;
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_data() {
        Python::with_gil(|py| {
            // Create a new Variant
            let rust_vec = vec![1, 2, 3];
            let mut variant = Variant::new(rust_vec, py);

            // Get the Info
            let info = variant.info(py).unwrap();
            let mut minfo = info.as_ref(py).borrow_mut();

            // Set data
            let index = 1;
            let value = 5;
            minfo.set_data(index, value).unwrap();

            // Check the data
            let mut guard = variant.inner.as_ref(py).borrow_mut();
            let inner = &mut *guard;
            assert_eq!(inner.rust[index], value);
        });
    }
}
